#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Inspect a Windows beta package without executing Windows code or tests."""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import stat
import struct
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]
SOURCE_DIRECTORIES = ('src', 'tests', 'examples', 'scripts', 'assets', 'vendor',
                      'docs', 'licenses', '.github')
SOURCE_FILES = ('Cargo.toml', 'Cargo.lock', 'LICENSE', 'README.md', 'CHANGELOG.md',
                'SESSION-SPEC.md', 'THIRD_PARTY_NOTICES.md')


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def inspect_pe(data):
    require(len(data) >= 64 and data[:2] == b'MZ', 'Missing DOS executable header')
    pe = struct.unpack_from('<I', data, 60)[0]
    require(pe + 24 <= len(data) and data[pe:pe + 4] == b'PE\0\0', 'Missing PE header')
    machine, sections, _, _, _, optional_size, characteristics = struct.unpack_from('<HHIIIHH', data, pe + 4)
    optional = pe + 24
    require(machine == 0x8664, f'Expected AMD64 machine, got {machine:#x}')
    require(characteristics & 2, 'PE is not an executable image')
    require(optional_size >= 128 and optional + optional_size <= len(data), 'Truncated optional header')
    require(struct.unpack_from('<H', data, optional)[0] == 0x20b, 'Expected PE32+ image')
    subsystem = struct.unpack_from('<H', data, optional + 68)[0]
    require(subsystem == 2, f'Expected Windows GUI subsystem, got {subsystem}')
    table = optional + optional_size
    require(table + sections * 40 <= len(data), 'Truncated PE sections')
    regions = []
    for index in range(sections):
        section = table + index * 40
        _, rva, raw_size, raw = struct.unpack_from('<IIII', data, section + 8)
        require(raw + raw_size <= len(data), 'PE section exceeds file size')
        regions.append((rva, raw_size, raw))

    def offset(rva, length=1):
        for base, size, raw in regions:
            if base <= rva and rva + length <= base + size:
                return raw + rva - base
        raise ValueError(f'Unmapped PE RVA: {rva:#x}')

    directory_count = struct.unpack_from('<I', data, optional + 108)[0]
    require(directory_count >= 2, 'Missing import data directory')
    import_rva, import_size = struct.unpack_from('<II', data, optional + 120)
    require(import_rva and import_size >= 20, 'Missing PE imports')
    imports = []
    for index in range(import_size // 20):
        descriptor = struct.unpack_from('<IIIII', data, offset(import_rva + index * 20, 20))
        if not any(descriptor):
            break
        start = offset(descriptor[3])
        end = data.find(b'\0', start, min(len(data), start + 512))
        require(end != -1, 'Unterminated imported DLL name')
        imports.append(data[start:end].decode('ascii'))
    else:
        raise ValueError('Unterminated PE import table')
    require(imports, 'PE has no imported DLL names')
    require(not any(name.lower().startswith(('vcruntime', 'msvcp', 'concrt')) for name in imports),
            'Portable EXE unexpectedly imports a VC++ redistributable DLL')
    certificate_size = struct.unpack_from('<I', data, optional + 148)[0] if directory_count >= 5 and optional_size >= 152 else 0
    require(not certificate_size, 'Unexpected certificate table in this unsigned beta')
    return {'format': 'PE32+', 'machine': 'AMD64 (0x8664)', 'subsystem': 'Windows GUI',
            'sections': sections, 'imports': sorted(set(imports), key=str.lower),
            'certificate_table_present': bool(certificate_size)}


def verify(artifact_dir, source_commit):
    version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version']
    name = f'Wireframe-{version}-windows-x86_64'
    exe = artifact_dir / (name + '.exe')
    archive = artifact_dir / (name + '.zip')
    payload = exe.read_bytes()
    pe = inspect_pe(payload)
    require(len(payload) > 1024 * 1024, 'Unexpectedly small release EXE')
    identities = {}
    for path in (exe, archive):
        digest = sha256(path.read_bytes())
        require(path.with_suffix(path.suffix + '.sha256').read_text().strip() == f'{digest}  {path.name}',
                f'Checksum mismatch: {path.name}')
        identities[path.name] = {'bytes': path.stat().st_size, 'sha256': digest}

    source_paths = [ROOT / path for path in SOURCE_FILES]
    for directory in SOURCE_DIRECTORIES:
        source_paths.extend(path for path in (ROOT / directory).rglob('*')
                            if path.is_file() and '__pycache__' not in path.parts and path.suffix != '.pyc')
    with zipfile.ZipFile(archive) as package:
        require(package.testzip() is None, 'ZIP CRC failure')
        entries = package.infolist()
        require(len({entry.filename for entry in entries}) == len(entries), 'Duplicate ZIP paths')
        for entry in entries:
            path = PurePosixPath(entry.filename)
            require(not path.is_absolute() and '..' not in path.parts and '\\' not in entry.filename
                    and path.parts[0] == name and ':' not in entry.filename, f'Unsafe ZIP path: {entry.filename}')
            require(not stat.S_ISLNK(entry.external_attr >> 16), f'Unexpected ZIP symlink: {entry.filename}')
        require(package.read(f'{name}/{name}/wireframe.exe') == payload, 'Archived EXE differs from portable EXE')
        expected_source = {f'{name}/source/{path.relative_to(ROOT).as_posix()}' for path in source_paths}
        actual_source = {entry.filename for entry in entries
                         if entry.filename.startswith(f'{name}/source/') and not entry.is_dir()}
        require(actual_source == expected_source, 'Corresponding-source file set differs from checkout')
        for path in source_paths:
            require(not path.is_symlink(), f'Windows source contains symlink: {path}')
            relative = path.relative_to(ROOT).as_posix()
            require(package.read(f'{name}/source/{relative}') == path.read_bytes(), f'Source mismatch: {relative}')
        for directory in ('licenses', 'assets'):
            for path in (ROOT / directory).rglob('*'):
                if path.is_file():
                    relative = path.relative_to(ROOT).as_posix()
                    require(package.read(f'{name}/{relative}') == path.read_bytes(), f'Reader payload mismatch: {relative}')
        require(package.read(f'{name}/LICENSE') == (ROOT / 'LICENSE').read_bytes(), 'Missing product license')
        manifest = json.loads(package.read(f'{name}/licenses/manifest.json'))
        require(manifest, 'Empty dependency notice manifest')
        for dependency in manifest:
            require(dependency['files'], f'No notice files: {dependency["name"]}')
            for path in dependency['files']:
                require(package.read(f'{name}/licenses/{path}'), f'Empty dependency notice: {path}')

    return {'source_commit': source_commit, 'version': version, 'platform': 'Windows x86-64',
            'status': 'UNTESTED ON WINDOWS — static package inspection only',
            'runtime_tests_performed': False, 'application_launched': False,
            'windows_icon_resource_verified': False, 'source_files_matched': len(source_paths),
            'dependency_notices': len(manifest), 'zip_entries': len(entries), 'pe': pe,
            'artifacts': identities}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--artifact-dir', type=Path, default=ROOT / 'dist')
    parser.add_argument('--source-commit', required=True)
    args = parser.parse_args()
    require(len(args.source_commit) == 40 and all(char in '0123456789abcdef' for char in args.source_commit),
            'Expected full lowercase source commit SHA')
    report = verify(args.artifact_dir, args.source_commit)
    (args.artifact_dir / 'windows-beta-verification.json').write_text(json.dumps(report, indent=2) + '\n')
    (args.artifact_dir / 'WINDOWS-BETA-NOTES.txt').write_text(
        'Wireframe beta — Windows x86-64 portable EXE\n\n'
        'UNTESTED ON WINDOWS. The build has not been launched or tested on Windows.\n'
        'Static PE, checksum, corresponding-source and license checks only.\n'
        'Keep the matching source/license ZIP and checksums with the EXE.\n'
        'Download the EXE to a writable folder and open it to start Wireframe.\n'
        'Windows may warn about this unsigned beta. No automatic updater is activated.\n'
        'Windows executable icon-resource integration remains unverified.\n'
        f'Source commit: {args.source_commit}\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()

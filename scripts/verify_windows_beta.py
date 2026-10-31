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
SOURCE_FILES = ('Cargo.toml', 'Cargo.lock', 'build.rs', 'LICENSE', 'README.md', 'CHANGELOG.md',
                'SESSION-SPEC.md', 'THIRD_PARTY_NOTICES.md')


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def ico_images(data):
    require(len(data) >= 6, 'Truncated ICO header')
    reserved, kind, count = struct.unpack_from('<HHH', data)
    require(reserved == 0 and kind == 1 and 0 < count <= 256, 'Invalid ICO header')
    end = 6 + count * 16
    require(end <= len(data), 'Truncated ICO directory')
    images = {}
    intervals = []
    for index in range(count):
        entry = struct.unpack_from('<BBBBHHII', data, 6 + index * 16)
        width, height, colors, reserved, planes, bits, size, offset = entry
        require(reserved == 0 and size > 0 and offset >= end and offset + size <= len(data),
                'Invalid ICO image bounds')
        descriptor = (width, height, colors, reserved, planes, bits, size)
        require(descriptor not in images, 'Duplicate ICO image descriptor')
        require(all(offset + size <= start or offset >= stop for start, stop in intervals),
                'Overlapping ICO images')
        intervals.append((offset, offset + size))
        images[descriptor] = data[offset:offset + size]
    return images


def icon_resources(data, offset, resource_rva, resource_size, icon):
    """Bounded PE type/name/language tree; no Windows loader or APIs."""
    require(resource_rva and resource_size >= 16, 'Missing PE icon resource directory')
    base = offset(resource_rva, resource_size)
    resources = {}
    visited = set()
    entries_seen = 0

    def relative(position, size):
        require(0 <= position and 0 < size and position + size <= resource_size,
                'Resource directory offset exceeds bounds')
        return base + position

    def walk(position, path):
        nonlocal entries_seen
        require(position not in visited, 'Cyclic or shared resource directory')
        require(len(path) < 3, 'Resource directory exceeds type/name/language depth')
        visited.add(position)
        header = relative(position, 16)
        named, numbered = struct.unpack_from('<HH', data, header + 12)
        count = named + numbered
        entries_seen += count
        require(count > 0 and entries_seen <= 4096, 'Invalid or oversized resource directory')
        start = relative(position + 16, count * 8)
        keys = set()
        previous_id = -1
        for index in range(count):
            name, target = struct.unpack_from('<II', data, start + index * 8)
            require(bool(name & 0x80000000) == (index < named), 'Resource named/ID count mismatch')
            if name & 0x80000000:
                string = name & 0x7fffffff
                length = struct.unpack_from('<H', data, relative(string, 2))[0]
                require(length > 0, 'Empty resource name')
                text = relative(string + 2, length * 2)
                key = data[text:text + length * 2].decode('utf-16-le')
            else:
                require(name <= 0xffff, 'Invalid numeric resource ID')
                key = name
            require(key not in keys, 'Duplicate resource directory key')
            keys.add(key)
            if isinstance(key, int):
                require(key > previous_id, 'Resource IDs are not in ascending order')
                previous_id = key
            child = path + (key,)
            pointer = target & 0x7fffffff
            if target & 0x80000000:
                walk(pointer, child)
            else:
                require(len(child) == 3, 'Resource leaf lacks type/name/language')
                leaf = relative(pointer, 16)
                rva, size, _, reserved = struct.unpack_from('<IIII', data, leaf)
                require(reserved == 0 and size > 0 and resource_rva <= rva
                        and rva + size <= resource_rva + resource_size, 'Invalid resource data bounds')
                address = offset(rva, size)
                require(child not in resources, 'Duplicate resource leaf')
                resources[child] = data[address:address + size]
    walk(0, ())

    expected = ico_images(icon)
    groups = {key: value for key, value in resources.items() if key[0] == 14}
    require(groups and all(key[1] == 1 for key in groups), 'Missing or unexpected application icon group')
    used = set()
    for (_, _, language), group in groups.items():
        require(isinstance(language, int) and len(group) >= 6, 'Invalid icon group language/header')
        reserved, kind, count = struct.unpack_from('<HHH', group)
        require(reserved == 0 and kind == 1 and count == len(expected)
                and len(group) == 6 + count * 14, 'Icon group differs from ICO directory')
        matched = set()
        identifiers = set()
        for index in range(count):
            entry = struct.unpack_from('<BBBBHHIH', group, 6 + index * 14)
            descriptor, identifier = entry[:-1], entry[-1]
            require(descriptor in expected and descriptor not in matched, 'Unexpected or duplicate icon frame')
            require(identifier > 0 and identifier not in identifiers, 'Duplicate or invalid icon image ID')
            key = (3, identifier, language)
            require(key in resources, 'Icon group references missing image')
            require(resources[key] == expected[descriptor], 'Embedded icon payload differs from committed ICO')
            identifiers.add(identifier)
            matched.add(descriptor)
            used.add(key)
        require(matched == set(expected), 'Incomplete embedded icon group')
    require(used == {key for key in resources if key[0] == 3}, 'Unreferenced icon image resource')
    return {'verified': True, 'group_id': 1, 'languages': sorted(key[2] for key in groups),
            'ico_sha256': sha256(icon), 'images': [
                {'width': descriptor[0] or 256, 'height': descriptor[1] or 256,
                 'bits': descriptor[5], 'bytes': len(payload), 'sha256': sha256(payload)}
                for descriptor, payload in expected.items()]}


def inspect_pe(data, icon=None):
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
        require(length > 0 and 0 <= rva and rva + length <= 0x100000000, 'Invalid PE RVA bounds')
        matches = [raw + rva - base for base, size, raw in regions
                   if base <= rva and rva + length <= base + size]
        require(len(matches) == 1, f'Unmapped or ambiguous PE RVA: {rva:#x}')
        return matches[0]

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
    icons = None
    if icon is not None:
        require(directory_count >= 3 and optional_size >= 136, 'Missing resource data directory')
        resource_rva, resource_size = struct.unpack_from('<II', data, optional + 128)
        icons = icon_resources(data, offset, resource_rva, resource_size, icon)
    return {'format': 'PE32+', 'machine': 'AMD64 (0x8664)', 'subsystem': 'Windows GUI',
            'sections': sections, 'imports': sorted(set(imports), key=lambda name: (name.lower(), name)),
            'certificate_table_present': bool(certificate_size), 'icon_resource': icons}


def verify(artifact_dir, source_commit):
    version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version']
    name = f'Wireframe-{version}-windows-x86_64'
    exe = artifact_dir / (name + '.exe')
    archive = artifact_dir / (name + '.zip')
    payload = exe.read_bytes()
    pe = inspect_pe(payload, (ROOT / 'assets/app-icon/Wireframe.ico').read_bytes())
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
            'windows_icon_resource_verified': pe['icon_resource']['verified'], 'source_files_matched': len(source_paths),
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
        'Embedded icon resources statically match the committed ICO. Explorer appearance is untested.\n'
        f'Source commit: {args.source_commit}\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()

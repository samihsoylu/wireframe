#!/usr/bin/env python3
"""Verify an already-built private Linux sizing TAR; never build, launch, or overwrite inputs."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import posixpath
import re
import stat
import struct
import subprocess
import sys
import tarfile
from urllib.parse import quote, unquote, urlsplit, urlunsplit

COMMIT = None
NAME = 'Wireframe-0.1.0-beta.4-linux-aarch64'
TOP = ('Cargo.toml', 'Cargo.lock', 'LICENSE', 'README.md', 'CHANGELOG.md',
       'SESSION-SPEC.md', 'THIRD_PARTY_NOTICES.md')
READER_TOP = ('LICENSE', 'README.md', 'CHANGELOG.md', 'SESSION-SPEC.md', 'THIRD_PARTY_NOTICES.md')
SOURCE_DIRS = ('src', 'tests', 'examples', 'scripts', 'assets', 'vendor', 'docs', 'licenses', '.github')
READER_DIRS = ('assets', 'licenses', 'docs', '.github')
LINK = re.compile(r'\]\(([^)\n]+)\)')


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def sha(path):
    with Path(path).open('rb') as file:
        return hashlib.file_digest(file, 'sha256').hexdigest()


def file_item(data, mode):
    return {'kind': 'file', 'mode': oct(mode), 'bytes': len(data), 'sha256': digest(data)}


def safe_name(raw):
    require(isinstance(raw, str) and raw and '\\' not in raw and ':' not in raw,
            f'Unsafe POSIX path: {raw!r}')
    require(all(ord(c) >= 32 and ord(c) != 127 for c in raw), f'Control character: {raw!r}')
    # TAR directories may end in one slash, but no other spelling aliases are allowed.
    name = raw[:-1] if raw.endswith('/') else raw
    parts = name.split('/')
    require(not name.startswith('/') and all(p not in ('', '.', '..') for p in parts),
            f'Noncanonical relative path: {raw!r}')
    require(PurePosixPath(name).as_posix() == name, f'Noncanonical path: {raw!r}')
    return name


def safe_link(name, target, hard=False):
    # Validate even though the current exact frozen package has no permitted links.
    safe_name(target)
    resolved = target if hard else posixpath.normpath(posixpath.join(posixpath.dirname(name), target))
    require(resolved == NAME or resolved.startswith(NAME + '/'), f'Escaping link: {name!r}')
    return resolved


def inspect_members(archive):
    result = {}
    members = []
    expanded = 0
    for member in archive:
        expanded += member.size
        require(len(members) < 10000 and 0 <= member.size <= 128 * 1024**2
                and expanded <= 512 * 1024**2, 'Archive expansion bound')
        members.append(member)
    for member in members:
        name = safe_name(member.name)
        require(name == NAME or name.startswith(NAME + '/'), f'Unexpected archive root: {name}')
        require(name not in result, f'Duplicate TAR path: {name}')
        require(member.mode in ((0o755, 0o775) if member.isdir() else (0o644, 0o755)), f'Unsupported mode: {name} {oct(member.mode)}')
        if member.issym() or member.islnk():
            safe_link(name, member.linkname, hard=member.islnk())
            raise ValueError(f'Links are absent from this frozen packaging contract: {name}')
        require(member.isfile() or member.isdir(), f'Unsupported TAR type: {name}')
        require(member.isdir() or not member.name.endswith('/'), f'File with directory spelling: {name}')
        require(member.sparse is None, f'Sparse files are outside this package contract: {name}')
        require(member.size == 0 if member.isdir() else member.size >= 0, f'Invalid size: {name}')
        if member.isdir():
            require(member.mode in (0o755, 0o775), f'Unsafe directory mode: {name}')
        result[name] = member
    require(NAME in result and result[NAME].isdir(), 'Missing package root directory')
    for name in result:
        for parent in PurePosixPath(name).parents:
            if str(parent) == '.':
                continue
            require(str(parent) in result and result[str(parent)].isdir(), f'Invalid/missing parent: {name}')
    return result


def git_frozen(repo, manifest_path):
    manifest = json.loads(manifest_path.read_text())
    require(bool(manifest), 'Frozen Git input manifest must not be empty')
    rows = subprocess.check_output(['git', '-C', str(repo), 'ls-tree', '-rz', '--full-tree', COMMIT])
    entries = {}
    for row in rows.split(b'\0'):
        if row:
            header, raw = row.split(b'\t', 1)
            mode, kind, oid = header.split()
            name = safe_name(raw.decode('utf-8'))
            require(kind == b'blob' and mode in (b'100644', b'100755'), f'Unexpected frozen type: {name}')
            entries[name] = (int(mode, 8) & 0o777, oid)
    require(set(entries) == set(manifest), 'Manifest paths differ from frozen Git tree')
    batch = subprocess.run(['git', '-C', str(repo), 'cat-file', '--batch'],
                           input=b''.join(oid + b'\n' for _, oid in entries.values()),
                           stdout=subprocess.PIPE, check=True).stdout
    stream = io.BytesIO(batch)
    contents, modes = {}, {}
    for name, (mode, oid) in entries.items():
        actual_oid, kind, size = stream.readline().rstrip(b'\n').split()
        require(actual_oid == oid and kind == b'blob', f'Git object mismatch: {name}')
        data = stream.read(int(size))
        require(stream.read(1) == b'\n' and len(data) == int(size), 'Truncated Git batch')
        entry = manifest[name]
        require(isinstance(entry, dict) and set(entry) == {'mode', 'bytes', 'sha256'},
                f'Frozen source manifest schema differs: {name}')
        require(entry == {'mode': oct(mode), 'bytes': len(data), 'sha256': digest(data)},
                f'Frozen manifest bytes/mode/hash differ from Git: {name}')
        contents[name], modes[name] = data, mode
    require(not stream.read(), 'Unexpected Git batch tail')
    return contents, modes


def verify_frozen_source(source, contents, modes, directory_modes):
    for name, data in contents.items():
        path = source / name
        require(path.is_file() and not path.is_symlink(), f'Missing/linked frozen input: {name}')
        require(sha(path) == digest(data), f'Frozen input bytes differ: {name}')
        require(stat.S_IMODE(path.stat().st_mode) == modes[name], f'Frozen input Git mode differs: {name}')
    # Packaging copytree copies all nonignored files: prohibit untracked payloads there.
    actual = set()
    actual_directories = {}
    for directory in SOURCE_DIRS:
        root = source / directory
        require(root.is_dir() and not root.is_symlink(), f'Unsafe frozen directory: {directory}')
        for path in [root, *root.rglob('*')]:
            relative = path.relative_to(source).as_posix()
            require(not path.is_symlink(), f'Unexpected frozen symlink: {relative}')
            require(path.is_file() or path.is_dir(), f'Unexpected frozen special file: {relative}')
            if path.is_dir():
                if '__pycache__' not in path.parts:
                    actual_directories[relative] = oct(stat.S_IMODE(path.stat().st_mode))
            if path.is_file() and '__pycache__' not in path.parts and path.suffix != '.pyc':
                actual.add(relative)
    wanted = {p for p in contents if p.split('/')[0] in SOURCE_DIRS}
    require(actual == wanted, f'Untracked/missing frozen package inputs: {sorted(actual ^ wanted)[:20]}')
    require(actual_directories == directory_modes, 'Frozen source directory set/modes differ from declared manifest')
    expected_directories = set()
    for name in wanted:
        expected_directories.update(str(p) for p in PurePosixPath(name).parents if str(p) != '.')
    require(set(directory_modes) == expected_directories, 'Directory manifest differs from frozen source hierarchy')
    require(all(int(mode, 8) in (0o755, 0o775) for mode in directory_modes.values()), 'Only 0755/0775 preserved directory modes are supported')


def expected_package(contents, modes, binary_data, directory_modes=None):
    directory_modes = directory_modes or {}
    files = {}
    source_files = set(contents) - {'.gitignore', '.gitattributes'}
    require(set(TOP).issubset(source_files), 'Missing required corresponding-source top files')
    for name in source_files:
        files['source/' + name] = (contents[name], 0o644 if name in TOP else modes[name])
    for name in READER_TOP:
        files[name] = (contents[name], 0o644)
    for name in contents:
        if name.split('/')[0] in READER_DIRS:
            files[name] = (contents[name], modes[name])
    files[NAME + '/wireframe'] = (binary_data, 0o755)
    paths = set(files) | {'.'}
    for name in files:
        paths.update(str(p) for p in PurePosixPath(name).parents)
    documents = sorted(p for p in files if p in READER_TOP[1:] or (p.startswith('docs/') and p.endswith('.md')))
    require(bool(documents), 'Missing reader Markdown documents')
    links, rewritten = [], []
    for document in documents:
        original = files[document][0].decode('utf-8').replace('\r\n', '\n').replace('\r', '\n')
        def relocate(match):
            raw = match[1]
            angled = raw.startswith('<') and raw.endswith('>')
            url = urlsplit(raw[1:-1] if angled else raw)
            if url.scheme or url.netloc or not url.path or url.path.startswith('/'):
                return match[0]
            destination = posixpath.normpath(posixpath.join(posixpath.dirname(document), unquote(url.path)))
            if destination in paths:
                return match[0]
            if destination == '..' or destination.startswith('../'):
                return match[0]  # Match authoritative rewrite_documentation_links behavior.
            alternate = 'source/' + destination
            if alternate not in paths:
                return match[0]
            local = posixpath.relpath(alternate, posixpath.dirname(document) or '.')
            target = urlunsplit(('', '', quote(local, safe='/@._-'), url.query, url.fragment))
            return '](' + ('<' + target + '>' if angled else target) + ')'
        revised = LINK.sub(relocate, original)
        if revised != original:
            files[document] = (revised.encode('utf-8'), files[document][1])
            rewritten.append(document)
        for raw in LINK.findall(revised):
            url = urlsplit(raw[1:-1] if raw.startswith('<') and raw.endswith('>') else raw)
            if url.scheme or url.netloc or not url.path or url.path.startswith('/'):
                continue
            target = posixpath.normpath(posixpath.join(posixpath.dirname(document), unquote(url.path)))
            require(target in paths and target != '..' and not target.startswith('../'),
                    f'Unresolved/escaping expected local reader link: {document}: {raw}')
            links.append({'document': document, 'target': raw, 'resolved': target})
    wanted = {NAME: {'kind': 'directory', 'mode': '0o755'}}
    for path in paths - {'.'}:
        if path not in files:
            original = path.removeprefix('source/') if path.startswith('source/') else path
            preserved = path.startswith('source/') or path.split('/')[0] in READER_DIRS
            mode = directory_modes.get(original, '0o755') if preserved else '0o755'
            wanted[NAME + '/' + path] = {'kind': 'directory', 'mode': mode}
    for path, (data, mode) in files.items():
        wanted[NAME + '/' + path] = file_item(data, mode)
    return wanted, links, rewritten


def elf_identity(data):
    require(len(data) >= 64 and data[:7] == b'\x7fELF\x02\x01\x01', 'Not ELF64 little-endian v1')
    e_type, machine, version = struct.unpack_from('<HHI', data, 16)
    require(machine == 183 and e_type in (2, 3) and version == 1, 'Not an AArch64 executable ELF')
    phoff = struct.unpack_from('<Q', data, 32)[0]
    ehsize, phsize, phnum = struct.unpack_from('<HHH', data, 52)
    require(ehsize == 64 and phsize == 56 and phnum > 0 and phoff + phsize * phnum <= len(data),
            'Invalid ELF program headers')
    interpreter = None
    for index in range(phnum):
        header = struct.unpack_from('<IIQQQQQQ', data, phoff + index * phsize)
        kind, _, offset, _, _, size, _, _ = header
        require(offset + size <= len(data), 'ELF segment exceeds binary')
        if kind == 3:
            require(interpreter is None, 'Multiple ELF interpreters')
            value = data[offset:offset + size]
            require(value.endswith(b'\0'), 'Invalid ELF interpreter string')
            interpreter = value[:-1].decode('ascii')
    require(interpreter == '/lib/ld-linux-aarch64.so.1', f'Unexpected Linux interpreter: {interpreter}')
    return {'architecture': 'AArch64 ELF64 little-endian', 'elf_type': e_type, 'interpreter': interpreter}


def extracted_tree(root):
    result = {}
    for path in sorted(root.rglob('*')):
        name = path.relative_to(root).as_posix()
        mode = stat.S_IMODE(path.lstat().st_mode)
        require(not path.is_symlink(), f'Unexpected extracted link: {name}')
        if path.is_dir():
            result[name] = {'kind': 'directory', 'mode': oct(mode)}
        elif path.is_file():
            result[name] = {'kind': 'file', 'mode': oct(mode), 'bytes': path.stat().st_size, 'sha256': sha(path)}
        else:
            raise ValueError(f'Unexpected extracted type: {name}')
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('repo', 'frozen-source', 'source-manifest', 'frozen-filesystem-modes', 'archive', 'checksum',
                 'validated-binary', 'build-record', 'frozen-directory-modes', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--source-commit', required=True)
    args = parser.parse_args()
    global COMMIT
    require(re.fullmatch(r'[0-9a-f]{40}', args.source_commit) is not None, 'Exact product SHA required')
    COMMIT = args.source_commit
    require(sys.version_info >= (3, 11), 'Python 3.11+ required')
    inputs = [args.source_manifest, args.frozen_filesystem_modes, args.archive, args.checksum,
              args.validated_binary, args.build_record, args.frozen_directory_modes]
    before = {str(p.resolve()): sha(p) for p in inputs}
    require(not args.output.exists() and not args.output.is_symlink(), 'Output must be a fresh nonexistent directory')
    output = args.output.resolve()
    for path in inputs + [args.repo, args.frozen_source]:
        require(not path.resolve().is_relative_to(output) and not output.is_relative_to(path.resolve()),
                f'Output overlaps an input: {path}')
    contents, modes = git_frozen(args.repo, args.source_manifest)
    directory_modes = json.loads(args.frozen_directory_modes.read_text())
    verify_frozen_source(args.frozen_source, contents, modes, directory_modes)
    filesystem_modes = json.loads(args.frozen_filesystem_modes.read_text())
    require(set(filesystem_modes) == set(contents), 'Frozen filesystem mode manifest path set differs')
    differences = [{'path': p, 'git_linux_mode': oct(modes[p]), 'frozen_filesystem_mode': filesystem_modes[p]}
                   for p in sorted(modes) if modes[p] != int(filesystem_modes[p], 8)]
    build = json.loads(args.build_record.read_text())
    require(build['source_commit'] == COMMIT and build['profile'] == 'release' and build['build_exit_code'] == 0,
            'Build record must identify successful exact-source optimized release build')
    require(args.validated_binary.stat().st_size <= 128 * 1024**2, 'ELF input byte bound')
    require(args.archive.stat().st_size <= 256 * 1024**2, 'Compressed archive byte bound')
    binary_data = args.validated_binary.read_bytes()
    require(digest(binary_data) == build['binary_sha256'], 'Validated standalone ELF differs from build record')
    require(stat.S_IMODE(args.validated_binary.stat().st_mode) == 0o755 and not args.validated_binary.is_symlink(),
            'Validated standalone ELF must be a regular file mode 0755')
    elf = elf_identity(binary_data)
    require(args.archive.name == NAME + '.tar.gz', 'Unexpected archive filename')
    require(args.checksum.read_text() == f'{sha(args.archive)}  {args.archive.name}\n', 'Checksum sidecar differs')
    wanted, links, rewritten = expected_package(contents, modes, binary_data, directory_modes)
    with tarfile.open(args.archive, 'r:gz') as archive:
        members = inspect_members(archive)
        require(set(members) == set(wanted), f'TAR path set mismatch: {sorted(set(members) ^ set(wanted))[:20]}')
        archive_tree = {}
        for name, member in members.items():
            item = {'kind': 'directory', 'mode': oct(member.mode)}
            if member.isfile():
                require(member.size == wanted[name]['bytes'], f'Archive size differs: {name}')
                with archive.extractfile(member) as file:
                    item = {'kind': 'file', 'mode': oct(member.mode), 'bytes': member.size,
                            'sha256': hashlib.file_digest(file, 'sha256').hexdigest()}
            require(item == wanted[name], f'Archive bytes/type/mode differ: {name}')
            archive_tree[name] = item
        args.output.mkdir(parents=True, exist_ok=False)
        extracted = args.output / 'extracted'
        extracted.mkdir()
        previous_umask = os.umask(0o022)
        try:
            # Exact members already match the frozen contract and reject all links,
            # aliases and special files. Materialize exclusively under fresh output;
            # no tarfile extraction API or version-dependent fallback is used.
            for name, member in sorted(members.items(), key=lambda row: (row[0].count('/'), row[0])):
                destination = extracted / name
                if member.isdir():
                    destination.mkdir(mode=0o755)
                else:
                    with archive.extractfile(member) as incoming, destination.open('xb') as outgoing:
                        while chunk := incoming.read(1024 * 1024):
                            outgoing.write(chunk)
                    destination.chmod(member.mode)
        finally:
            os.umask(previous_umask)
    actual = extracted_tree(extracted)
    expected_extracted = {name: ({'kind': 'directory', 'mode': '0o755'} if item['kind'] == 'directory' else item)
                          for name, item in wanted.items()}
    require(actual == expected_extracted, 'Extracted full bytes/type/mode tree differs from filtered archive policy')
    payload = extracted / NAME
    binary = payload / NAME / 'wireframe'
    require(sha(binary) == sha(args.validated_binary) == build['binary_sha256'], 'Extracted/standalone/build ELF differs')
    notices = json.loads((payload / 'licenses/manifest.json').read_text())
    require(notices == json.loads(contents['licenses/manifest.json']), 'Dependency notices differ from frozen source')
    notice_files = []
    for dependency in notices:
        require(bool(dependency['files']), f'Empty notice record: {dependency["name"]}')
        for raw in dependency['files']:
            relative = safe_name(raw)
            path = payload / 'licenses' / relative
            require(path.is_file() and not path.is_symlink() and path.stat().st_size > 0,
                    f'Missing/empty notice file: {raw}')
            notice_files.append({'dependency': dependency['name'], 'path': relative, 'sha256': sha(path)})
    verify_frozen_source(args.frozen_source, contents, modes, directory_modes)
    require(before == {str(p.resolve()): sha(p) for p in inputs}, 'An input changed during verification')
    reader_count = sum(p.split('/')[0] in ('assets', 'licenses') for p in contents)
    require(reader_count > 0, 'Missing frozen reader asset/license files')
    records = {'archive-tree.json': archive_tree, 'extracted-tree.json': actual, 'expected-tree.json': wanted,
               'expected-extracted-tree.json': expected_extracted, 'reader-links.json': links, 'notice-files.json': notice_files}
    report = {'source_commit': COMMIT, 'package_name': NAME, 'archive_sha256': sha(args.archive),
              'archive_bytes': args.archive.stat().st_size, 'archive_entries': len(actual),
              'extracted_entry_tree_sha256': digest(json.dumps(actual, sort_keys=True, separators=(',', ':')).encode()),
              'archive_entry_tree_sha256': digest(json.dumps(archive_tree, sort_keys=True, separators=(',', ':')).encode()),
              'archive_directory_modes': directory_modes,
              'filtered_directory_modes': '0755 from exclusive validated-member materialization and process umask022',
              'frozen_inputs_exact_git_bytes_modes': len(contents), 'corresponding_source_files_exact': len(set(contents) - {'.gitignore', '.gitattributes'}),
              'reader_assets_licenses_exact': reader_count, 'reader_markdown_exact': sum(n in READER_TOP[1:] or (n.startswith('docs/') and n.endswith('.md')) for n in contents),
              'reader_markdown_changed_by_relocation': rewritten, 'local_reader_links_valid': len(links),
              'dependency_notice_records': len(notices), 'notice_file_references_valid': len(notice_files),
              'full_reader_docs_github_set_exact': True, 'mode_policy': 'Linux frozen Git modes; copyfile top-level files 0644; copytree retains Git file modes and declared frozen filesystem directory modes; new directories 0755, umask 022',
              'frozen_filesystem_mode_differences_from_linux_git': differences,
              'binary_sha256': sha(binary), 'binary_mode': '0o755', **elf,
              'inputs_unchanged': before, 'verifier_sha256': sha(Path(__file__)),
              'native_execution': False, 'acceptance': 'Parent review required; static archive/source/ELF verification only',
              'limits': ['Optimized build provenance is supplied by build record; ELF bytes alone do not prove optimization',
                         'No product execution, dynamic dependency availability/ABI test, UI, display-server, OS or native acceptance',
                         'No packaging, publication, VM/container control or updater/security changes']}
    for name, value in {**records, 'verification.json': report}.items():
        (args.output / name).write_text(json.dumps(value, indent=2, sort_keys=True) + '\n')
    print(json.dumps(report, indent=2, sort_keys=True))


if __name__ == '__main__':
    main()

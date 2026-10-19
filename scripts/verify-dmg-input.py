#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Refuse to pair an app with a different package's corresponding source."""
import hashlib
import os
from pathlib import Path, PurePosixPath
import plistlib
import stat
import sys
import zipfile


def digest(stream):
    result = hashlib.sha256()
    for chunk in iter(lambda: stream.read(1024 * 1024), b''):
        result.update(chunk)
    return result.digest()


def disk_entries(app):
    """Enumerate actual entries without traversing directory aliases."""
    entries = {}
    for directory, dirs, files in os.walk(app, followlinks=False):
        for name in dirs + files:
            path = Path(directory) / name
            relative = path.relative_to(app).as_posix()
            mode = path.lstat().st_mode
            if stat.S_ISLNK(mode):
                if os.path.isabs(os.readlink(path)):
                    raise ValueError(f'App link must be relative: {relative}')
                try:
                    target = path.resolve(strict=True)
                except (OSError, RuntimeError) as error:
                    raise ValueError(f'Broken app link: {relative}') from error
                if not target.is_relative_to(app.resolve()):
                    raise ValueError(f'App link leaves the bundle: {relative}')
            elif not (stat.S_ISREG(mode) or stat.S_ISDIR(mode)):
                raise ValueError(f'Unsupported app entry: {relative}')
            entries[relative] = (path, mode)
    return entries


def archive_entries(package):
    entries = {}
    for item in package.infolist():
        name = item.filename.rstrip('/')
        if (not name or item.filename.startswith('/') or '\\' in name
                or any(part in ('', '.', '..') for part in name.split('/'))):
            raise ValueError(f'Invalid archive path: {item.filename}')
        if name in entries:
            raise ValueError(f'Duplicate archive entry: {name}')
        entries[name] = item
    return entries


def verify(app, archive):
    app = Path(app)
    with zipfile.ZipFile(archive) as package:
        bad = package.testzip()
        if bad:
            raise ValueError(f'Corrupt archive member: {bad}')
        entries = archive_entries(package)
        # ditto stores Finder/resource metadata alongside the payload root.
        roots = {name.split('/')[0] for name in entries if not name.startswith('__MACOSX/') and name != '__MACOSX'}
        if len(roots) != 1:
            raise ValueError('Expected one package root')
        root = roots.pop() + '/'
        bundled = root + 'Wireframe.app/'
        members = {name[len(bundled):]: item for name, item in entries.items()
                   if name.startswith(bundled)}
        # Legacy framework-free ZIPs omitted explicit directory entries.
        # Infer those directories, but never infer their modes or follow links.
        for name in list(members):
            for parent in PurePosixPath(name).parents:
                if str(parent) != '.':
                    members.setdefault(str(parent), None)
        local = disk_entries(app)
        if members.keys() != local.keys():
            raise ValueError('App contents differ from the source package')
        for relative, (path, mode) in sorted(local.items()):
            item = members[relative]
            if item is None:
                if not stat.S_ISDIR(mode):
                    raise ValueError(f'App entry type differs: {relative}')
                continue
            packed_mode = item.external_attr >> 16
            if item.create_system != 3 or stat.S_IFMT(mode) != stat.S_IFMT(packed_mode):
                raise ValueError(f'App entry type differs: {relative}')
            if stat.S_IMODE(mode) != stat.S_IMODE(packed_mode):
                raise ValueError(f'App permissions differ: {relative}')
            if stat.S_ISLNK(mode):
                if package.read(item) != os.fsencode(os.readlink(path)):
                    raise ValueError(f'App link differs: {relative}')
            elif stat.S_ISREG(mode):
                with path.open('rb') as disk, package.open(item) as packed:
                    if digest(disk) != digest(packed):
                        raise ValueError(f'App file differs: {relative}')
        info = plistlib.loads((app / 'Contents/Info.plist').read_bytes())
        for required in ('source/Cargo.toml', 'source/Cargo.lock', 'source/src/main.rs',
                         'source/scripts/package.py', 'source/LICENSE',
                         'source/THIRD_PARTY_NOTICES.md', 'licenses/manifest.json'):
            item = entries.get(root + required)
            if item is None or stat.S_ISLNK(item.external_attr >> 16) or item.is_dir():
                raise ValueError(f'Missing corresponding-source file: {required}')
        license_bytes = (Path(__file__).resolve().parents[1] / 'LICENSE').read_bytes()
        if package.read(root + 'source/LICENSE') != license_bytes:
            raise ValueError('Corresponding-source license differs')
        print(f'Verified app and companion source package: {info["CFBundleShortVersionString"]}')


if __name__ == '__main__':
    if len(sys.argv) != 3:
        raise SystemExit('Usage: verify-dmg-input.py APP PACKAGE.zip')
    verify(*sys.argv[1:])

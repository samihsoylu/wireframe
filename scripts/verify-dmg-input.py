#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Refuse to pair an app with a different package's corresponding source."""
import hashlib
from pathlib import Path
import plistlib
import sys
import zipfile


def verify(app, archive):
    app = Path(app)
    with zipfile.ZipFile(archive) as package:
        bad = package.testzip()
        if bad:
            raise ValueError(f'Corrupt archive member: {bad}')
        roots = {name.split('/')[0] for name in package.namelist()}
        if len(roots) != 1:
            raise ValueError('Expected one package root')
        root = roots.pop() + '/'
        bundled = root + 'Wireframe.app/'
        info = plistlib.loads((app / 'Contents/Info.plist').read_bytes())
        archived_info = plistlib.loads(package.read(bundled + 'Contents/Info.plist'))
        if info != archived_info:
            raise ValueError('App metadata differs from the source package')
        members = {name[len(bundled):] for name in package.namelist()
                   if name.startswith(bundled) and not name.endswith('/')}
        local_files = {str(path.relative_to(app)) for path in app.rglob('*') if path.is_file()}
        if members != local_files:
            raise ValueError('App contents differ from the source package')
        for relative in sorted(members):
            disk = hashlib.sha256((app / relative).read_bytes()).digest()
            packed = hashlib.sha256(package.read(bundled + relative)).digest()
            if disk != packed:
                raise ValueError(f'App file differs: {relative}')
        for required in ('source/Cargo.toml', 'source/Cargo.lock', 'source/src/main.rs',
                         'source/scripts/package.py', 'source/LICENSE',
                         'source/THIRD_PARTY_NOTICES.md', 'licenses/manifest.json'):
            if root + required not in package.namelist():
                raise ValueError(f'Missing corresponding-source file: {required}')
        license_bytes = (Path(__file__).resolve().parents[1] / 'LICENSE').read_bytes()
        if package.read(root + 'source/LICENSE') != license_bytes:
            raise ValueError('Corresponding-source license differs')
        print(f'Verified app and companion source package: {info["CFBundleShortVersionString"]}')


if __name__ == '__main__':
    if len(sys.argv) != 3:
        raise SystemExit('Usage: verify-dmg-input.py APP PACKAGE.zip')
    verify(*sys.argv[1:])

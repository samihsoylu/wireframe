#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Build and package the native app, with source and dependency notices."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import plistlib
import re
import shutil
import subprocess
import tarfile
import tempfile
import zipfile
from urllib.parse import quote, unquote, urlsplit, urlunsplit
import sparkle

ROOT = Path(__file__).resolve().parents[1]

def run(*args):
    env = os.environ.copy()
    if platform.system() == 'Darwin':
        # Match the active compiler and SDK even when the caller exported another SDK.
        env['SDKROOT'] = subprocess.check_output(['xcrun','--sdk','macosx','--show-sdk-path'], text=True).strip()
    subprocess.run(args, cwd=ROOT, check=True, env=env)

def copy_payload(payload, destination):
    # Framework aliases must stay links, including links to directories.
    shutil.copytree(payload, destination, symlinks=True)

def rewrite_documentation_links(staging):
    """Keep reader copies usable when code lives in corresponding source."""
    staging = staging.resolve()
    documents = [staging / name for name in ('README.md', 'SESSION-SPEC.md', 'CHANGELOG.md')]
    documents.extend((staging / 'docs').rglob('*.md'))
    for document in documents:
        if not document.is_file():
            continue
        def relocate(match):
            target = match[1]
            angled = target.startswith('<') and target.endswith('>')
            url = urlsplit(target[1:-1] if angled else target)
            if url.scheme or url.netloc or not url.path or url.path.startswith('/'):
                return match[0]
            destination = (document.parent / unquote(url.path)).resolve()
            if destination.exists():
                return match[0]
            try:
                relative = destination.relative_to(staging)
            except ValueError:
                return match[0]
            source = staging / 'source' / relative
            if not source.exists():
                return match[0]
            path = Path(os.path.relpath(source, document.parent)).as_posix()
            target = urlunsplit(('', '', quote(path, safe='/@._-'), url.query, url.fragment))
            return '](' + ('<' + target + '>' if angled else target) + ')'
        text = document.read_text()
        revised = re.sub(r'\]\(([^)\n]+)\)', relocate, text)
        if revised != text:
            document.write_text(revised)

def write_archive(staging, archive, system, name):
    # A refused or interrupted archive must not replace a prior good artifact.
    with tempfile.TemporaryDirectory(prefix='.archive-', dir=archive.parent) as temporary:
        candidate = Path(temporary) / archive.name
        _write_archive(staging, candidate, system, name)
        os.replace(candidate, archive)

def _write_archive(staging, archive, system, name):
    if system == 'darwin':
        if staging.name != name:
            raise ValueError('macOS archive staging must use the package name')
        # Use the native archiver for framework links, helper permissions and
        # resource metadata. Python's ZipFile.write follows file symlinks.
        run('ditto', '-c', '-k', '--sequesterRsrc', '--keepParent', str(staging), str(archive))
    elif system == 'linux':
        with tarfile.open(archive, 'w:gz') as tar:
            tar.add(staging, arcname=name)
    else:
        # Cargo sources can carry timestamps outside ZIP's date range.
        with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED, strict_timestamps=False) as zip_file:
            for path in sorted(staging.rglob('*')):
                if path.is_symlink():
                    raise ValueError(f'Windows packages cannot contain symlinks: {path}')
                if path.is_file(): zip_file.write(path, Path(name) / path.relative_to(staging))

def mac_bundle(binary, out, version, update_info=None):
    # Stage completely before replacing a prior generated bundle. A download,
    # copy or signing failure leaves the prior bundle usable.
    destination = out / 'Wireframe.app'
    with tempfile.TemporaryDirectory(prefix='.bundle-', dir=out) as temporary:
        bundle = Path(temporary) / 'Wireframe.app'
        executable = bundle / 'Contents/MacOS/wireframe'
        executable.parent.mkdir(parents=True)
        shutil.copy2(binary, executable)
        info = {'CFBundleName': 'Wireframe', 'CFBundleDisplayName':'Wireframe',
            'CFBundleIdentifier':sparkle.BUNDLE_ID, 'CFBundleExecutable':'wireframe',
            'CFBundlePackageType':'APPL', 'CFBundleShortVersionString':version,
            'CFBundleVersion':version, 'NSHighResolutionCapable':True,
            'CFBundleIconFile':'Wireframe.icns',
            'LSMinimumSystemVersion':'12.0'}
        if update_info: info.update(update_info)
        (bundle / 'Contents/Info.plist').write_bytes(plistlib.dumps(info))
        resources = bundle / 'Contents/Resources'
        resources.mkdir()
        shutil.copyfile(ROOT / 'assets/app-icon/Wireframe.icns', resources / 'Wireframe.icns')
        shutil.copyfile(ROOT / 'LICENSE', resources / 'LICENSE')
        shutil.copytree(ROOT / 'assets', resources / 'assets')
        if update_info: sparkle.embed(bundle, ROOT / '.tooling' / 'sparkle')
        # Local ad-hoc signing is not Developer ID signing or notarization.
        run('codesign', '--force', '--deep', '--sign', '-', str(bundle))
        run('codesign', '--verify', '--deep', '--strict', str(bundle))
        previous = Path(temporary) / 'previous.app'
        had_previous = destination.exists() or destination.is_symlink()
        if had_previous: destination.rename(previous)
        try:
            bundle.rename(destination)
        except OSError:
            if had_previous: previous.rename(destination)
            raise
    return destination

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--debug', action='store_true', help='Build a development app')
    parser.add_argument('--no-archive', action='store_true', help='Refresh only the local bundle')
    parser.add_argument('--updater-config', type=Path,
        help='macOS only: JSON containing public feed_url, public_key and integer build string')
    parser.add_argument('--target', choices=('x86_64-pc-windows-gnu', 'x86_64-pc-windows-msvc', 'aarch64-pc-windows-msvc'),
        help='Build a Windows package with an installed target toolchain')
    args = parser.parse_args()
    if args.updater_config and (args.target or platform.system() != 'Darwin'):
        parser.error('--updater-config requires a native macOS build')
    update_info = sparkle.configuration(args.updater_config) if args.updater_config else None
    if os.environ.get('CARGO_BUILD_TARGET') and not args.target:
        parser.error('Use --target explicitly instead of CARGO_BUILD_TARGET when packaging')
    profile = 'debug' if args.debug else 'release'
    build = ['cargo', 'build', '--locked']
    if args.target: build.extend(['--target', args.target])
    if not args.debug: build.append('--release')
    run(*build)
    metadata = json.loads(subprocess.check_output(['cargo','metadata','--no-deps','--format-version','1'], cwd=ROOT))
    version = metadata['packages'][0]['version']
    system = platform.system().lower()
    machine = platform.machine().lower()
    if args.target:
        system = 'windows'
        machine = args.target.split('-')[0]
    out = ROOT / 'dist'
    out.mkdir(exist_ok=True)
    name = f'Wireframe-{version}-{system}-{machine}'
    if args.debug: name += '-debug'
    if system == 'darwin' and not args.no_archive and (out / (name + '.dmg')).exists():
        raise SystemExit(f'Refusing to replace an existing installer: {out / (name + ".dmg")}')
    target_dir = Path(metadata['target_directory'])
    if args.target: target_dir /= args.target
    binary = target_dir / profile / ('wireframe.exe' if system == 'windows' else 'wireframe')
    if system == 'darwin':
        payload = mac_bundle(binary, out, version, update_info)
    else:
        payload = out / name
        payload.mkdir(exist_ok=True)
        shutil.copy2(binary, payload / binary.name)
    if args.no_archive:
        print(payload)
        return
    if not (ROOT / 'licenses' / 'manifest.json').exists():
        raise SystemExit('Generate notices first: python3 scripts/generate_notices.py')
    staging_root = out / (name + '-package')
    if staging_root.exists(): shutil.rmtree(staging_root)
    staging = staging_root / name
    staging.mkdir(parents=True)
    if payload.is_dir():
        copy_payload(payload, staging / payload.name)
    for path in ('LICENSE','README.md','CHANGELOG.md','SESSION-SPEC.md','THIRD_PARTY_NOTICES.md'):
        shutil.copyfile(ROOT / path, staging / path)
    shutil.copytree(ROOT / 'licenses', staging / 'licenses', dirs_exist_ok=True)
    shutil.copytree(ROOT / 'assets', staging / 'assets', dirs_exist_ok=True)
    shutil.copytree(ROOT / 'docs', staging / 'docs', dirs_exist_ok=True)
    shutil.copytree(ROOT / '.github', staging / '.github', dirs_exist_ok=True)
    # Include this product's corresponding source and build scripts with every package.
    source = staging / 'source'
    source.mkdir(exist_ok=True)
    for path in ('src','tests','examples','scripts','assets','vendor','docs','licenses','.github'):
        shutil.copytree(ROOT / path, source / path, dirs_exist_ok=True, symlinks=True,
            ignore=shutil.ignore_patterns('__pycache__','*.pyc'))
    for path in ('Cargo.toml','Cargo.lock','LICENSE','README.md','CHANGELOG.md','SESSION-SPEC.md','THIRD_PARTY_NOTICES.md'):
        shutil.copyfile(ROOT / path, source / path)
    rewrite_documentation_links(staging)
    archive = out / (name + ('.tar.gz' if system == 'linux' else '.zip'))
    write_archive(staging, archive, system, name)
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    archive.with_suffix(archive.suffix + '.sha256').write_text(f'{digest}  {archive.name}\n')
    print(json.dumps({'archive':str(archive),'sha256':digest,'profile':profile}))
    if system == 'darwin':
        run('sh', 'scripts/package-dmg.sh', str(payload), str(out / (name + '.dmg')), str(archive))
    elif system == 'windows':
        # A portable EXE, accompanied by the source/license ZIP above.
        portable = out / (name + '.exe')
        shutil.copy2(binary, portable)
        digest = hashlib.sha256(portable.read_bytes()).hexdigest()
        portable.with_suffix('.exe.sha256').write_text(f'{digest}  {portable.name}\n')

if __name__ == '__main__': main()

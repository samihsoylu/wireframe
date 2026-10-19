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
import shutil
import subprocess
import tarfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]

def run(*args):
    env = os.environ.copy()
    if platform.system() == 'Darwin':
        # Match the active compiler and SDK even when the caller exported another SDK.
        env['SDKROOT'] = subprocess.check_output(['xcrun','--sdk','macosx','--show-sdk-path'], text=True).strip()
    subprocess.run(args, cwd=ROOT, check=True, env=env)

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--debug', action='store_true', help='Build a development app')
    parser.add_argument('--no-archive', action='store_true', help='Refresh only the local bundle')
    parser.add_argument('--target', choices=('x86_64-pc-windows-gnu', 'x86_64-pc-windows-msvc', 'aarch64-pc-windows-msvc'),
        help='Build a Windows package with an installed target toolchain')
    args = parser.parse_args()
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
    target_dir = Path(metadata['target_directory'])
    if args.target: target_dir /= args.target
    binary = target_dir / profile / ('wireframe.exe' if system == 'windows' else 'wireframe')
    if system == 'darwin':
        bundle = out / 'Wireframe.app'
        executable = bundle / 'Contents' / 'MacOS' / 'wireframe'
        executable.parent.mkdir(parents=True, exist_ok=True)
        temporary = executable.with_suffix('.new')
        shutil.copy2(binary, temporary)
        os.replace(temporary, executable)
        info = {'CFBundleName': 'Wireframe', 'CFBundleDisplayName':'Wireframe',
            'CFBundleIdentifier':'local.wireframe.desktop', 'CFBundleExecutable':'wireframe',
            'CFBundlePackageType':'APPL', 'CFBundleShortVersionString':version,
            'CFBundleVersion':version, 'NSHighResolutionCapable':True,
            'LSMinimumSystemVersion':'12.0'}
        (bundle / 'Contents' / 'Info.plist').write_bytes(plistlib.dumps(info))
        resources = bundle / 'Contents' / 'Resources'
        resources.mkdir(exist_ok=True)
        shutil.copyfile(ROOT / 'LICENSE', resources / 'LICENSE')
        shutil.copytree(ROOT / 'assets', resources / 'assets', dirs_exist_ok=True)
        # A local ad-hoc signature; this is not Developer ID signing or notarization.
        run('codesign', '--force', '--deep', '--sign', '-', str(bundle))
        payload = bundle
    else:
        payload = out / name
        payload.mkdir(exist_ok=True)
        shutil.copy2(binary, payload / binary.name)
    if args.no_archive:
        print(payload)
        return
    if not (ROOT / 'licenses' / 'manifest.json').exists():
        raise SystemExit('Generate notices first: python3 scripts/generate_notices.py')
    staging = out / (name + '-package')
    if staging.exists(): shutil.rmtree(staging)
    staging.mkdir()
    if payload.is_dir():
        shutil.copytree(payload, staging / payload.name, dirs_exist_ok=True)
    for path in ('LICENSE','README.md','SESSION-SPEC.md','THIRD_PARTY_NOTICES.md'):
        shutil.copyfile(ROOT / path, staging / path)
    shutil.copytree(ROOT / 'licenses', staging / 'licenses', dirs_exist_ok=True)
    shutil.copytree(ROOT / 'assets', staging / 'assets', dirs_exist_ok=True)
    shutil.copytree(ROOT / 'docs', staging / 'docs', dirs_exist_ok=True)
    # Include this product's corresponding source and build scripts with every package.
    source = staging / 'source'
    source.mkdir(exist_ok=True)
    for path in ('src','tests','examples','scripts','assets','vendor','docs','licenses'):
        shutil.copytree(ROOT / path, source / path, dirs_exist_ok=True,
            ignore=shutil.ignore_patterns('__pycache__','*.pyc'))
    for path in ('Cargo.toml','Cargo.lock','LICENSE','README.md','SESSION-SPEC.md','THIRD_PARTY_NOTICES.md'):
        shutil.copyfile(ROOT / path, source / path)
    if system == 'linux':
        archive = out / (name + '.tar.gz')
        with tarfile.open(archive, 'w:gz') as tar:
            tar.add(staging, arcname=name)
    else:
        archive = out / (name + '.zip')
        # Cargo source archives can carry epoch timestamps, outside ZIP's date range.
        with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED, strict_timestamps=False) as zip_file:
            for path in sorted(staging.rglob('*')):
                if path.is_file(): zip_file.write(path, Path(name) / path.relative_to(staging))
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    archive.with_suffix(archive.suffix + '.sha256').write_text(f'{digest}  {archive.name}\n')
    print(json.dumps({'archive':str(archive),'sha256':digest,'profile':profile}))

if __name__ == '__main__': main()

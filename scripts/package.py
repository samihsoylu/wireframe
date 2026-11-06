#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Build and package the native app, with source and dependency notices."""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import platform
import plistlib
import re
import shutil
import stat
import subprocess
import tarfile
import tempfile
import unicodedata
import zipfile
from urllib.parse import quote, unquote, urlsplit, urlunsplit
import sparkle

ROOT = Path(__file__).resolve().parents[1]
SOURCE_MANIFEST = 'PACKAGE-SOURCE.json'
PRIVATE_NOTES = {'SESSION.md', 'SESSION-SPEC.md', 'TEAM-EXECUTION-PLAN.md',
                 'TEST-STRATEGY-AUDIT.md', 'TEST-STRATEGY-INVENTORY.json'}
READER_ROOTS = ('LICENSE', 'README.md', 'CHANGELOG.md', 'THIRD_PARTY_NOTICES.md',
                'licenses', 'assets', 'docs', '.github')

def destination_key(name):
    return unicodedata.normalize('NFC', name).casefold()


def private_path(name):
    return destination_key(PurePosixPath(name).name) in {destination_key(note) for note in PRIVATE_NOTES}


def approved_path(name):
    if not isinstance(name, str):
        return False
    path = PurePosixPath(name)
    return (bool(name) and not path.is_absolute()
            and '\\' not in name and ':' not in name
            and all(ord(character) >= 32 and ord(character) != 127 for character in name)
            and all(part not in ('', '.', '..') and destination_key(part) != '.git'
                    and not part.endswith((' ', '.')) for part in name.split('/'))
            and not private_path(name) and destination_key(name) != destination_key(SOURCE_MANIFEST))


def validate_inventory_paths(files):
    destinations = {}
    for name in files:
        if not approved_path(name):
            raise ValueError(f'Invalid source inventory path: {name}')
        parts = name.split('/')
        for length in range(1, len(parts) + 1):
            prefix = '/'.join(parts[:length])
            identity = (prefix, length == len(parts))
            previous = destinations.setdefault(destination_key(prefix), identity)
            if previous != identity:
                raise ValueError(f'Conflicting source inventory destination: {name}')


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f'Duplicate source inventory key: {key}')
        result[key] = value
    return result


def source_entry(name):
    path = ROOT / name
    # A tracked path must never redirect a recursive copy through a local link.
    if any(parent.is_symlink() for parent in path.parents if parent != ROOT and ROOT in parent.parents):
        raise ValueError(f'Source parent is a symlink: {name}')
    mode = path.lstat().st_mode
    if stat.S_ISLNK(mode):
        target = os.readlink(path)
        if os.path.isabs(target) or not path.resolve(strict=True).is_relative_to(ROOT.resolve()):
            raise ValueError(f'Source link leaves the source tree: {name}')
        data = os.fsencode(target)
        return {'mode': stat.S_IMODE(mode), 'sha256': hashlib.sha256(data).hexdigest(), 'link': target}
    if not stat.S_ISREG(mode):
        raise ValueError(f'Unsupported source entry: {name}')
    return {'mode': stat.S_IMODE(mode), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}


def matches_entry(name, entry):
    if not isinstance(entry, dict) or type(entry.get('mode')) is not int:
        return False
    actual = source_entry(name)
    allowed = {0o777} if 'link' in actual else {0o644, 0o755}
    # Inventory permissions are canonical Git permissions, independent of the
    # host's executable-bit support (notably Windows/core.filemode=false).
    actual['mode'] = entry['mode']
    return entry['mode'] in allowed and actual == entry


def approved_inventory(require_clean=True):
    """Use a reviewed Git tree, or the inventory shipped with Gitless source.

    Untracked/ignored files are never inputs. Archives carry this public, hashed
    inventory so repackaging needs neither local Git nor operational notes.
    """
    if (ROOT / '.git').exists():
        def git(*args):
            return subprocess.check_output(['git', '-C', str(ROOT), *args])
        if require_clean:
            result = subprocess.run(['git', '-C', str(ROOT), 'diff', '--no-ext-diff',
                                     '--no-textconv', '--quiet', 'HEAD', '--'], check=False)
            if result.returncode:
                raise ValueError('Packaging requires a clean frozen tracked source tree')
        files = {}
        for row in git('ls-tree', '-rz', '--full-tree', 'HEAD').split(b'\0'):
            if not row:
                continue
            header, raw = row.split(b'\t', 1)
            mode, kind, _ = header.split()
            name = raw.decode('utf-8')
            if private_path(name):
                continue
            if not approved_path(name) or kind != b'blob' or mode not in (b'100644', b'100755', b'120000'):
                raise ValueError(f'Unsupported tracked source: {name}')
            files[name] = source_entry(name)
            if ('link' in files[name]) != (mode == b'120000'):
                raise ValueError(f'Source type differs from Git: {name}')
            files[name]['mode'] = 0o777 if mode == b'120000' else int(mode, 8) & 0o777
    else:
        manifest = ROOT / SOURCE_MANIFEST
        if not manifest.is_file() or manifest.is_symlink():
            raise ValueError('No approved source inventory; use a frozen Git checkout or shipped source manifest')
        data = json.loads(manifest.read_text(), object_pairs_hook=unique_object)
        if (not isinstance(data, dict) or set(data) != {'format', 'files'}
                or type(data['format']) is not int or data['format'] != 1 or not isinstance(data['files'], dict)):
            raise ValueError('Invalid source inventory')
        files = data['files']
    validate_inventory_paths(files)
    for name, entry in files.items():
        if not matches_entry(name, entry):
            raise ValueError(f'Source inventory mismatch: {name}')
    required = ('Cargo.toml', 'Cargo.lock', 'build.rs', 'LICENSE', 'README.md',
                'CHANGELOG.md', 'THIRD_PARTY_NOTICES.md', 'licenses/manifest.json',
                'scripts/package.py', 'src/main.rs')
    if any(name not in files for name in required):
        raise ValueError('Approved source inventory lacks required build or legal files')
    for name, entry in files.items():
        if 'link' in entry:
            target = Path(os.path.normpath(ROOT / name / '..' / entry['link'])).relative_to(ROOT).as_posix()
            if target not in files and not any(path.startswith(target + '/') for path in files):
                raise ValueError(f'Source link target is not approved: {name}')
    # A tracked manifest must not point to an omitted licence, even in Gitless source.
    notices = json.loads((ROOT / 'licenses/manifest.json').read_text())
    if not isinstance(notices, list) or not notices:
        raise ValueError('Approved source inventory lacks dependency notices')
    for notice in notices:
        if not notice['files'] or any('licenses/' + name not in files for name in notice['files']):
            raise ValueError('Approved source inventory lacks dependency licence files')
    return files


def copy_approved_files(destination, inventory, roots=None):
    """The same allowlist governs source, readers and bundled assets."""
    validate_inventory_paths(inventory)
    for name, entry in inventory.items():
        if roots is not None and not any(name == root or name.startswith(root + '/') for root in roots):
            continue
        if not matches_entry(name, entry):
            raise ValueError(f'Source changed after inventory: {name}')
        target = destination / name
        target.parent.mkdir(parents=True, exist_ok=True)
        for parent in PurePosixPath(name).parents:
            if str(parent) != '.':
                (destination / str(parent)).chmod(stat.S_IMODE((ROOT / str(parent)).stat().st_mode))
        shutil.copy2(ROOT / name, target, follow_symlinks=False)
        if 'link' not in entry:
            target.chmod(entry['mode'])


def copy_corresponding_source(destination, inventory=None):
    inventory = approved_inventory() if inventory is None else inventory
    destination.mkdir()
    copy_approved_files(destination, inventory)
    (destination / SOURCE_MANIFEST).write_bytes(source_manifest_bytes(inventory))


def source_manifest_bytes(inventory):
    return (json.dumps({'format': 1, 'files': inventory}, indent=2) + '\n').encode('utf-8')


def copy_readers(destination, inventory):
    copy_approved_files(destination, inventory, READER_ROOTS)

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
    documents = [staging / name for name in ('README.md', 'CHANGELOG.md', 'THIRD_PARTY_NOTICES.md')]
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
        original = document.read_text()
        # Removed operational records are not distributed reader destinations.
        def public_link(match):
            url = urlsplit(match[2].strip('<>'))
            return match[1] if not (url.scheme or url.netloc) and private_path(unquote(url.path)) else match[0]
        text = re.sub(r'\[([^]\n]+)\]\(([^)\n]+)\)', public_link, original)
        revised = re.sub(r'\]\(([^)\n]+)\)', relocate, text)
        if revised != original:
            document.write_text(revised)

def installer_notices(text, archive_name):
    """Keep installer notices readable when their files live inside the companion ZIP."""
    def describe_location(match):
        label, target = match[1], match[2]
        url = urlsplit(target[1:-1] if target.startswith('<') and target.endswith('>') else target)
        if url.scheme or url.netloc or not url.path or url.path.startswith('/'):
            return match[0]
        return f'{label} (`{target}` inside the companion package)'
    title, _, body = text.partition('\n')
    introduction = ('Full source and license files are in the '
        f'[companion ZIP]({quote(archive_name, safe="._-")}). '
        'Copy it to a writable folder and extract it. Open THIRD_PARTY_NOTICES.md '
        'inside that package for working file links. Paths below refer to that package.')
    body = re.sub(r'\[([^]\n]+)\]\(([^)\n]+)\)', describe_location, body)
    return title + '\n\n' + introduction + '\n' + body

def write_archive(staging, archive, system, name, inventory=None):
    # A refused or interrupted archive must not replace a prior good artifact.
    with tempfile.TemporaryDirectory(prefix='.archive-', dir=archive.parent) as temporary:
        candidate = Path(temporary) / archive.name
        _write_archive(staging, candidate, system, name, inventory)
        os.replace(candidate, archive)

def _write_archive(staging, archive, system, name, inventory=None):
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
        if inventory is not None:
            validate_inventory_paths(inventory)
        # Cargo sources can carry timestamps outside ZIP's date range.
        with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED, strict_timestamps=False) as zip_file:
            for path in sorted(staging.rglob('*')):
                if path.is_symlink():
                    raise ValueError(f'Windows packages cannot contain symlinks: {path}')
                if path.is_file():
                    relative = path.relative_to(staging).as_posix()
                    source_name = relative.removeprefix('source/')
                    canonical = (inventory or {}).get(source_name if relative.startswith('source/') else relative)
                    mode = canonical['mode'] if canonical else stat.S_IMODE(path.stat().st_mode)
                    if relative == 'source/' + SOURCE_MANIFEST:
                        mode = 0o644
                    info = zipfile.ZipInfo.from_file(path, str(PurePosixPath(name) / relative), strict_timestamps=False)
                    info.create_system = 3
                    info.external_attr = (stat.S_IFREG | mode) << 16
                    info.compress_type = zipfile.ZIP_DEFLATED
                    with path.open('rb') as incoming, zip_file.open(info, 'w') as outgoing:
                        shutil.copyfileobj(incoming, outgoing)

def mac_version_fields(version, build_number=None):
    # Apple bundle versions are numeric; the archive/Cargo version keeps beta labels.
    short = version.split('-', 1)[0].split('+', 1)[0]
    if not re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+', short):
        raise ValueError('macOS release version requires three numeric components')
    build = str(build_number) if build_number is not None else short
    if build_number is not None and not re.fullmatch(r'[1-9][0-9]{0,17}', build):
        raise ValueError('macOS build number must be a positive integer')
    return short, build


def diagnostic_bundle_info(run_id, root):
    """Prepare only fresh owned marker state; never use normal application roots."""
    if not re.fullmatch(r'[A-Za-z0-9-]{1,40}', run_id):
        raise ValueError('Invalid diagnostic run ID')
    root = Path(root)
    if not root.is_absolute() or root.parent.resolve(strict=True) != root.parent:
        raise ValueError('Diagnostic root must have a canonical absolute parent')
    if root.exists() or root.is_symlink():
        raise ValueError('Diagnostic root must be fresh')
    root.mkdir(mode=0o700)
    root.chmod(0o700)
    identity = f'{sparkle.BUNDLE_ID}.diagnostic.{run_id}'
    marker = {'version': 1, 'run_id': run_id, 'bundle_id': identity, 'root': str(root)}
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, 'O_NOFOLLOW', 0)
    fd = os.open(root / 'diagnostic.json', flags, 0o600)
    with os.fdopen(fd, 'w') as stream:
        json.dump(marker, stream)
    return {'CFBundleIdentifier': identity,
            'CFBundleName': f'Wireframe Diagnostic {run_id}',
            'CFBundleDisplayName': f'Wireframe Diagnostic {run_id}',
            'WireframeDiagnostic': True, 'WireframeDiagnosticRunID': run_id,
            'WireframeDiagnosticRoot': str(root), 'WireframeEnableUpdater': False}

def mac_bundle(binary, out, version, update_info=None, build_number=None, diagnostic=None, inventory=None):
    # Stage completely before replacing a prior generated bundle. A download,
    # copy or signing failure leaves the prior bundle usable.
    if diagnostic and update_info is not None:
        raise ValueError('Diagnostic bundles cannot contain an updater configuration or payload')
    inventory = approved_inventory() if inventory is None else inventory
    diagnostic_info = diagnostic_bundle_info(*diagnostic) if diagnostic else None
    bundle_name = diagnostic_info['CFBundleDisplayName'] + '.app' if diagnostic_info else 'Wireframe.app'
    destination = out / bundle_name
    if diagnostic and (destination.exists() or destination.is_symlink()):
        raise ValueError('Diagnostic bundle destination must be fresh')
    with tempfile.TemporaryDirectory(prefix='.bundle-', dir=out) as temporary:
        bundle = Path(temporary) / bundle_name
        executable = bundle / 'Contents/MacOS/wireframe'
        executable.parent.mkdir(parents=True)
        shutil.copy2(binary, executable)
        short_version, build_version = mac_version_fields(version, build_number)
        info = {'CFBundleName': 'Wireframe', 'CFBundleDisplayName':'Wireframe',
            'CFBundleIdentifier':sparkle.BUNDLE_ID, 'CFBundleExecutable':'wireframe',
            'CFBundlePackageType':'APPL', 'CFBundleShortVersionString':short_version,
            'CFBundleVersion':build_version, 'NSHighResolutionCapable':True,
            'CFBundleIconFile':'Wireframe.icns',
            'LSMinimumSystemVersion':'12.0'}
        if update_info: info.update(update_info)
        if diagnostic_info: info.update(diagnostic_info)
        (bundle / 'Contents/Info.plist').write_bytes(plistlib.dumps(info))
        resources = bundle / 'Contents/Resources'
        resources.mkdir()
        copy_approved_files(resources, inventory, ('assets', 'LICENSE'))
        shutil.copyfile(resources / 'assets/app-icon/Wireframe.icns', resources / 'Wireframe.icns')
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
    parser.add_argument('--build-number', type=int,
        help='Positive macOS bundle build number, including beta iterations')
    parser.add_argument('--diagnostic-run-id', help='Private keyboard-only diagnostic identity (requires --debug --no-archive)')
    parser.add_argument('--diagnostic-root', type=Path, help='Fresh owned marker/trace directory')
    parser.add_argument('--diagnostic-output', type=Path, help='Explicit private bundle output, never dist')
    args = parser.parse_args()
    diagnostic_args = (args.diagnostic_run_id, args.diagnostic_root, args.diagnostic_output)
    diagnostic = None
    if any(value is not None for value in diagnostic_args):
        if not all(value is not None for value in diagnostic_args):
            parser.error('All three diagnostic options are required')
        if not args.debug or not args.no_archive or args.target or args.updater_config or platform.system() != 'Darwin':
            parser.error('Diagnostic packaging requires native macOS --debug --no-archive and no updater/target')
        if args.diagnostic_output.resolve().is_relative_to((ROOT / 'dist').resolve()):
            parser.error('Diagnostic output must be private, not dist')
        diagnostic = (args.diagnostic_run_id, args.diagnostic_root)

    if args.updater_config and (args.target or platform.system() != 'Darwin'):
        parser.error('--updater-config requires a native macOS build')
    update_info = sparkle.configuration(args.updater_config) if args.updater_config else None
    if args.build_number is not None:
        if args.build_number < 1 or args.target or platform.system() != 'Darwin':
            parser.error('--build-number requires a positive integer and a native macOS build')
        if update_info and update_info['CFBundleVersion'] != str(args.build_number):
            parser.error('--build-number must match the updater configuration build')
    if os.environ.get('CARGO_BUILD_TARGET') and not args.target:
        parser.error('Use --target explicitly instead of CARGO_BUILD_TARGET when packaging')
    inventory = approved_inventory(require_clean=not args.no_archive)
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
    out = args.diagnostic_output if diagnostic else ROOT / 'dist'
    out.mkdir(exist_ok=True)
    name = f'Wireframe-{version}-{system}-{machine}'
    if args.debug: name += '-debug'
    if system == 'darwin' and not args.no_archive and (out / (name + '.dmg')).exists():
        raise SystemExit(f'Refusing to replace an existing installer: {out / (name + ".dmg")}')
    target_dir = Path(metadata['target_directory'])
    if args.target: target_dir /= args.target
    binary = target_dir / profile / ('wireframe.exe' if system == 'windows' else 'wireframe')
    if system == 'darwin':
        payload = mac_bundle(binary, out, version, update_info, args.build_number, diagnostic=diagnostic, inventory=inventory)
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
    copy_readers(staging, inventory)
    # Include this product's corresponding source and build scripts with every package.
    source = staging / 'source'
    copy_corresponding_source(source, inventory)
    rewrite_documentation_links(staging)
    archive = out / (name + ('.tar.gz' if system == 'linux' else '.zip'))
    write_archive(staging, archive, system, name, inventory)
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

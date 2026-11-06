#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Distribution integrity checks; no build, signing key or installed app needed."""
import copy
import importlib.util
import json
from pathlib import Path
import os
import platform
import plistlib
import shutil
import stat
import subprocess
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import zipfile

import package


class MacBetaVersionTests(unittest.TestCase):
    def test_beta_label_stays_out_of_numeric_bundle_versions(self):
        self.assertEqual(package.mac_version_fields('0.1.0-beta.1', 1), ('0.1.0', '1'))
        self.assertEqual(package.mac_version_fields('0.1.0-beta.2', 2), ('0.1.0', '2'))
        self.assertEqual(package.mac_version_fields('0.1.0'), ('0.1.0', '0.1.0'))

    def test_invalid_bundle_versions_are_rejected(self):
        for version, build in [('beta', 1), ('0.1', 1), ('0.1.0', 0), ('0.1.0', -1)]:
            with self.subTest(version=version, build=build), self.assertRaises(ValueError):
                package.mac_version_fields(version, build)

spec = importlib.util.spec_from_file_location('verify_dmg_input', Path(__file__).with_name('verify-dmg-input.py'))
verifier = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verifier)


def write_file(path, data, mode=0o644):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)
    path.chmod(mode)


class SourceInventoryTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.base = Path(self.temporary.name)
        self.root = self.base / 'checkout'
        self.root.mkdir()
        files = {
            'Cargo.toml': b'[package]\nname="wireframe"\nversion="0.1.0-beta.6"\n',
            'Cargo.lock': b'fixture lock\n', 'build.rs': b'fn main() {}\n',
            'LICENSE': b'product licence\n', 'README.md': b'[patches](vendor/README.md)\n',
            'CHANGELOG.md': b'changes\n',
            'THIRD_PARTY_NOTICES.md': b'[licence](licenses/item/LICENSE)\n',
            'licenses/manifest.json': b'[{"name":"item","files":["item/LICENSE"]}]',
            'licenses/item/LICENSE': b'dependency licence\n',
            'src/main.rs': b'fn main() {}\n', 'scripts/package.py': b'# build input\n',
            'docs/guide.md': b'[source](../src/main.rs)\n[current plan](../sEsSion%2DsPeC.md)\n[public history](https://example.test/SESSION-SPEC.md)\n',
            'vendor/README.md': b'vendor patch notes\n',
            'vendor/patched.rs': b'local tracked vendor changes\n',
            'assets/app-icon/Wireframe.icns': b'icon fixture',
            '.gitignore': b'SESSION*\nTEAM-EXECUTION-PLAN.md\nTEST-STRATEGY-*\nprivate-sentinel*\n',
        }
        for name, data in files.items():
            write_file(self.root / name, data, 0o755 if name == 'scripts/package.py' else 0o644)
        self.git('init', '-q')
        self.git('add', '.')
        self.git('-c', 'user.name=Packaging Fixture', '-c', 'user.email=fixture@example.invalid',
                 '-c', 'commit.gpgsign=false', '-c', 'core.hooksPath=/dev/null', 'commit', '-qm', 'fixture')
        self.root_patch = patch.object(package, 'ROOT', self.root)
        self.root_patch.start()
        self.addCleanup(self.root_patch.stop)

    def git(self, *args):
        return subprocess.check_output(['git', '-C', str(self.root), *args], stderr=subprocess.PIPE)

    def stage(self):
        inventory = package.approved_inventory()
        destination = self.base / 'staging'
        destination.mkdir()
        package.copy_readers(destination, inventory)
        package.copy_corresponding_source(destination / 'source', inventory)
        package.rewrite_documentation_links(destination)
        return destination, inventory

    def test_absent_notes_and_private_recursive_inputs_never_ship(self):
        for name in ['SESSION.md', 'SESSION-SPEC.md', 'docs/verification/TEAM-EXECUTION-PLAN.md',
                     'docs/verification/TEST-STRATEGY-AUDIT.md', 'docs/verification/TEST-STRATEGY-INVENTORY.json']:
            write_file(self.root / name, b'private fixture sentinel')
        for directory in ['src', 'tests', 'examples', 'scripts', 'assets', 'vendor', 'docs', 'licenses', '.github']:
            write_file(self.root / directory / 'private-sentinel.txt', b'ignored private fixture')
            write_file(self.root / directory / 'untracked.txt', b'unapproved fixture')
        destination, inventory = self.stage()
        names = [p.relative_to(destination).as_posix() for p in destination.rglob('*')]
        self.assertFalse(any('sentinel' in name or 'untracked' in name or Path(name).name in package.PRIVATE_NOTES for name in names))
        for name in inventory:
            self.assertEqual((destination / 'source' / name).read_bytes(), (self.root / name).read_bytes())
        self.assertEqual((destination / 'licenses/item/LICENSE').read_bytes(), b'dependency licence\n')
        self.assertEqual((destination / 'README.md').read_text(), '[patches](source/vendor/README.md)\n')
        self.assertEqual((destination / 'docs/guide.md').read_text(), '[source](../source/src/main.rs)\ncurrent plan\n[public history](https://example.test/SESSION-SPEC.md)\n')
        self.assertEqual(stat.S_IMODE((destination / 'source/scripts/package.py').stat().st_mode), 0o755)
        self.assertFalse((destination / 'source/SESSION-SPEC.md').exists())

    def test_absent_notes_package_and_gitless_repackage_with_same_inventory(self):
        destination, expected = self.stage()
        with patch.object(package, 'ROOT', destination / 'source'), patch.object(package.subprocess, 'check_output', side_effect=AssertionError('Git must not be needed')):
            actual = package.approved_inventory()
            self.assertEqual(actual, expected)
            package.copy_corresponding_source(self.base / 'repackaged', actual)
        self.assertEqual((self.base / 'repackaged' / package.SOURCE_MANIFEST).read_bytes(),
                         (destination / 'source' / package.SOURCE_MANIFEST).read_bytes())

    def test_dirty_tracked_source_stops_before_build(self):
        write_file(self.root / 'vendor/patched.rs', b'uncommitted tracked change')
        with patch.object(package, 'run', side_effect=AssertionError('build must not start')), \
             patch('sys.argv', ['package.py']), self.assertRaisesRegex(ValueError, 'clean frozen'):
            package.main()

    def test_git_modes_and_windows_zip_permissions_ignore_lossy_host_stat(self):
        self.git('config', 'core.filemode', 'false')
        (self.root / 'scripts/package.py').chmod(0o644)
        destination, inventory = self.stage()
        self.assertEqual(inventory['scripts/package.py']['mode'], 0o755)
        self.assertEqual(stat.S_IMODE((destination / 'source/scripts/package.py').stat().st_mode), 0o755)
        # Windows may report this mode even after chmod; ZIP permissions must
        # still come from Git, rather than ZipInfo's host-stat default.
        (destination / 'source/scripts/package.py').chmod(0o644)
        with patch.object(package, 'ROOT', destination / 'source'):
            self.assertEqual(package.approved_inventory(), inventory)
            package.copy_corresponding_source(self.base / 'gitless-restaged')
        self.assertEqual(stat.S_IMODE((self.base / 'gitless-restaged/scripts/package.py').stat().st_mode), 0o755)
        archive = self.base / 'synthetic-windows.zip'
        package.write_archive(destination, archive, 'windows', 'Fixture', inventory)
        with zipfile.ZipFile(archive) as packaged:
            info = packaged.getinfo('Fixture/source/scripts/package.py')
            self.assertEqual(info.create_system, 3)
            self.assertEqual(stat.S_IMODE(info.external_attr >> 16), 0o755)
            manifest = json.loads(packaged.read('Fixture/source/' + package.SOURCE_MANIFEST))
            self.assertEqual(manifest['files']['scripts/package.py']['mode'], 0o755)

    def test_casefolded_private_names_and_destination_aliases_are_rejected(self):
        destination, inventory = self.stage()
        source = destination / 'source'
        manifest = source / package.SOURCE_MANIFEST
        for name in ['session.md', 'Session-Spec.md', 'docs/team-execution-plan.MD',
                     'docs/test-strategy-audit.Md', '.GIT/config', 'docs/trailing.']:
            manifest.write_text(json.dumps({'format': 1, 'files': {**inventory, name: {}}}))
            with self.subTest(name=name), patch.object(package, 'ROOT', source), self.assertRaises(ValueError):
                package.approved_inventory()
        for additions in [{'README.MD': {}}, {'DOCS/other.md': {}},
                          {'docs/caf\u00e9.md': {}, 'docs/cafe\u0301.md': {}}]:
            manifest.write_text(json.dumps({'format': 1, 'files': {**inventory, **additions}}))
            with self.subTest(additions=additions), patch.object(package, 'ROOT', source), self.assertRaisesRegex(ValueError, 'Conflicting'):
                package.approved_inventory()
        for name in ['session.md', 'Session-Spec.md', 'TEAM-execution-plan.md']:
            self.assertFalse(package.approved_path(name))

    def test_missing_inventory_and_changed_or_missing_legal_input_fail_closed(self):
        destination, _ = self.stage()
        source = destination / 'source'
        with patch.object(package, 'ROOT', source):
            write_file(source / 'licenses/item/LICENSE', b'changed')
            with self.assertRaisesRegex(ValueError, 'inventory mismatch'):
                package.approved_inventory()
            (source / package.SOURCE_MANIFEST).unlink()
            with self.assertRaisesRegex(ValueError, 'No approved source inventory'):
                package.approved_inventory()
        manifest = json.loads((destination / 'source' / 'licenses/manifest.json').read_text())
        manifest[0]['files'].append('missing/LICENSE')
        write_file(self.root / 'licenses/manifest.json', json.dumps(manifest).encode())
        with self.assertRaisesRegex(ValueError, 'dependency licence'):
            package.approved_inventory(require_clean=False)

    def test_gitless_manifest_rejects_private_traversal_and_duplicate_entries(self):
        destination, inventory = self.stage()
        source = destination / 'source'
        manifest = source / package.SOURCE_MANIFEST
        for name in ['SESSION-SPEC.md', 'SESSION.md', 'docs/verification/TEAM-EXECUTION-PLAN.md',
                     'docs/verification/TEST-STRATEGY-AUDIT.md', '../outside', '/absolute', 'docs//alias', '.git/config']:
            with self.subTest(name=name), patch.object(package, 'ROOT', source):
                manifest.write_text(json.dumps({'format': 1, 'files': {**inventory, name: {}}}))
                with self.assertRaises(ValueError):
                    package.approved_inventory()
        manifest.write_text('{"format":1,"files":{},"files":{}}')
        with patch.object(package, 'ROOT', source), self.assertRaisesRegex(ValueError, 'Duplicate'):
            package.approved_inventory()
        for data in [{'format': True, 'files': inventory}, {'format': 1, 'files': []}, []]:
            manifest.write_text(json.dumps(data))
            with patch.object(package, 'ROOT', source), self.assertRaisesRegex(ValueError, 'Invalid source inventory'):
                package.approved_inventory()

    @unittest.skipIf(os.name == 'nt', 'Link fixture needs POSIX symlink creation')
    def test_source_symlinks_and_directory_modes_preserved_and_redirects_rejected(self):
        (self.root / 'docs/alias.md').symlink_to('guide.md')
        (self.root / 'docs').chmod(0o775)
        self.git('add', 'docs/alias.md')
        self.git('-c', 'user.name=Packaging Fixture', '-c', 'user.email=fixture@example.invalid',
                 '-c', 'commit.gpgsign=false', '-c', 'core.hooksPath=/dev/null', 'commit', '-qm', 'link fixture')
        destination, inventory = self.stage()
        self.assertEqual(os.readlink(destination / 'source/docs/alias.md'), 'guide.md')
        self.assertEqual(os.readlink(destination / 'docs/alias.md'), 'guide.md')
        self.assertEqual(stat.S_IMODE((destination / 'source/docs').stat().st_mode), 0o775)
        (self.root / 'docs/alias.md').unlink()
        (self.root / 'docs/alias.md').symlink_to(self.base)
        with self.assertRaisesRegex(ValueError, 'leaves the source tree'):
            package.copy_approved_files(self.base / 'refused', inventory)


class DiagnosticIdentityTests(unittest.TestCase):
    def test_fresh_private_marker_matches_bundle_and_run(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve() / 'owned'
            info = package.diagnostic_bundle_info('run-1', root)
            self.assertEqual(info['CFBundleIdentifier'], 'nl.samihsoylu.wireframe.diagnostic.run-1')
            self.assertTrue(info['WireframeDiagnostic'])
            self.assertFalse(info['WireframeEnableUpdater'])
            marker = json.loads((root / 'diagnostic.json').read_text())
            self.assertEqual(marker, {'version': 1, 'run_id': 'run-1', 'bundle_id': info['CFBundleIdentifier'], 'root': str(root)})
            self.assertEqual(stat.S_IMODE(root.stat().st_mode), 0o700)
            self.assertEqual(stat.S_IMODE((root / 'diagnostic.json').stat().st_mode), 0o600)
            with self.assertRaises(ValueError):
                package.diagnostic_bundle_info('run-1', root)

    def test_synthetic_bundle_has_private_identity_and_no_updater_payload(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary).resolve()
            source = base / 'source'
            write_file(source / 'assets/app-icon/Wireframe.icns', b'icon fixture')
            write_file(source / 'LICENSE', b'license fixture')
            binary = base / 'dummy-binary'
            binary.write_bytes(b'non-executable test fixture')
            with patch.object(package, 'ROOT', source), patch.object(package, 'run') as native, patch('sparkle.embed', side_effect=AssertionError('updater factory')):
                inventory = {name: package.source_entry(name) for name in ['assets/app-icon/Wireframe.icns', 'LICENSE']}
                write_file(source / 'assets/private-sentinel.txt', b'private fixture')
                bundle = package.mac_bundle(binary, base, '0.1.0', diagnostic=('run-2', base / 'owned'), inventory=inventory)
            self.assertEqual(bundle.name, 'Wireframe Diagnostic run-2.app')
            info = plistlib.loads((bundle / 'Contents/Info.plist').read_bytes())
            self.assertEqual(info['CFBundleIdentifier'], 'nl.samihsoylu.wireframe.diagnostic.run-2')
            self.assertEqual(info['WireframeDiagnosticRunID'], 'run-2')
            self.assertEqual(info['WireframeDiagnosticRoot'], str(base / 'owned'))
            self.assertFalse(info['WireframeEnableUpdater'])
            self.assertNotIn('SUPublicEDKey', info)
            self.assertFalse((bundle / 'Contents/Frameworks').exists())
            self.assertFalse((bundle / 'Contents/Resources/assets/private-sentinel.txt').exists())
            self.assertEqual(native.call_count, 2)  # signing commands were mocked

    def test_diagnostic_cli_refuses_production_distribution_before_building(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary).resolve()
            for output in [base / 'dist', base / 'dist/private']:
                with self.subTest(output=output), patch.object(package, 'ROOT', base), patch.object(package.platform, 'system', return_value='Darwin'), patch.object(package, 'run', side_effect=AssertionError('build must not start')), patch('sys.argv', ['package.py', '--debug', '--no-archive', '--diagnostic-run-id', 'run-1', '--diagnostic-root', str(base / 'owned'), '--diagnostic-output', str(output)]), self.assertRaises(SystemExit) as error:
                    package.main()
                self.assertEqual(error.exception.code, 2)

    def test_diagnostic_updater_is_rejected_before_marker_or_copy(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve() / 'owned'
            with self.assertRaises(ValueError), patch.object(package, 'run', side_effect=AssertionError('native command')):
                package.mac_bundle(Path('unused'), Path(temporary), '0.1.0', update_info={}, diagnostic=('run-1', root))
            self.assertFalse(root.exists())

    def test_invalid_identity_and_redirected_root_are_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary).resolve()
            for run in ['', 'bad.name', '../escape', 'x' * 41]:
                with self.assertRaises(ValueError):
                    package.diagnostic_bundle_info(run, base / 'unused')
            (base / 'redirect').symlink_to(base, target_is_directory=True)
            with self.assertRaises(ValueError):
                package.diagnostic_bundle_info('run-1', base / 'redirect' / 'owned')


class ArchiveTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.name = 'Wireframe-0.1.0-test'
        self.staging = self.root / self.name
        self.original = self.root / 'original' / 'Wireframe.app'
        self.app = self.staging / 'Wireframe.app'
        write_file(self.original / 'Contents/Info.plist', plistlib.dumps({'CFBundleShortVersionString': '0.1.0'}))
        write_file(self.original / 'Contents/MacOS/wireframe', b'executable fixture\n', 0o755)
        package.copy_payload(self.original, self.app)
        for relative in ('source/Cargo.toml', 'source/Cargo.lock', 'source/src/main.rs',
                         'source/scripts/package.py', 'source/THIRD_PARTY_NOTICES.md',
                         'licenses/manifest.json'):
            write_file(self.staging / relative, b'fixture\n')
        write_file(self.staging / 'source/LICENSE', (package.ROOT / 'LICENSE').read_bytes())
        self.archive = self.root / 'package.zip'

    def framework(self):
        framework = self.original / 'Contents/Frameworks/Example.framework'
        version = framework / 'Versions/A'
        write_file(version / 'Example', b'framework executable\n', 0o755)
        write_file(version / 'Helpers/Update Helper', b'helper executable\n', 0o750)
        write_file(version / 'Resources/license.txt', b'framework license\n')
        (version / 'Resources/empty').mkdir()
        (framework / 'Versions/Current').symlink_to('A', target_is_directory=True)
        (framework / 'Example').symlink_to('Versions/Current/Example')
        (framework / 'Resources').symlink_to('Versions/Current/Resources', target_is_directory=True)
        shutil.rmtree(self.app)
        package.copy_payload(self.original, self.app)

    def test_reader_notices_links_reach_corresponding_source_without_changing_source(self):
        text = '# Notices\n\n[changes](vendor/README.md#patches) and [license](licenses/item/LICENSE).\n'
        write_file(self.staging / 'THIRD_PARTY_NOTICES.md', text.encode())
        write_file(self.staging / 'source/THIRD_PARTY_NOTICES.md', text.encode())
        write_file(self.staging / 'source/vendor/README.md', b'# Patches\n')
        write_file(self.staging / 'licenses/item/LICENSE', b'License text\n')
        package.rewrite_documentation_links(self.staging)
        self.assertEqual((self.staging / 'THIRD_PARTY_NOTICES.md').read_text(),
                         text.replace('(vendor/README.md#patches)', '(source/vendor/README.md#patches)'))
        self.assertEqual((self.staging / 'source/THIRD_PARTY_NOTICES.md').read_text(), text)

    def test_private_note_only_links_are_written_without_other_relocations(self):
        original = 'See [current plan](SESSION-SPEC.md). [Public history](https://example.test/SESSION-SPEC.md).\n'
        write_file(self.staging / 'README.md', original.encode())
        write_file(self.staging / 'source/README.md', original.encode())
        write_file(self.staging / 'docs/guide.md', b'See [current plan](../sEsSion%2DsPeC.md).\n')
        package.rewrite_documentation_links(self.staging)
        self.assertEqual((self.staging / 'README.md').read_text(),
                         'See current plan. [Public history](https://example.test/SESSION-SPEC.md).\n')
        self.assertEqual((self.staging / 'docs/guide.md').read_text(), 'See current plan.\n')
        self.assertEqual((self.staging / 'source/README.md').read_text(), original)
        before = (self.staging / 'README.md').read_bytes()
        package.rewrite_documentation_links(self.staging)
        self.assertEqual((self.staging / 'README.md').read_bytes(), before)

    def test_installer_notices_describe_zip_locations_and_keep_upstream_links(self):
        text = '# Notices\n\nCopyright retained. [changes](source/vendor/README.md#patches), [license](licenses/item/LICENSE), [upstream](https://example.test/license).\n'
        actual = package.installer_notices(text, 'Wireframe source.zip')
        self.assertTrue(actual.startswith('# Notices\n'))
        self.assertIn('[companion ZIP](Wireframe%20source.zip)', actual)
        self.assertIn('Copy it to a writable folder and extract it.', actual)
        self.assertIn('Copyright retained.', actual)
        self.assertIn('changes (`source/vendor/README.md#patches` inside the companion package)', actual)
        self.assertIn('license (`licenses/item/LICENSE` inside the companion package)', actual)
        self.assertIn('[upstream](https://example.test/license)', actual)
        self.assertNotIn('[license](licenses/', actual)

    def test_legacy_framework_free_zip_still_verifies(self):
        # The previous packager omitted directory records.
        package.write_archive(self.staging, self.archive, 'windows', self.name)
        verifier.verify(self.original, self.archive)

    def test_regular_windows_archive_keeps_executable_bytes(self):
        write_file(self.staging / 'wireframe.exe', b'MZ portable fixture\n')
        package.write_archive(self.staging, self.archive, 'windows', self.name)
        with zipfile.ZipFile(self.archive) as archive:
            self.assertEqual(archive.read(self.name + '/wireframe.exe'), b'MZ portable fixture\n')

    def test_existing_dmg_stops_before_app_or_zip_replacement(self):
        out = self.root / 'dist'
        out.mkdir()
        dmg = out / 'Wireframe-0.1.0-darwin-arm64.dmg'
        archive = dmg.with_suffix('.zip')
        dmg.write_bytes(b'previous installer')
        archive.write_bytes(b'previous companion')
        metadata = json.dumps({'packages': [{'version': '0.1.0'}], 'target_directory': str(self.root / 'target')})
        with patch.object(package, 'ROOT', self.root), \
             patch.object(package, 'approved_inventory', return_value={}), \
             patch.object(package, 'run'), \
             patch.object(package.subprocess, 'check_output', return_value=metadata), \
             patch.object(package.platform, 'system', return_value='Darwin'), \
             patch.object(package.platform, 'machine', return_value='arm64'), \
             patch.dict(os.environ, {'CARGO_BUILD_TARGET': ''}), \
             patch('sys.argv', ['package.py']):
            with self.assertRaisesRegex(SystemExit, 'Refusing to replace'):
                package.main()
        self.assertEqual(dmg.read_bytes(), b'previous installer')
        self.assertEqual(archive.read_bytes(), b'previous companion')
        self.assertFalse((out / 'Wireframe.app').exists())

    @unittest.skipIf(os.name == 'nt', 'Link fixture needs POSIX symlink creation')
    def test_linux_tar_keeps_links_and_helper_permissions(self):
        self.framework()
        archive = self.root / 'package.tar.gz'
        package.write_archive(self.staging, archive, 'linux', self.name)
        with tarfile.open(archive) as tar:
            prefix = self.name + '/Wireframe.app/Contents/Frameworks/Example.framework/'
            current = tar.getmember(prefix + 'Versions/Current')
            self.assertTrue(current.issym())
            self.assertEqual(current.linkname, 'A')
            self.assertEqual(tar.getmember(prefix + 'Versions/A/Helpers/Update Helper').mode, 0o750)
            self.assertTrue(tar.getmember(prefix + 'Versions/A/Resources/empty').isdir())

    @unittest.skipIf(os.name == 'nt', 'Link fixture needs POSIX symlink creation')
    def test_windows_archive_refuses_to_flatten_links(self):
        self.framework()
        self.archive.write_bytes(b'previous verified archive')
        with self.assertRaisesRegex(ValueError, 'cannot contain symlinks'):
            package.write_archive(self.staging, self.archive, 'windows', self.name)
        self.assertEqual(self.archive.read_bytes(), b'previous verified archive')
        self.assertEqual(list(self.root.glob('.archive-*')), [])

    @unittest.skipUnless(platform.system() == 'Darwin', 'Native ditto round trip requires macOS')
    def test_native_mac_archive_round_trip(self):
        self.framework()
        package.write_archive(self.staging, self.archive, 'darwin', self.name)
        verifier.verify(self.original, self.archive)
        destination = self.root / 'extracted'
        subprocess.run(['ditto', '-x', '-k', str(self.archive), str(destination)], check=True)
        restored = destination / self.name / 'Wireframe.app'
        expected = verifier.disk_entries(self.original)
        actual = verifier.disk_entries(restored)
        self.assertEqual(expected.keys(), actual.keys())
        for relative, (path, mode) in expected.items():
            other, restored_mode = actual[relative]
            self.assertEqual(mode, restored_mode, relative)
            if path.is_symlink():
                self.assertEqual(os.readlink(path), os.readlink(other), relative)
            elif path.is_file():
                self.assertEqual(path.read_bytes(), other.read_bytes(), relative)

    @unittest.skipUnless(platform.system() == 'Darwin', 'Framework ZIP fixture requires macOS')
    def test_verifier_rejects_changed_bytes_modes_types_links_and_paths(self):
        self.framework()
        package.write_archive(self.staging, self.archive, 'darwin', self.name)
        with zipfile.ZipFile(self.archive) as archive:
            records = [(copy.copy(item), archive.read(item)) for item in archive.infolist()]
        prefix = self.name + '/Wireframe.app/'
        cases = [
            ('bytes', prefix + 'Contents/MacOS/wireframe', 'App file differs'),
            ('mode', prefix + 'Contents/Frameworks/Example.framework/Versions/A/Helpers/Update Helper', 'App permissions differ'),
            ('link', prefix + 'Contents/Frameworks/Example.framework/Versions/Current', 'App link differs'),
            ('type', prefix + 'Contents/Frameworks/Example.framework/Example', 'App entry type differs'),
            ('missing', prefix + 'Contents/MacOS/wireframe', 'App contents differ'),
            ('duplicate', prefix + 'Contents/MacOS/wireframe', 'Duplicate archive entry'),
            ('traversal', prefix + '../escape', 'Invalid archive path'),
        ]
        for mutation, target, expected in cases:
            with self.subTest(mutation=mutation):
                damaged = self.root / (mutation + '.zip')
                with zipfile.ZipFile(damaged, 'w') as output:
                    for original, data in records:
                        item = copy.copy(original)
                        if item.filename == target:
                            if mutation == 'bytes': data += b'changed'
                            if mutation == 'mode': item.external_attr = (stat.S_IFREG | 0o644) << 16
                            if mutation == 'link': data = b'B'
                            if mutation == 'type': item.external_attr = (stat.S_IFREG | 0o755) << 16
                            if mutation == 'missing': continue
                        output.writestr(item, data)
                    if mutation in ('duplicate', 'traversal'):
                        output.writestr(target, b'invalid')
                with self.assertRaisesRegex(ValueError, expected):
                    verifier.verify(self.original, damaged)

    @unittest.skipIf(os.name == 'nt', 'Link fixture needs POSIX symlink creation')
    def test_app_links_must_resolve_inside_bundle(self):
        self.framework()
        link = self.app / 'Contents/Frameworks/Example.framework/Versions/Current'
        for target in ['missing', 'Current', str(self.root), str(link.parent / 'A')]:
            with self.subTest(target=target):
                link.unlink()
                link.symlink_to(target, target_is_directory=True)
                with self.assertRaises(ValueError):
                    verifier.disk_entries(self.app)


if __name__ == '__main__':
    unittest.main()

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

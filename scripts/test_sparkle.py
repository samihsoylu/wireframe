#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Updater configuration and packaging failure boundaries; no network or keys."""
import base64
import json
from pathlib import Path
import plistlib
import tempfile
import unittest
from unittest.mock import patch

import package
import sparkle


class UpdaterTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.path = self.root / 'configuration.json'
        self.valid = {'feed_url': 'https://example.org/appcast.xml',
                      'public_key': base64.b64encode(bytes(range(32))).decode(), 'build': '1'}

    def configure(self, data):
        self.path.write_text(json.dumps(data))
        return sparkle.configuration(self.path)

    def test_public_configuration_keeps_installation_explicit(self):
        info = self.configure(self.valid)
        self.assertEqual(info['CFBundleVersion'], '1')
        self.assertEqual(info['CFBundleIdentifier'], sparkle.BUNDLE_ID)
        self.assertTrue(info['WireframeEnableUpdater'])
        self.assertTrue(info['SUVerifyUpdateBeforeExtraction'])
        self.assertTrue(info['SURequireSignedFeed'])
        self.assertEqual(info['SUSignedFeedFailureExpirationInterval'], 0)
        for key in ['SUEnableAutomaticChecks', 'SUAllowsAutomaticUpdates',
                    'SUAutomaticallyUpdate', 'SUSendProfileInfo']:
            self.assertIs(info[key], False)

    def test_rejects_secret_fields_and_invalid_release_configuration(self):
        cases = [dict(self.valid, private_key='must not embed'), {}, [],
                 dict(self.valid, build=1), dict(self.valid, build='0'),
                 dict(self.valid, build='1.2'), dict(self.valid, build='-1'),
                 dict(self.valid, public_key='bad'),
                 dict(self.valid, public_key=base64.b64encode(bytes(31)).decode())]
        for url in ['http://example.org/feed', 'file:///tmp/feed', 'https://user@example.org/feed',
                    'https://example.org/feed#fragment', 'https://example.org/feed#',
                    'https://example.org:bad/feed', 'https://example.org:99999/feed',
                    'https://example.org/ feed', 'https:///feed']:
            cases.append(dict(self.valid, feed_url=url))
        for data in cases:
            with self.subTest(data=data), self.assertRaises(ValueError):
                self.configure(data)

    def test_cached_archive_is_verified_before_extraction(self):
        cache = self.root / 'cache'
        cache.mkdir()
        (cache / f'Sparkle-{sparkle.VERSION}.tar.xz').write_bytes(b'corrupt cached archive')
        with patch('sparkle.subprocess.run') as process, self.assertRaisesRegex(ValueError, 'checksum'):
            sparkle.embed(self.root / 'app', cache)
        process.assert_not_called()
        self.assertFalse((self.root / 'app').exists())

    def test_failed_framework_stage_preserves_previous_bundle(self):
        (self.root / 'LICENSE').write_text('license')
        (self.root / 'assets').mkdir()
        binary = self.root / 'wireframe'
        binary.write_bytes(b'new binary')
        bundle = self.root / 'Wireframe.app'
        bundle.mkdir()
        (bundle / 'previous').write_bytes(b'prior usable build')
        with patch.object(package, 'ROOT', self.root), patch('package.run'), \
             patch('sparkle.embed', side_effect=ValueError('download failed')):
            with self.assertRaisesRegex(ValueError, 'download failed'):
                package.mac_bundle(binary, self.root, '0.1.0', self.configure(self.valid))
        self.assertEqual((bundle / 'previous').read_bytes(), b'prior usable build')
        self.assertEqual(list(self.root.glob('.bundle-*')), [])

    def test_plain_build_does_not_keep_an_old_updater(self):
        (self.root / 'LICENSE').write_text('license')
        (self.root / 'assets').mkdir()
        binary = self.root / 'wireframe'
        binary.write_bytes(b'new binary')
        stale = self.root / 'Wireframe.app/Contents/Frameworks/Sparkle.framework'
        stale.mkdir(parents=True)
        with patch.object(package, 'ROOT', self.root), patch('package.run'), patch('sparkle.embed') as embed:
            bundle = package.mac_bundle(binary, self.root, '0.1.0')
        embed.assert_not_called()
        self.assertFalse(stale.exists())
        info = plistlib.loads((bundle / 'Contents/Info.plist').read_bytes())
        self.assertNotIn('WireframeEnableUpdater', info)
        self.assertNotIn('SUPublicEDKey', info)
        self.assertEqual((bundle / 'Contents/MacOS/wireframe').read_bytes(), b'new binary')


if __name__ == '__main__':
    unittest.main()

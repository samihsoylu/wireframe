#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Static report reproducibility checks. No Windows executable is launched."""
import json
import os
from pathlib import Path
import struct
import subprocess
import sys
import unittest

from test_windows_resources import ICON, pe_fixture
import verify_windows_beta as verifier

NAMES = ['kernel32.dll', 'KERNEL32.dll', 'User32.dll', 'USER32.dll',
         'KERNEL32.dll', 'advapi32.dll']
EXPECTED = ['advapi32.dll', 'KERNEL32.dll', 'kernel32.dll', 'USER32.dll', 'User32.dll']


def imported_image(names):
    data, meta = pe_fixture()
    # Replace only the fixture's inert import section, preserving its real icon tree.
    data[512:1024] = bytes(512)
    position = (len(names) + 1) * 20
    for index, name in enumerate(names):
        encoded = name.encode('ascii') + b'\0'
        assert position + len(encoded) <= 512
        struct.pack_into('<IIIII', data, 512 + index * 20, 0, 0, 0,
                         0x1000 + position, 0)
        data[512 + position:512 + position + len(encoded)] = encoded
        position += len(encoded)
    struct.pack_into('<II', data, meta['optional'] + 120, 0x1000, (len(names) + 1) * 20)
    return data


def report(names=NAMES):
    return verifier.inspect_pe(imported_image(names), ICON)


class ReportOrderTests(unittest.TestCase):
    def test_exact_spelling_survives_deduplication_and_import_order(self):
        for names in [NAMES, list(reversed(NAMES))]:
            with self.subTest(names=names):
                value = report(names)
                self.assertEqual(value['imports'], EXPECTED)
                self.assertEqual(len(value['imports']), len(set(NAMES)))
                self.assertTrue(value['icon_resource']['verified'])
                self.assertFalse(value['certificate_table_present'])

    def test_complete_report_is_stable_across_hash_seeds(self):
        outputs = []
        for seed in ['0', '1', '2', '3', '4']:
            env = dict(os.environ, PYTHONHASHSEED=seed, PYTHONDONTWRITEBYTECODE='1')
            result = subprocess.run([sys.executable, '-B', str(Path(__file__).resolve()),
                                     '--emit-report'], env=env, check=True,
                                    capture_output=True, text=True, timeout=30)
            self.assertEqual(result.stderr, '')
            value = json.loads(result.stdout)
            self.assertEqual(value['imports'], EXPECTED, 'hash seed ' + seed)
            outputs.append(result.stdout)
        self.assertEqual(len(set(outputs)), 1)


if __name__ == '__main__':
    if sys.argv[1:] == ['--emit-report']:
        print(json.dumps(report(), sort_keys=True))
    else:
        unittest.main()

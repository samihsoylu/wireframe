#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Synthetic PE/packaging checks; never executes a Windows image or builds the app."""
import copy
import json
from pathlib import Path
import shutil
import struct
import tempfile
import unittest
from unittest.mock import patch
import zipfile

import package
import verify_windows_beta as verifier

ICON = (package.ROOT / 'assets/app-icon/Wireframe.ico').read_bytes()
LANGUAGE = 1033


def resources_fixture():
    images = verifier.ico_images(ICON)
    resources = {}
    group = bytearray(struct.pack('<HHH', 0, 1, len(images)))
    for identifier, (descriptor, payload) in enumerate(images.items(), 101):
        resources[(3, identifier, LANGUAGE)] = payload
        group.extend(struct.pack('<BBBBHHIH', *descriptor, identifier))
    resources[(14, 1, LANGUAGE)] = bytes(group)
    return resources


def pe_fixture(resources=None):
    """Minimal inert PE32+ bytes with an import table and a real multi-image resource tree."""
    resources = resources_fixture() if resources is None else resources
    tree = {}
    for (kind, name, language), payload in resources.items():
        tree.setdefault(kind, {}).setdefault(name, {})[language] = payload
    blob = bytearray()
    tables, leaves, payloads = {}, {}, {}

    def allocate(size):
        offset = len(blob)
        blob.extend(b'\0' * size)
        return offset

    def directory(node, path):
        position = allocate(16 + 8 * len(node))
        tables[path] = position
        struct.pack_into('<HH', blob, position + 12, 0, len(node))
        for index, (key, value) in enumerate(sorted(node.items())):
            if isinstance(value, dict):
                target = directory(value, path + (key,)) | 0x80000000
            else:
                target = allocate(16)
                leaves[path + (key,)] = target
            struct.pack_into('<II', blob, position + 16 + index * 8, key, target)
        return position
    directory(tree, ())
    for key, payload in sorted(resources.items()):
        blob.extend(b'\0' * (-len(blob) % 4))
        address = allocate(len(payload))
        blob[address:address + len(payload)] = payload
        payloads[key] = address
        struct.pack_into('<IIII', blob, leaves[key], 0x2000 + address, len(payload), 0, 0)
    resource_size = len(blob)
    raw_size = (resource_size + 511) // 512 * 512
    data = bytearray(1024 + raw_size)
    data[:2] = b'MZ'
    struct.pack_into('<I', data, 60, 128)
    data[128:132] = b'PE\0\0'
    struct.pack_into('<HHIIIHH', data, 132, 0x8664, 2, 0, 0, 0, 240, 2)
    optional = 152
    struct.pack_into('<H', data, optional, 0x20b)
    struct.pack_into('<H', data, optional + 68, 2)
    struct.pack_into('<I', data, optional + 108, 16)
    struct.pack_into('<II', data, optional + 120, 0x1000, 40)
    struct.pack_into('<II', data, optional + 128, 0x2000, resource_size)
    for index, (name, size, rva, raw) in enumerate([(b'.idata', 512, 0x1000, 512),
                                                  (b'.rsrc', raw_size, 0x2000, 1024)]):
        section = optional + 240 + index * 40
        data[section:section + len(name)] = name
        struct.pack_into('<IIII', data, section + 8, size, rva, size, raw)
    struct.pack_into('<IIIII', data, 512, 0, 0, 0, 0x1060, 0x1040)
    data[608:621] = b'KERNEL32.dll\0'
    data[1024:1024 + len(blob)] = blob
    return data, {'optional': optional, 'tables': tables, 'leaves': leaves, 'payloads': payloads}


class ResourceTests(unittest.TestCase):
    def test_committed_five_frames_match_by_resource_reference(self):
        data, _ = pe_fixture()
        result = verifier.inspect_pe(data, ICON)
        self.assertEqual(result['imports'], ['KERNEL32.dll'])
        self.assertFalse(result['certificate_table_present'])
        self.assertEqual(result['icon_resource']['group_id'], 1)
        self.assertEqual([r['width'] for r in result['icon_resource']['images']], [16, 32, 64, 128, 256])
        self.assertTrue(result['icon_resource']['verified'])

    def test_missing_resource_directory(self):
        data, meta = pe_fixture()
        struct.pack_into('<II', data, meta['optional'] + 128, 0, 0)
        with self.assertRaisesRegex(ValueError, 'Missing PE icon'):
            verifier.inspect_pe(data, ICON)

    def test_missing_group_and_image(self):
        for key in [(14, 1, LANGUAGE), (3, 101, LANGUAGE)]:
            with self.subTest(key=key):
                resources = resources_fixture(); resources.pop(key)
                with self.assertRaises(ValueError):
                    verifier.inspect_pe(pe_fixture(resources)[0], ICON)

    def test_payload_mismatch(self):
        resources = resources_fixture()
        value = bytearray(resources[(3, 101, LANGUAGE)]); value[-1] ^= 1
        resources[(3, 101, LANGUAGE)] = bytes(value)
        with self.assertRaisesRegex(ValueError, 'payload differs'):
            verifier.inspect_pe(pe_fixture(resources)[0], ICON)

    def test_directory_cycle(self):
        data, _ = pe_fixture()
        struct.pack_into('<I', data, 1024 + 20, 0x80000000)
        with self.assertRaisesRegex(ValueError, 'Cyclic'):
            verifier.inspect_pe(data, ICON)

    def test_duplicate_resource_id(self):
        data, _ = pe_fixture()
        struct.pack_into('<I', data, 1024 + 24, 3)
        with self.assertRaisesRegex(ValueError, 'Duplicate resource'):
            verifier.inspect_pe(data, ICON)

    def test_invalid_resource_name_counts_and_unsorted_ids(self):
        data, _ = pe_fixture()
        struct.pack_into('<HH', data, 1024 + 12, 1, 1)
        with self.assertRaisesRegex(ValueError, 'named/ID'): verifier.inspect_pe(data, ICON)
        data, _ = pe_fixture()
        first, second = data[1040:1048], data[1048:1056]
        data[1040:1048], data[1048:1056] = second, first
        with self.assertRaisesRegex(ValueError, 'ascending'): verifier.inspect_pe(data, ICON)

    def test_duplicate_group_frame_and_identifier(self):
        for change in ['frame', 'identifier']:
            resources = resources_fixture(); group = bytearray(resources[(14, 1, LANGUAGE)])
            if change == 'frame': group[20:32] = group[6:18]
            else: struct.pack_into('<H', group, 32, 101)
            resources[(14, 1, LANGUAGE)] = bytes(group)
            with self.subTest(change=change), self.assertRaises(ValueError):
                verifier.inspect_pe(pe_fixture(resources)[0], ICON)

    def test_truncated_file_tree_and_payload(self):
        data, meta = pe_fixture()
        with self.assertRaises(ValueError): verifier.inspect_pe(data[:-512], ICON)
        bad = copy.copy(data); struct.pack_into('<HH', bad, 1024 + 12, 0, 65535)
        with self.assertRaises(ValueError): verifier.inspect_pe(bad, ICON)
        bad = copy.copy(data)
        struct.pack_into('<I', bad, 1024 + meta['leaves'][(3, 101, LANGUAGE)] + 4, 0xffffffff)
        with self.assertRaises(ValueError): verifier.inspect_pe(bad, ICON)

    def test_icon_header_and_payload_bounds(self):
        for icon in [ICON[:5], ICON[:20], b'\0\0\2\0' + ICON[4:], ICON[:-1]]:
            with self.subTest(bytes=len(icon)), self.assertRaises(ValueError): verifier.ico_images(icon)

    def test_wrong_group_reference_language(self):
        resources = resources_fixture(); resources[(3, 101, 0)] = resources.pop((3, 101, LANGUAGE))
        with self.assertRaisesRegex(ValueError, 'missing image'):
            verifier.inspect_pe(pe_fixture(resources)[0], ICON)

    def test_existing_signature_and_portability_guards_remain(self):
        data, meta = pe_fixture()
        struct.pack_into('<I', data, meta['optional'] + 148, 1)
        with self.assertRaisesRegex(ValueError, 'certificate'): verifier.inspect_pe(data, ICON)
        data, _ = pe_fixture(); data[608:621] = b'MSVCP140.dll\0'
        with self.assertRaisesRegex(ValueError, 'redistributable'): verifier.inspect_pe(data, ICON)


class CorrespondingSourceTests(unittest.TestCase):
    def test_archive_contains_build_source_and_verifier_rejects_its_removal(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / 'checkout'; root.mkdir()
            for directory in verifier.SOURCE_DIRECTORIES: (root / directory).mkdir()
            for filename in verifier.SOURCE_FILES: (root / filename).write_text('fixture\n')
            (root / 'Cargo.toml').write_text('[package]\nname="wireframe"\nversion="0.1.0-test"\n')
            build = b'fn main() { /* fixture build source */ }\n'; (root / 'build.rs').write_bytes(build)
            icons = root / 'assets/app-icon'; icons.mkdir()
            (icons / 'Wireframe.ico').write_bytes(ICON); (icons / 'windows.rc').write_text('1 ICON "Wireframe.ico"\n')
            license_dir = root / 'licenses/example'; license_dir.mkdir()
            (license_dir / 'LICENSE').write_text('Example terms\n')
            (root / 'licenses/manifest.json').write_text(json.dumps([{'name':'example','files':['example/LICENSE']}]))
            out = Path(temporary) / 'dist'; out.mkdir()
            name = 'Wireframe-0.1.0-test-windows-x86_64'
            staging = Path(temporary) / name; staging.mkdir()
            with patch.object(package, 'ROOT', root): package.copy_corresponding_source(staging / 'source')
            self.assertEqual((staging / 'source/build.rs').read_bytes(), build)
            self.assertEqual((staging / 'source/assets/app-icon/Wireframe.ico').read_bytes(), ICON)
            self.assertEqual((staging / 'source/assets/app-icon/windows.rc').read_text(), '1 ICON "Wireframe.ico"\n')
            for directory in ['assets','licenses']: shutil.copytree(root / directory, staging / directory)
            shutil.copyfile(root / 'LICENSE', staging / 'LICENSE')
            exe = out / (name + '.exe'); image, _ = pe_fixture()
            image.extend(b'\0' * (1024 * 1024 + 1 - len(image))); exe.write_bytes(image)
            payload = staging / name; payload.mkdir(); shutil.copyfile(exe, payload / 'wireframe.exe')
            archive = out / (name + '.zip')
            package.write_archive(staging, archive, 'windows', name)
            def checksums():
                for p in [exe,archive]:p.with_suffix(p.suffix+'.sha256').write_text(verifier.sha256(p.read_bytes())+'  '+p.name+'\n')
            checksums()
            with patch.object(verifier,'ROOT',root):
                report = verifier.verify(out,'a'*40)
                self.assertTrue(report['windows_icon_resource_verified'])
                self.assertFalse(report['runtime_tests_performed']); self.assertFalse(report['application_launched'])
                with zipfile.ZipFile(archive) as z: entries = {n:z.read(n) for n in z.namelist() if n != name+'/source/build.rs'}
                with zipfile.ZipFile(archive,'w') as z:
                    for n,value in entries.items():z.writestr(n,value)
                checksums()
                with self.assertRaisesRegex(ValueError,'source file set'):verifier.verify(out,'a'*40)


if __name__ == '__main__': unittest.main()

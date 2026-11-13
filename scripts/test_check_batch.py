#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Synthetic batch accounting tests: no compiler, product, network or UI."""
import contextlib
import io
import json
import os
from pathlib import Path
from types import SimpleNamespace
import tempfile
import unittest
from unittest.mock import patch

import check
import check_batch as batch
from test_check import completed, listing, output


class BatchTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name).resolve()
        self.summary = self.root / 'summary.json'
        (self.root / 'Cargo.toml').write_text('synthetic manifest')
        self.identity = {'fixture_and_config_sha256': {'Cargo.toml': check.digest(self.root / 'Cargo.toml')}}
        self.targets, self.messages, self.executables = [], [], {}
        for key, kind, name, source in [('lib', 'lib', 'wireframe', 'src/lib.rs'),
                                        ('bin', 'bin', 'wireframe', 'src/main.rs'),
                                        ('test/example', 'test', 'example', 'tests/example.rs')]:
            path = self.root / source
            path.parent.mkdir(exist_ok=True)
            path.write_text('synthetic test source')
            target = {'name': name, 'kind': [kind], 'test': True, 'src_path': str(path)}
            self.targets.append(target)
            executable = self.root / (kind + '-inert')
            executable.write_bytes(b'inert, never executed')
            self.executables[key] = executable
            self.messages.append({'reason': 'compiler-artifact', 'target': target,
                                  'executable': str(executable), 'profile': {
                                      'test': True, 'debug_assertions': True, 'overflow_checks': True}})
        self.messages.append({'reason': 'build-finished', 'success': True})
        self.names = {'lib': ['lib_case', *batch.CRASH, *batch.IGNORED['lib']],
                      'bin': ['app_case', *batch.IGNORED['bin']], 'test/example': ['integration_case']}
        self.args = SimpleNamespace(json=self.summary, offline=True, jobs=2, test_threads=2)

    def metadata(self):
        return json.dumps({'packages': [{'manifest_path': str(self.root / 'Cargo.toml'), 'targets': self.targets}]})

    def responses(self):
        responses = [completed('host: aarch64-apple-darwin\n'), completed(self.metadata()), completed('\n'.join(map(json.dumps, self.messages)))]
        for key, names in self.names.items():
            responses.extend([completed(listing(names)), completed(listing(batch.IGNORED.get(key, {})))])
        for key, names in self.names.items():
            selected, _, _ = batch.select(key, names, list(batch.IGNORED.get(key, {})), 'darwin')
            responses.append(completed(output(selected, filtered=len(names)-len(selected))))
        return responses

    def run_batch(self, responses, platform='darwin', cross=''):
        with patch.object(check, 'ROOT', self.root), patch.object(check, 'source_identity', return_value=self.identity), \
                patch.object(check, 'invoke', side_effect=responses) as invoke, \
                patch.object(batch.sys, 'platform', platform), \
                patch.dict(os.environ, {'CARGO_BUILD_TARGET': cross, 'WIREFRAME_CRASH_TEST_DIR': '/never-use'}), \
                contextlib.redirect_stdout(io.StringIO()):
            code = batch.run(self.args)
        return code, json.loads(self.summary.read_text()), invoke.call_args_list

    def test_discovers_all_targets_before_execution_and_preserves_exclusions(self):
        code, result, calls = self.run_batch(self.responses())
        self.assertEqual(code, 0)
        self.assertEqual(result['status'], 'passed')
        self.assertEqual([result[k] for k in ['expected', 'executed', 'passed']], [3, 3, 3])
        self.assertEqual(result['held'], 3)
        self.assertEqual(result['ignored_discovered'], 5)
        self.assertEqual(result['discovered'], 11)
        self.assertEqual(len(calls), 12)
        for call in calls[3:9]:
            self.assertIn('--list', call.args[0])
        for call in calls[9:]:
            self.assertIn('--exact', call.args[0])
            self.assertNotIn('--ignored', call.args[0])
            self.assertFalse(set(batch.CRASH).intersection(call.args[0]))
        for call in calls:
            self.assertNotIn('WIREFRAME_CRASH_TEST_DIR', call.args[1])

    def test_windows_and_cross_target_stop_before_any_command(self):
        for platform, cross in [('win32', ''), ('darwin', 'x86_64-pc-windows-msvc')]:
            with self.subTest(platform=platform):
                code, result, calls = self.run_batch([], platform, cross)
                self.assertEqual(code, 1)
                self.assertEqual(result['executed'], 0)
                self.assertFalse(calls)

    def test_metadata_requires_all_root_kinds_and_rejects_disabled_target(self):
        for mutation in ('missing', 'disabled', 'duplicate'):
            targets = list(self.targets)
            if mutation == 'missing':
                self.targets.pop()
            elif mutation == 'disabled':
                self.targets[2] = {**self.targets[2], 'test': False}
            else:
                self.targets.append(self.targets[0])
            with self.assertRaises(check.CheckError):
                batch.catalog(self.metadata(), self.root)
            self.targets = targets

    def test_missing_duplicate_or_unsafe_built_artifact_cannot_pass(self):
        declared = batch.catalog(self.metadata(), self.root)
        for rows in [self.messages[1:], self.messages + [self.messages[0]],
                     [{**self.messages[0], 'profile': {'test': True}}, *self.messages[1:]]]:
            with self.assertRaises(check.CheckError):
                batch.artifacts('\n'.join(map(json.dumps, rows)), declared)

    def test_unknown_ignored_or_new_crash_test_stops_before_execution(self):
        for key, name, ignored in [('bin', 'new::ignored', True), ('lib', 'working_copy::crash_tests::new_case', False)]:
            responses = self.responses()
            index = 3 if key == 'lib' else 5
            responses[index] = completed(listing([*self.names[key], name]))
            if ignored:
                responses[index + 1] = completed(listing([*batch.IGNORED[key], name]))
            code, result, calls = self.run_batch(responses)
            self.assertEqual(code, 1)
            self.assertEqual(result['executed'], 0)
            target = next(t for t in result['targets'] if t['target'] == key)
            self.assertIn(name, target['discovered'])
            if ignored:
                self.assertIn(name, target['source_ignored_names'])
            self.assertLessEqual(len(calls), 9)

    def test_only_declared_platform_empty_target_is_allowed(self):
        self.assertEqual(batch.select('test/interface_typography', [], [], 'linux'), ([], {}, {}))
        for key, names, platform in [('test/interface_typography', [], 'darwin'),
                                     ('test/example', [], 'linux'),
                                     ('test/interface_typography', ['unexpected'], 'darwin')]:
            with self.assertRaises(check.CheckError):
                batch.select(key, names, [], platform)
        selected, held, _ = batch.select('test/interface_typography', list(batch.TYPOGRAPHY), [], 'darwin')
        self.assertEqual(selected, [])
        self.assertEqual(len(held), 3)

    def test_failed_or_truncated_result_is_not_retried_or_passed(self):
        for response, count in [(completed(output(['lib_case'], failed=['lib_case'], filtered=5), 101), 1),
                                (completed('test lib_case ... ok\n'), None)]:
            responses = self.responses()
            responses[9] = response
            code, result, calls = self.run_batch(responses)
            self.assertEqual(code, 1)
            self.assertEqual(result['executed'], count)
            self.assertEqual(len(calls), 10)

    def test_build_failure_replaces_previous_green_report(self):
        self.summary.write_text('{"status":"passed"}')
        code, result, calls = self.run_batch([completed('host: aarch64-apple-darwin\n'), completed(self.metadata()), completed('', 101, 'build failed')])
        self.assertEqual(code, 1)
        self.assertEqual(result['executed'], 0)
        self.assertEqual(len(calls), 3)

    def test_configured_cross_target_is_overridden_with_verified_host(self):
        config = self.root / '.cargo/config.toml'
        config.parent.mkdir()
        config.write_text('[build]\ntarget = "x86_64-pc-windows-msvc"\n')
        code, result, calls = self.run_batch(self.responses())
        self.assertEqual(code, 0)
        self.assertEqual(result['host_target'], 'aarch64-apple-darwin')
        command = calls[2].args[0]
        self.assertEqual(command[command.index('--target') + 1], 'aarch64-apple-darwin')
        for text in ['host: x86_64-pc-windows-msvc\n', '',
                     'host: aarch64-apple-darwin\nhost: aarch64-apple-darwin\n']:
            code, result, calls = self.run_batch([completed(text)])
            self.assertEqual(code, 1)
            self.assertEqual(len(calls), 1)

    def test_alias_and_hardlinked_executables_are_rejected_before_discovery(self):
        for alias in ('same', 'symlink', 'hardlink'):
            rows = [dict(row) for row in self.messages]
            path = self.executables['bin']
            if alias != 'same':
                path = self.root / alias
                if alias == 'symlink':
                    path.symlink_to(self.executables['bin'])
                else:
                    path.hardlink_to(self.executables['bin'])
            rows[2]['executable'] = str(path)
            responses = self.responses()
            responses[2] = completed('\n'.join(map(json.dumps, rows)))
            code, result, calls = self.run_batch(responses)
            self.assertEqual(code, 1)
            self.assertEqual(len(calls), 3)
            self.assertEqual(result['executed'], 0)

    def test_ignored_selected_body_is_not_counted_as_executed(self):
        responses = self.responses()
        responses[9] = completed(output(['lib_case'], ignored=['lib_case'], filtered=5))
        code, result, calls = self.run_batch(responses)
        self.assertEqual(code, 1)
        self.assertEqual(len(calls), 10)
        self.assertEqual(result['executed'], 0)
        self.assertEqual(result['passed'], 0)
        self.assertEqual(result['ignored_executed_selection'], 1)
        self.assertEqual(result['targets'][0]['ignored'], 1)

    def test_executable_or_source_drift_fails_even_with_passing_output(self):
        for drift in ('executable', 'source'):
            responses = iter(self.responses())
            count = 0
            def invoke(*unused):
                nonlocal count
                count += 1
                if count == 10:
                    path = self.executables['lib'] if drift == 'executable' else self.root / 'src/lib.rs'
                    path.write_bytes(b'changed after build')
                return next(responses)
            code, result, _ = self.run_batch(invoke)
            self.assertEqual(code, 1)
            self.assertEqual(result['status'], 'failed')
            self.executables['lib'].write_bytes(b'inert, never executed')
            (self.root / 'src/lib.rs').write_text('synthetic test source')


if __name__ == '__main__':
    unittest.main()

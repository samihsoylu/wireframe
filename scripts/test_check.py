#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Runner contract checks with synthetic Cargo/libtest output; no Rust or app execution."""
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


def completed(stdout='', returncode=0, stderr=''):
    return SimpleNamespace(stdout=stdout, returncode=returncode, stderr=stderr)


def listing(names):
    return ''.join(name + ': test\n' for name in names)


def output(names, *, failed=(), ignored=(), filtered=0):
    rows = [f"test {name} ... {'FAILED' if name in failed else 'ignored' if name in ignored else 'ok'}"
            for name in names]
    rows += ['', f"test result: {'FAILED' if failed else 'ok'}. {len(names)-len(failed)-len(ignored)} passed; "
             f"{len(failed)} failed; {len(ignored)} ignored; 0 measured; {filtered} filtered out; finished in 0.01s"]
    return '\n'.join(rows) + '\n'


class AccountingTests(unittest.TestCase):
    names = ('required::one', 'required::two')

    def test_valid_selection_counts_only_executed_checks(self):
        inventory = check.discovery(listing((*self.names, 'not::selected')))
        check.validate_inventory(inventory, self.names)
        result = check.execution(output(self.names, filtered=1), 0, self.names, len(inventory))
        self.assertTrue(result['succeeded'])
        self.assertEqual((result['executed'], result['passed'], result['filtered_out']), (2, 2, 1))

    def test_missing_duplicate_and_zero_required_inventory_fail(self):
        for names, expected in [(self.names[:1], self.names),
                                ((*self.names, self.names[0]), self.names), (self.names, ())]:
            with self.subTest(names=names, expected=expected), self.assertRaises(check.CheckError):
                check.validate_inventory(names, expected)
        with self.assertRaises(check.CheckError):
            check.expected_names((self.names[0], self.names[0]))
        with self.assertRaises(check.CheckError):
            check.discovery('')

    def test_malformed_discovery_and_false_footer_fail(self):
        for text in ['unexpected output\n', listing(self.names) + '3 tests, 0 benchmarks\n',
                     listing(self.names) + '2 tests, 0 benchmarks\n2 tests, 0 benchmarks\n',
                     '2 tests, 0 benchmarks\n' + listing(self.names)]:
            with self.subTest(text=text), self.assertRaises(check.CheckError):
                check.discovery(text)

    def test_incomplete_duplicate_or_unexpected_execution_fails(self):
        for text in [output(self.names[:1]), output((*self.names, self.names[0])),
                     output((self.names[0], 'unexpected::case'))]:
            with self.subTest(text=text), self.assertRaises(check.CheckError):
                check.execution(text, 0, self.names, 2)

    def test_missing_multiple_or_malformed_summary_fails(self):
        good = output(self.names)
        for text in [listing(self.names), good + good, good.replace('2 passed', 'many passed'),
                     good.replace('2 passed', '3 passed'),
                     '\n'.join(good.splitlines()[-1:] + good.splitlines()[:-1]),
                     good + 'test malformed result\n']:
            with self.subTest(text=text), self.assertRaises(check.CheckError):
                check.execution(text, 0, self.names, 2)

    def test_failed_ignored_or_failed_process_is_accounted_but_never_success(self):
        cases = [(output(self.names, failed=self.names[:1]), 101),
                 (output(self.names, ignored=self.names[:1]), 0), (output(self.names), 9)]
        for text, code in cases:
            with self.subTest(code=code, text=text):
                result = check.execution(text, code, self.names, 2)
                self.assertTrue(result['complete'])
                self.assertEqual(result['executed'], 2)
                self.assertFalse(result['succeeded'])

    def test_result_counts_and_discovery_filtered_count_must_agree(self):
        good = output(self.names, filtered=1)
        for text, total in [(good.replace('test required::one ... ok', 'test required::one ... FAILED'), 3),
                            (good, 2)]:
            with self.subTest(text=text, total=total), self.assertRaises(check.CheckError):
                check.execution(text, 0, self.names, total)


class SourceIdentityTests(unittest.TestCase):
    revision = 'a' * 40

    def identity(self, responses):
        with patch.object(check, 'digest', return_value='b' * 64), \
             patch.object(check.subprocess, 'run', side_effect=responses) as run:
            identity = check.source_identity()
        return identity, run.call_args_list

    def test_diff_clean_dirty_error_and_signal_remain_distinct(self):
        for code, expected in [(0, False), (1, True), (128, None), (-15, None)]:
            with self.subTest(code=code):
                identity, calls = self.identity([
                    completed(self.revision + '\n'),
                    completed(returncode=code, stderr='fatal: cannot read index\n' if code == 128 else ''),
                    completed(),
                ])
                self.assertIs(identity['tracked_dirty'], expected)
                self.assertEqual(identity['git_head'], self.revision)
                self.assertEqual(identity['git']['diff']['returncode'], code)
                self.assertEqual(len(calls), 3)
                self.assertNotIn('stdout', identity['git']['diff'])
                if code == 128:
                    self.assertEqual(identity['git']['diff']['stderr'], 'fatal: cannot read index\n')

    def test_revision_error_signal_spawn_and_malformed_output_are_separate(self):
        cases = [completed(returncode=128, stderr='fatal: bad revision\n'),
                 completed(returncode=-15), FileNotFoundError(2, 'private exception text'),
                 completed('not-a-revision')]
        for revision in cases:
            with self.subTest(revision=revision):
                identity, _ = self.identity([revision, completed(), completed()])
                self.assertIsNone(identity['git_head'])
                self.assertIs(identity['tracked_dirty'], False)
                self.assertTrue(identity['git']['tracked_paths']['complete'])
                if isinstance(revision, OSError):
                    self.assertEqual(identity['git']['revision']['spawn_error'], 'FileNotFoundError')
                elif revision.returncode == 0:
                    self.assertEqual(identity['git']['revision']['error'], 'malformed revision output')
                else:
                    self.assertEqual(identity['git']['revision']['returncode'], revision.returncode)
                self.assertNotIn('private exception text', json.dumps(identity))

    def test_diff_spawn_failure_is_unknown_without_erasing_other_outcomes(self):
        identity, _ = self.identity([completed(self.revision), PermissionError(13, 'private path'),
                                     completed('M\0src/example.rs\0')])
        self.assertIsNone(identity['tracked_dirty'])
        self.assertEqual(identity['git_head'], self.revision)
        self.assertEqual(identity['git']['diff']['spawn_errno'], 13)
        self.assertIsNone(identity['git']['diff']['returncode'])
        self.assertIsNone(identity['git']['diff']['stderr'])
        self.assertTrue(identity['git']['tracked_paths']['complete'])
        self.assertNotIn('private path', json.dumps(identity))

    def test_tracked_metadata_preserves_status_and_unusual_names_only(self):
        name = 'src/name with spaces\tand\nnewline.rs'
        identity, calls = self.identity([completed(self.revision), completed(returncode=1),
                                         completed('M\0' + name + '\0D\0removed.rs\0')])
        metadata = identity['git']['tracked_paths']
        self.assertEqual(metadata['returncode'], 0)
        self.assertTrue(metadata['complete'])
        self.assertEqual(metadata['paths'], [
            {'status': 'M', 'path': name, 'path_truncated': False},
            {'status': 'D', 'path': 'removed.rs', 'path_truncated': False},
        ])
        self.assertEqual(calls[2].args[0], ['git', 'diff', '--name-status', '-z', '--no-renames',
                         '--no-ext-diff', '--no-textconv', 'HEAD', '--'])
        self.assertNotIn('config', str(calls))

    def test_tracked_metadata_failures_are_not_empty_success(self):
        for probe in [completed(returncode=128, stderr='fatal: bad index\n'),
                      completed(returncode=-15), FileNotFoundError(2, 'missing'),
                      completed('M\0unterminated'), completed('file contents, not name-status'),
                      completed('R100\0old\0new\0')]:
            with self.subTest(probe=probe):
                identity, _ = self.identity([completed(self.revision), completed(), probe])
                metadata = identity['git']['tracked_paths']
                self.assertFalse(metadata['complete'])
                self.assertEqual(metadata['paths'], [])
                self.assertIs(identity['tracked_dirty'], False)

    def test_stderr_and_tracked_metadata_are_bounded_and_marked(self):
        names = ''.join('M\0' + 'x' * 600 + str(i) + '\0' for i in range(40))
        identity, _ = self.identity([completed(self.revision),
                                     completed(returncode=128, stderr='x' * 2000), completed(names)])
        diff = identity['git']['diff']
        self.assertEqual(len(diff['stderr']), 1024)
        self.assertTrue(diff['stderr_truncated'])
        metadata = identity['git']['tracked_paths']
        self.assertEqual(len(metadata['paths']), 32)
        self.assertTrue(all(len(item['path']) == 512 and item['path_truncated'] for item in metadata['paths']))
        self.assertTrue(metadata['truncated'])
        self.assertFalse(metadata['complete'])

    def test_filter_commands_and_credential_diagnostics_are_redacted_as_a_block(self):
        for text in ["fatal: external filter 'echo secret-value' failed\nsecret-value\n",
                     'fatal: https://user:secret-value@example.invalid/repo\n',
                     'Authorization: Bearer secret-value\n',
                     'x' * 2000 + "\nfilter 'echo secret-value' failed\n"]:
            with self.subTest(text=text):
                identity, _ = self.identity([completed(self.revision),
                                             completed(returncode=128, stderr=text), completed()])
                self.assertTrue(identity['git']['diff']['stderr_redacted'])
                self.assertNotIn('secret-value', json.dumps(identity))
                self.assertEqual(identity['git']['diff']['stderr'], '[redacted sensitive Git diagnostics]')

    def test_git_unavailable_records_three_unknown_independent_outcomes(self):
        identity, _ = self.identity([FileNotFoundError(2, 'missing Git') for _ in range(3)])
        self.assertIsNone(identity['git_head'])
        self.assertIsNone(identity['tracked_dirty'])
        self.assertTrue(all(probe['returncode'] is None for probe in identity['git'].values()))
        self.assertFalse(identity['git']['tracked_paths']['complete'])


class OrchestrationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.summary = self.root / 'report.json'
        (self.root / 'Cargo.toml').write_text('synthetic fixture/config input\n')
        self.identity = {'fixture_and_config_sha256': {'Cargo.toml': check.digest(self.root / 'Cargo.toml')}}
        self.artifacts = {}
        messages = []
        for lane, source in [('lib', 'lib.rs'), ('bin', 'main.rs')]:
            executable = self.root / (lane + '-inert-fixture')
            executable.write_bytes(b'not executable; invoke is mocked')
            self.artifacts[lane] = executable
            messages.append({'reason': 'compiler-artifact', 'target': {'name': 'wireframe', 'kind': [lane],
                'src_path': str(self.root / 'src' / source)}, 'profile': {'test': True,
                'debug_assertions': True, 'overflow_checks': True}, 'executable': str(executable)})
        self.messages = messages + [{'reason': 'build-finished', 'success': True}]
        self.build = '\n'.join(json.dumps(item) for item in self.messages)

    def run_main(self, responses):
        text = io.StringIO()
        with patch.object(check, 'ROOT', self.root), patch.object(check, 'source_identity', return_value=self.identity), \
             patch.object(check, 'invoke', side_effect=responses) as invoke, \
             patch.dict(os.environ, {'CARGO_BUILD_TARGET': '', 'WIREFRAME_INPUT_TRACE': '/never-read'}), \
             contextlib.redirect_stdout(text):
            code = check.main(['fast', '--offline', '--json', str(self.summary)])
        return code, json.loads(self.summary.read_text()), text.getvalue(), invoke.call_args_list

    def success_responses(self):
        responses = [completed(self.build)]
        responses += [completed(listing((*names, 'not::selected'))) for names in check.FAST.values()]
        responses += [completed(output(names, filtered=1)) for names in check.FAST.values()]
        return responses

    def test_one_build_then_only_two_declared_discoveries_and_executions(self):
        code, summary, text, calls = self.run_main(self.success_responses())
        self.assertEqual(code, 0)
        self.assertEqual(summary['status'], 'passed')
        self.assertEqual(summary['source'], self.identity)
        self.assertEqual([summary[key] for key in ['expected', 'selected_discovered', 'executed', 'passed']], [12]*4)
        self.assertEqual(sum(target['discovered_total'] for target in summary['targets']), 14)
        self.assertEqual(len(calls), 5)
        build = calls[0].args[0]
        self.assertIn('--lib', build)
        self.assertIn('--bin', build)
        self.assertNotIn('--all-targets', build)
        for index, execution_index, lane in [(1, 3, 'lib'), (2, 4, 'bin')]:
            self.assertEqual(calls[index].args[0][0], str(self.artifacts[lane]))
            self.assertIn('--list', calls[index].args[0])
            self.assertIn('--exact', calls[execution_index].args[0])
            self.assertEqual(calls[execution_index].args[0][-len(check.FAST[lane]):], list(check.FAST[lane]))
        for call in calls:
            self.assertNotIn('WIREFRAME_INPUT_TRACE', call.args[1])
        self.assertIn('FAST PASSED: 12/12', text)
        self.assertIn('unselected', text)
        self.assertIn('Windows desktop runtime', summary['omitted'])

    def test_failed_build_replaces_stale_green_and_does_not_run_tests(self):
        self.summary.write_text('{"status":"passed"}')
        code, summary, text, calls = self.run_main([completed('', 101, 'synthetic build failure')])
        self.assertEqual(code, 1)
        self.assertEqual(summary['status'], 'failed')
        self.assertEqual(summary['executed'], 0)
        self.assertEqual(len(calls), 1)
        self.assertNotIn('FAST PASSED', text)

    def test_executable_replacement_with_unchanged_results_never_passes(self):
        # Replace either binary at each list/run boundary. Names/results stay green.
        for call_index in range(1, 5):
            for lane in ('lib', 'bin'):
                with self.subTest(call_index=call_index, lane=lane):
                    for executable in self.artifacts.values():
                        executable.write_bytes(b'not executable; invoke is mocked')
                    responses = self.success_responses()
                    calls_seen = 0

                    def replace_during_invoke(argv, env):
                        nonlocal calls_seen
                        index = calls_seen
                        calls_seen += 1
                        if index == call_index:
                            self.artifacts[lane].write_bytes(b'replacement with identical test names/results')
                        return responses[index]

                    code, summary, text, calls = self.run_main(replace_during_invoke)
                    self.assertEqual((code, summary['status']), (1, 'failed'))
                    self.assertIn('test executable changed since build', summary['error'])
                    self.assertNotIn('FAST PASSED', text)
                    self.assertLessEqual(len(calls), 5)

    def test_fixture_config_drift_after_snapshot_never_passes(self):
        for call_index in range(5):
            with self.subTest(call_index=call_index):
                (self.root / 'Cargo.toml').write_text('synthetic fixture/config input\n')
                responses = self.success_responses()
                calls_seen = 0

                def change_fixture_during_invoke(argv, env):
                    nonlocal calls_seen
                    index = calls_seen
                    calls_seen += 1
                    if index == call_index:
                        (self.root / 'Cargo.toml').write_text('concurrent fixture/config change\n')
                    return responses[index]

                code, summary, text, calls = self.run_main(change_fixture_during_invoke)
                self.assertEqual((code, summary['status'], len(calls)), (1, 'failed', 5))
                self.assertEqual(summary['failure_stage'], 'fixture/config verification')
                self.assertIn('fixture/config changed during check', summary['error'])
                self.assertNotIn('FAST PASSED', text)

    def test_missing_discovered_case_stops_before_execution(self):
        lib = check.FAST['lib']
        code, summary, text, calls = self.run_main([completed(self.build), completed(listing(lib[1:]))])
        self.assertEqual((code, len(calls), summary['executed']), (1, 2, 0))
        self.assertEqual(summary['selected_discovered'], 6)
        self.assertFalse(summary['targets'][0]['discovery_complete'])
        self.assertNotIn('FAST PASSED', text)

    def test_missing_second_inventory_runs_no_tests(self):
        responses = self.success_responses()[:3]
        responses[2] = completed(listing(check.FAST['bin'][1:]))
        code, summary, text, calls = self.run_main(responses)
        self.assertEqual((code, len(calls), summary['executed']), (1, 3, 0))
        self.assertFalse(any(target['execution_complete'] for target in summary['targets']))
        self.assertNotIn('FAST PASSED', text)

    def test_incomplete_result_is_unknown_not_green_or_retried(self):
        responses = self.success_responses()[:4]
        responses[3] = completed('test result truncated\n')
        code, summary, text, calls = self.run_main(responses)
        self.assertEqual((code, len(calls)), (1, 4))
        self.assertIsNone(summary['executed'])
        self.assertFalse(summary['targets'][0]['execution_complete'])
        self.assertNotIn('FAST PASSED', text)

    def test_ignored_required_case_fails_with_complete_execution_count(self):
        responses = self.success_responses()[:4]
        lib = check.FAST['lib']
        responses[3] = completed(output(lib, ignored=lib[:1], filtered=1))
        code, summary, text, calls = self.run_main(responses)
        self.assertEqual((code, len(calls), summary['executed'], summary['passed']), (1, 4, 7, 6))
        self.assertTrue(summary['targets'][0]['execution_complete'])
        self.assertEqual(summary['targets'][0]['ignored'], 1)
        self.assertNotIn('FAST PASSED', text)

    def test_build_artifact_missing_duplicate_or_failed_finish_rejected(self):
        for messages in [self.messages[1:], [*self.messages[:1], *self.messages],
                         [*self.messages[:-1], {'reason': 'build-finished', 'success': False}],
                         [*self.messages, {'reason': 'build-finished', 'success': True}],
                         [*self.messages[:-1], {'reason': 'build-finished', 'success': 1}]]:
            with self.subTest(messages=messages), self.assertRaises(check.CheckError):
                check.build_artifacts('\n'.join(json.dumps(item) for item in messages), self.root)
        with self.assertRaises(check.CheckError):
            check.build_artifacts('malformed Cargo output', self.root)

    def test_disabled_profile_safety_is_rejected(self):
        for field in ['debug_assertions', 'overflow_checks']:
            messages = json.loads(json.dumps(self.messages))
            messages[0]['profile'][field] = False
            with self.subTest(field=field), self.assertRaises(check.CheckError):
                check.build_artifacts('\n'.join(json.dumps(item) for item in messages), self.root)


if __name__ == '__main__':
    unittest.main()

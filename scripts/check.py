#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Small host-headless feedback lane. Build/list/run only root lib and bin tests."""
import argparse
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
FAST = {
    'lib': tuple('arrows::tests::' + name for name in (
        'collapse_and_resize_keep_finite_state_for_later_expansion',
        'curve_passes_through_middle_and_bounds_contain_entire_path',
        'labels_on_lines_follow_travel_distance_not_equal_leg_time',
        'middle_handle_half_unit_does_not_drift_during_rebasing',
        'moving_and_switching_modes_preserve_stationary_world_points',
        'relative_middle_round_trips_across_chord_directions',
        'shifted_endpoints_and_middle_snap_preserve_expected_directions',
    )),
    'bin': tuple('app::scenario_tests::' + name for name in (
        'pilot_picker_navigation_selection_cancel_and_input_order',
        'pilot_interrupted_drag_cancel_and_one_step_history',
        'pilot_text_draft_save_history_and_fresh_reopen',
        'pilot_async_open_save_failure_and_stale_ownership',
        'whole_control_no_link_enter_down_escape_preserves_graph_and_history',
    )),
}
CASES = {'arrows': {'lib': FAST['lib']}, **{
    alias: {'bin': (name,)} for alias, name in zip(
        ('picker', 'drag', 'text', 'io', 'link'), FAST['bin'], strict=True)
}}


def selection(cases):
    if not cases:
        return FAST
    wanted = {name for case in cases for names in CASES[case].values() for name in names}
    return {lane: tuple(name for name in names if name in wanted)
            for lane, names in FAST.items() if wanted.intersection(names)}


OMITTED = ['other root tests', 'integration and vendor test suites',
           'native desktop/UI, clipboard, dialogs and OS input',
           'Windows desktop runtime', 'visual/graphics checks', 'release builds']
SUMMARY = re.compile(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; '
                     r'(\d+) ignored; (\d+) measured; (\d+) filtered out; '
                     r'finished in (\d+(?:\.\d+)?)s$')
CASE = re.compile(r'^test (\S+) \.\.\. (ok|FAILED|ignored(?:, .*)?)$')


class CheckError(Exception):
    pass


def require(ok, message):
    if not ok:
        raise CheckError(message)


def expected_names(names):
    require(bool(names) and len(names) == len(set(names)), 'empty or duplicate required selection')


def build_artifacts(output, root, lanes=FAST):
    artifacts = {}
    finished = []
    for line in output.splitlines():
        if not line.strip():
            continue
        try:
            value = json.loads(line)
        except ValueError as error:
            raise CheckError('malformed Cargo JSON output') from error
        require(isinstance(value, dict), 'malformed Cargo record')
        if value.get('reason') == 'build-finished':
            finished.append(value.get('success'))
        if value.get('reason') != 'compiler-artifact' or not value.get('executable') or value.get('profile', {}).get('test') is not True:
            continue
        target = value.get('target', {})
        kind = target.get('kind')
        require(kind in [['lib'], ['bin']], 'unexpected executable test target')
        lane = kind[0]
        source = root / 'src' / ('lib.rs' if lane == 'lib' else 'main.rs')
        require(target.get('name') == 'wireframe' and Path(target.get('src_path', '')).resolve() == source.resolve(),
                'test executable is not the declared root target')
        require(lane not in artifacts, 'duplicate root test executable: ' + lane)
        profile = value['profile']
        require(profile.get('debug_assertions') is True and profile.get('overflow_checks') is True,
                'test assertions and overflow checks must remain enabled')
        executable = Path(value['executable'])
        require(executable.is_file(), 'Cargo test executable is missing')
        artifacts[lane] = executable
    require(len(finished) == 1 and finished[0] is True, 'missing, duplicate or unsuccessful Cargo build result')
    require(set(artifacts) == set(lanes), 'missing or unexpected root test executable')
    return artifacts


def discovery(output):
    names = []
    footer = None
    for line in output.splitlines():
        if not line.strip():
            continue
        match = re.fullmatch(r'(\S+): test', line)
        if match:
            require(footer is None, 'discovery entries follow footer')
            names.append(match[1])
            continue
        summary = re.fullmatch(r'(\d+) tests?, (\d+) benchmarks?', line)
        require(summary is not None and footer is None, 'malformed or duplicate test discovery output')
        footer = tuple(map(int, summary.groups()))
    require(names, 'zero discovered tests')
    require(footer is None or footer == (len(names), 0), 'discovery footer does not match inventory')
    return names


def validate_inventory(names, expected):
    expected_names(expected)
    counts = Counter(names)
    missing = sorted(set(expected) - counts.keys())
    duplicates = sorted(name for name, count in counts.items() if count != 1)
    require(not missing, 'missing required tests: ' + ', '.join(missing))
    require(not duplicates, 'duplicate discovered tests: ' + ', '.join(duplicates))


def execution(output, returncode, expected, discovered_total):
    expected_names(expected)
    rows = [match.groups() for line in output.splitlines() if (match := CASE.fullmatch(line))]
    summaries = [match.groups() for line in output.splitlines() if (match := SUMMARY.fullmatch(line))]
    require(len(summaries) == 1, 'missing or duplicate libtest result summary')
    lines = output.splitlines()
    summary_index = next(i for i, line in enumerate(lines) if SUMMARY.fullmatch(line))
    require(all(i < summary_index for i, line in enumerate(lines) if CASE.fullmatch(line)), 'test results follow summary')
    require(all(not line.startswith('test ') or CASE.fullmatch(line) or SUMMARY.fullmatch(line) for line in lines),
            'malformed test result line')
    status, passed, failed, ignored, measured, filtered, _ = summaries[0]
    passed, failed, ignored, measured, filtered = map(int, (passed, failed, ignored, measured, filtered))
    names = [name for name, _ in rows]
    require(len(names) == len(set(names)), 'duplicate executed test result')
    require(set(names) == set(expected) and len(names) == len(expected), 'incomplete or unexpected executed selection')
    require(passed + failed + ignored + measured == len(expected), 'libtest selected count mismatch')
    require(filtered == discovered_total - len(expected), 'filtered count does not match discovery')
    require(passed == sum(result == 'ok' for _, result in rows) and
            failed == sum(result == 'FAILED' for _, result in rows) and
            ignored == sum(result.startswith('ignored') for _, result in rows), 'case results disagree with summary')
    return {'complete': True, 'executed': len(rows), 'passed': passed, 'failed': failed,
            'ignored': ignored, 'filtered_out': filtered,
            'succeeded': returncode == 0 and status == 'ok' and passed == len(expected) and
                         failed == ignored == measured == 0}


def digest(path):
    result = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            result.update(chunk)
    return result.hexdigest()


def git_stderr(raw):
    # Filter failures can echo configured commands/URLs containing credentials.
    redacted = bool(re.search(r'filter|[a-z][a-z0-9+.-]*://|password|token|credential|authorization', raw, re.I))
    return {'stderr': '[redacted sensitive Git diagnostics]' if redacted else raw[:1024],
            'stderr_redacted': redacted, 'stderr_truncated': len(raw) > 1024}


def tracked_paths(output):
    # --no-renames makes each name-status record exactly status NUL path NUL.
    fields = output.split('\0')
    if not output:
        return {'paths': [], 'complete': True, 'truncated': False}
    if fields[-1] != '' or len(fields) % 2 != 1 or any(
            not re.fullmatch(r'[ADMTUXB]', fields[i]) or not fields[i + 1]
            for i in range(0, len(fields) - 1, 2)):
        return {'paths': [], 'complete': False, 'truncated': False, 'error': 'malformed tracked name-status output'}
    count = (len(fields) - 1) // 2
    paths = [{'status': fields[i], 'path': fields[i + 1][:512],
              'path_truncated': len(fields[i + 1]) > 512}
             for i in range(0, min(count, 32) * 2, 2)]
    truncated = count > 32 or any(item['path_truncated'] for item in paths)
    return {'paths': paths, 'complete': not truncated, 'truncated': truncated}


def source_identity():
    files = ['Cargo.toml', 'Cargo.lock', 'src/app/scenario_tests.rs', 'src/app/test_driver.rs', 'src/arrows.rs']
    identity = {'fixture_and_config_sha256': {name: digest(ROOT / name) for name in files},
                'runner_sha256': digest(Path(__file__).resolve())}
    identity.update(git_head=None, tracked_dirty=None, git={})
    commands = [
        ('revision', ['git', 'rev-parse', 'HEAD']),
        ('diff', ['git', 'diff', '--quiet', 'HEAD', '--']),
        ('tracked_paths', ['git', 'diff', '--name-status', '-z', '--no-renames',
                           '--no-ext-diff', '--no-textconv', 'HEAD', '--']),
    ]
    for name, command in commands:
        try:
            probe = subprocess.run(command, cwd=ROOT, capture_output=True, text=True,
                                   encoding='utf-8', errors='replace')
        except OSError as error:
            # No process stderr exists, and exception text can contain private paths.
            outcome = {'returncode': None, 'stderr': None, 'spawn_error': type(error).__name__,
                       'spawn_errno': error.errno}
        else:
            outcome = {'returncode': probe.returncode, **git_stderr(probe.stderr)}
            if name == 'revision' and probe.returncode == 0:
                revision = probe.stdout.strip()
                if re.fullmatch(r'[0-9a-fA-F]{40}|[0-9a-fA-F]{64}', revision):
                    identity['git_head'] = revision
                else:
                    outcome['error'] = 'malformed revision output'
            elif name == 'diff':
                identity['tracked_dirty'] = {0: False, 1: True}.get(probe.returncode)
            elif name == 'tracked_paths' and probe.returncode == 0:
                outcome.update(tracked_paths(probe.stdout))
        if name == 'tracked_paths' and 'paths' not in outcome:
            outcome.update(paths=[], complete=False, truncated=False)
        identity['git'][name] = outcome
    return identity


def verify_executable(path, recorded_hash):
    require(digest(path) == recorded_hash, 'test executable changed since build: ' + str(path))


def verify_fixtures(identity):
    for name, recorded_hash in identity['fixture_and_config_sha256'].items():
        require(digest(ROOT / name) == recorded_hash, 'fixture/config changed during check: ' + name)


def write_summary(path, summary):
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(mode='w', encoding='utf-8', dir=path.parent,
                                         prefix='.check-', suffix='.json', delete=False) as stream:
            temporary = stream.name
            json.dump(summary, stream, indent=2, allow_nan=False)
            stream.write('\n')
        os.replace(temporary, path)
        temporary = None
    finally:
        if temporary is not None:
            os.unlink(temporary)  # only this operation's generated temporary report


def invoke(argv, env):
    return subprocess.run(argv, cwd=ROOT, env=env, capture_output=True, text=True, encoding='utf-8', errors='replace')


def timed_invoke(argv, env, result, timing):
    before = time.perf_counter()
    try:
        return invoke(argv, env)
    finally:
        result[timing] += time.perf_counter() - before


def cargo_errors(output):
    rendered = []
    for line in output.splitlines():
        try:
            value = json.loads(line)
        except ValueError:
            continue
        if isinstance(value, dict) and value.get('reason') == 'compiler-message':
            message = value.get('message', {})
            if message.get('level') == 'error':
                rendered.append(message.get('rendered') or message.get('message', 'compiler error'))
    return '\n'.join(rendered)[-4000:]


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=['fast'])
    parser.add_argument('--json', type=Path, default=ROOT / 'target/check-fast.json', metavar='PATH',
                        help='machine summary (default: target/check-fast.json)')
    parser.add_argument('--offline', action='store_true', help='use only cached Cargo dependencies')
    parser.add_argument('--jobs', type=int, choices=[1, 2], default=2)
    parser.add_argument('--test-threads', type=int, choices=[1, 2], default=2)
    parser.add_argument('--case', action='append', choices=CASES,
                        help='run only this existing regression group; repeat to combine (default: all)')
    args = parser.parse_args(argv)
    selected = selection(args.case)
    start = time.perf_counter()
    result = {'schema': 1, 'mode': args.mode, 'status': 'running', 'expected': sum(map(len, selected.values())),
              'selection': sorted(set(args.case)) if args.case else ['all'],
              'unselected_fast_tests': [name for lane, names in FAST.items()
                                        for name in names if name not in selected.get(lane, ())],
              'selected_discovered': 0, 'executed': 0, 'passed': 0,
              'build_seconds': 0.0, 'discovery_seconds': 0.0, 'execution_seconds': 0.0,
              'jobs': args.jobs, 'test_threads': args.test_threads,
              'targets': [{'target': lane, 'expected': list(names), 'discovery_complete': False,
                           'discovered_total': None, 'selected_discovered': None,
                           'execution_complete': False, 'executed': 0, 'passed': 0}
                          for lane, names in selected.items()], 'omitted': OMITTED,
              'timing_scope': 'this invocation only; full-cold and whole-suite speed unmeasured'}
    stage = 'setup'
    active = None
    try:
        # Replace any earlier green report before starting work.
        write_summary(args.json, result)
        for expected in selected.values():
            expected_names(expected)
        require(not os.environ.get('CARGO_BUILD_TARGET'), 'unset CARGO_BUILD_TARGET; fast executes host test binaries')
        result['source'] = source_identity()
        env = {key: value for key, value in os.environ.items() if not key.startswith('WIREFRAME_')}
        env['CARGO_TERM_COLOR'] = 'never'
        command = ['cargo', 'test', '--locked', '--no-run']
        if 'lib' in selected:
            command += ['--lib']
        if 'bin' in selected:
            command += ['--bin', 'wireframe']
        command += ['--message-format=json', '-j', str(args.jobs)]
        if args.offline:
            command.append('--offline')
        stage = 'build'
        print(f"Fast scope: {', '.join(result['selection'])}; {result['expected']} required checks. "
              f"Building root {'/'.join(selected)} only.", flush=True)
        built = timed_invoke(command, env, result, 'build_seconds')
        result['build_returncode'] = built.returncode
        require(built.returncode == 0, 'Cargo build failed:\n' + cargo_errors(built.stdout) + built.stderr[-2000:])
        artifacts = build_artifacts(built.stdout, ROOT, selected)
        # Record both identities before discovery; the shared Cargo cache is mutable.
        for target in result['targets']:
            executable = artifacts[target['target']]
            target.update(executable=str(executable), executable_sha256=digest(executable))
        for lane, expected in selected.items():
            active = next(item for item in result['targets'] if item['target'] == lane)
            executable = artifacts[lane]
            stage = lane + ' discovery'
            verify_executable(executable, active['executable_sha256'])
            listed = timed_invoke([str(executable), '--list', '--format', 'terse', '--color', 'never'], env, result, 'discovery_seconds')
            active['discovery_returncode'] = listed.returncode
            verify_executable(executable, active['executable_sha256'])
            require(listed.returncode == 0, 'test discovery failed: ' + listed.stderr[-2000:])
            names = discovery(listed.stdout)
            active.update(discovered_total=len(names), selected_discovered=len(set(names) & set(expected)))
            result['selected_discovered'] += active['selected_discovered']
            validate_inventory(names, expected)
            active['discovery_complete'] = True
        for lane, expected in selected.items():
            active = next(item for item in result['targets'] if item['target'] == lane)
            executable = artifacts[lane]
            stage = lane + ' execution'
            verify_executable(executable, active['executable_sha256'])
            active['executed'] = None  # unknown as soon as execution is attempted
            tested = timed_invoke([str(executable), '--format', 'pretty', '--color', 'never',
                                  '--test-threads=' + str(args.test_threads), '--exact', *expected], env, result, 'execution_seconds')
            active['test_returncode'] = tested.returncode
            verify_executable(executable, active['executable_sha256'])
            accounted = execution(tested.stdout, tested.returncode, expected, active['discovered_total'])
            active.update(accounted, execution_complete=True)
            result['executed'] += accounted['executed']
            result['passed'] += accounted['passed']
            require(accounted['succeeded'], 'required tests failed/ignored/measured or process failed:\n' + tested.stdout[-4000:] + tested.stderr[-2000:])
            print(f"{lane}: {accounted['passed']}/{len(expected)} passed; {active['discovered_total']} listed, "
                  f"{accounted['filtered_out']} unselected.", flush=True)
        require(result['expected'] == result['selected_discovered'] == result['executed'] == result['passed'],
                'fast lane aggregate accounting mismatch')
        stage = 'executable verification'
        for target in result['targets']:
            verify_executable(artifacts[target['target']], target['executable_sha256'])
        stage = 'fixture/config verification'
        verify_fixtures(result['source'])
        result['status'] = 'passed'
    except (CheckError, OSError, ValueError, TypeError, KeyError, AttributeError, KeyboardInterrupt) as error:
        result.update(status='failed', failure_stage=stage, error='interrupted' if isinstance(error, KeyboardInterrupt) else str(error))
        if active is not None and not active['execution_complete'] and active['executed'] is None:
            result['executed'] = None
        print('FAST FAILED (' + stage + '): ' + result['error'], flush=True)
    finally:
        result['total_seconds'] = time.perf_counter() - start
        try:
            write_summary(args.json, result)
        except OSError as error:
            print('Cannot write check summary: ' + str(error), flush=True)
            result['status'] = 'failed'
    print(f"Build {result['build_seconds']:.2f}s; discovery {result['discovery_seconds']:.2f}s; "
          f"test execution {result['execution_seconds']:.2f}s.", flush=True)
    print('Not run: ' + '; '.join(OMITTED) + '. No whole-suite/cold speed claim.', flush=True)
    if result['unselected_fast_tests']:
        print(f"Focused run: {len(result['unselected_fast_tests'])} other fast checks not run. "
              "Run without --case for the complete fast lane.", flush=True)
    print('Summary: ' + str(args.json), flush=True)
    if result['status'] == 'passed':
        print(f"FAST PASSED: {result['passed']}/{result['expected']} required checks executed and passed.", flush=True)
        return 0
    return 1


if __name__ == '__main__':
    raise SystemExit(main())

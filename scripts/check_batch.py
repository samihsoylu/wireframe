#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Broad root batch; uses check.py accounting without changing product tests."""
import json
import os
from pathlib import Path
import sys
import time

import check

CRASH = tuple('working_copy::crash_tests::' + name for name in (
    'interrupted_acknowledgement_does_not_overwrite_external_changes',
    'interrupted_save_as_preserves_source_destination_and_previous_local_edits',
    'killed_writer_retains_last_checkpoint_at_each_save_boundary',
))
TYPOGRAPHY = (
    'heading_weight_changes_ink_without_mutating_existing_regular_layouts',
    'installing_interface_fonts_preserves_canvas_families_and_project_bytes',
    'interface_labels_and_fields_follow_native_sizing_across_display_scales',
)
IGNORED = {
    'lib': {
        'working_copy::crash_tests::child_process': 'Child helper; never execute alone',
        'images::search::tests::commons_provider_search_thumbnail_and_full_preview_smoke': 'Opt-in public network',
    },
    'bin': {
        'app::offscreen_tests::nested_opacity_keeps_overlap_masks_clips_and_tile_boundaries': 'Opt-in graphics',
        'app::offscreen_tests::canvas_text_preserves_raster_coverage_at_actual_size': 'Opt-in graphics',
        'app::review::tests::render_review_states_without_windows': 'Opt-in macOS graphics',
    },
}
OMITTED = ['examples', 'doctests', 'dependency tests', 'Python tooling tests',
           'native UI/clipboard/dialogs, installation and release acceptance',
           'Windows runtime', 'explicit held and source-ignored cases listed per target']


def catalog(output, root):
    packages = [p for p in json.loads(output)['packages']
                if Path(p['manifest_path']).resolve() == root / 'Cargo.toml']
    check.require(len(packages) == 1, 'missing or duplicate root package metadata')
    result = {}
    for target in packages[0]['targets']:
        kind = target['kind']
        if kind not in (['lib'], ['bin'], ['test']):
            continue
        check.require(target.get('test') is True, 'disabled root test target: ' + target['name'])
        key = kind[0] if kind != ['test'] else 'test/' + target['name']
        source = Path(target['src_path']).resolve()
        check.require(source.is_relative_to(root) and source.is_file(), 'invalid root test source')
        check.require(key not in result, 'duplicate target: ' + key)
        result[key] = {'name': target['name'], 'kind': kind, 'source': str(source)}
    check.require('lib' in result and 'bin' in result and any(k.startswith('test/') for k in result),
                  'root library, app and integration targets are required')
    return result


def artifacts(output, declared):
    result, finished, identities = {}, [], set()
    for line in output.splitlines():
        if not line.strip():
            continue
        item = json.loads(line)
        if item.get('reason') == 'build-finished':
            finished.append(item.get('success'))
        if item.get('reason') != 'compiler-artifact' or not item.get('executable') or item.get('profile', {}).get('test') is not True:
            continue
        target = item['target']
        keys = [k for k, d in declared.items() if target['name'] == d['name']
                and target['kind'] == d['kind'] and str(Path(target['src_path']).resolve()) == d['source']]
        check.require(len(keys) == 1, 'unexpected test executable from Cargo')
        key = keys[0]
        check.require(key not in result, 'duplicate executable: ' + key)
        check.require(item['profile'].get('debug_assertions') is True
                      and item['profile'].get('overflow_checks') is True, 'disabled test safety checks')
        path = Path(item['executable']).resolve()
        check.require(path.is_file(), 'missing test executable: ' + key)
        identity = (path.stat().st_dev, path.stat().st_ino)
        check.require(path not in result.values() and identity not in identities, 'test targets share an executable')
        identities.add(identity)
        result[key] = path
    check.require(finished == [True] and result.keys() == declared.keys(), 'incomplete Cargo build inventory')
    return result


def select(key, names, ignored, platform):
    check.require(len(names) == len(set(names)) and len(ignored) == len(set(ignored)), 'duplicate discovery')
    check.require(set(ignored) <= set(names), 'ignored list contains undiscovered tests')
    reasons = IGNORED.get(key, {})
    check.require(set(ignored) <= reasons.keys(), 'unclassified source-ignored test: ' + key)
    check.require(not (set(names) & reasons.keys()) - set(ignored), 'expected opt-in test is no longer ignored')
    held = {}
    if key == 'lib':
        check.require(set(CRASH) <= set(names), 'missing declared crash boundary test')
        check.require({n for n in names if n.startswith('working_copy::crash_tests::')}
                      == set(CRASH) | {'working_copy::crash_tests::child_process'}, 'unclassified crash test')
        held = dict.fromkeys(CRASH, 'Deliberate child termination excluded from this batch')
    if key == 'test/interface_typography':
        check.require(set(names) == (set(TYPOGRAPHY) if platform == 'darwin' else set()),
                      'unexpected native typography inventory')
        held = dict.fromkeys(names, 'Native AppKit adapter excluded from this batch')
    check.require(bool(names) or key == 'test/interface_typography' and platform != 'darwin',
                  'unexpected empty test target: ' + key)
    selected = [n for n in names if n not in ignored and n not in held]
    return selected, held, {n: reasons[n] for n in ignored}


def run(args):
    start = time.perf_counter()
    result = {'schema': 1, 'mode': 'batch', 'status': 'running', 'targets': [],
              'expected': 0, 'discovered': 0, 'ignored_discovered': 0, 'held': 0,
              'executed': 0, 'passed': 0, 'failed': 0, 'ignored_executed_selection': 0,
              'build_seconds': 0.0, 'metadata_seconds': 0.0,
              'discovery_seconds': 0.0, 'execution_seconds': 0.0, 'omitted': OMITTED,
              'jobs': args.jobs, 'test_threads': args.test_threads,
              'timing_scope': 'this invocation; no equivalent-work speedup or native acceptance'}
    stage, active = 'setup', None
    try:
        check.write_summary(args.json, result)
        check.require(sys.platform in ('darwin', 'linux'), 'batch supports macOS/Linux hosts only; Windows runtime held')
        check.require(not os.environ.get('CARGO_BUILD_TARGET'), 'unset CARGO_BUILD_TARGET; batch executes host tests')
        env = {k: v for k, v in os.environ.items() if not k.startswith('WIREFRAME_')}
        env['CARGO_TERM_COLOR'] = 'never'
        result['source'] = check.source_identity()
        result['source']['batch_runner_sha256'] = check.digest(Path(__file__))
        optional = ['--offline'] if args.offline else []
        stage = 'compiler host'
        compiler = check.timed_invoke(['rustc', '-vV'], env, result, 'metadata_seconds')
        check.require(compiler.returncode == 0, 'cannot identify compiler host: ' + compiler.stderr[-2000:])
        result['host_target'] = check.host_target(compiler.stdout, sys.platform)
        stage = 'metadata'
        metadata = check.timed_invoke(['cargo', 'metadata', '--no-deps', '--format-version=1', '--locked', *optional], env, result, 'metadata_seconds')
        check.require(metadata.returncode == 0, 'Cargo metadata failed: ' + metadata.stderr[-2000:])
        declared = catalog(metadata.stdout, check.ROOT)
        for d in declared.values():
            path = Path(d['source'])
            result['source']['fixture_and_config_sha256'][str(path.relative_to(check.ROOT))] = check.digest(path)
        stage = 'build'
        print(f'Batch: build {len(declared)} root targets; no examples or dependency test targets.', flush=True)
        built = check.timed_invoke(['cargo', 'test', '--locked', '--no-run', '--lib', '--bins', '--tests',
                                    '--target', result['host_target'], '--message-format=json', '-j', str(args.jobs), *optional], env, result, 'build_seconds')
        result['build_returncode'] = built.returncode
        check.require(built.returncode == 0, 'Cargo build failed:\n' + check.cargo_errors(built.stdout) + built.stderr[-2000:])
        executables = artifacts(built.stdout, declared)
        for key, path in executables.items():
            result['targets'].append({'target': key, 'executable': str(path), 'executable_sha256': check.digest(path),
                                      'discovery_complete': False, 'execution_complete': False, 'executed': 0, 'passed': 0})
        # Discover every target and validate exclusions before running any test.
        for active in result['targets']:
            stage = active['target'] + ' discovery'
            path = Path(active['executable'])
            inventories = []
            for flags in ([], ['--ignored']):
                check.verify_executable(path, active['executable_sha256'])
                listed = check.timed_invoke([str(path), '--list', '--format', 'terse', '--color', 'never', *flags], env, result, 'discovery_seconds')
                check.verify_executable(path, active['executable_sha256'])
                check.require(listed.returncode == 0, 'discovery failed: ' + listed.stderr[-2000:])
                names = check.discovery(listed.stdout, allow_empty=True)
                inventories.append(names)
                if flags:
                    active['source_ignored_names'] = names
                    result['ignored_discovered'] += len(names)
                else:
                    active['discovered'] = names
                    result['discovered'] += len(names)
            names, ignored = inventories
            selected, held, reasons = select(active['target'], names, ignored, sys.platform)
            active.update(discovery_complete=True, discovered=names, expected=selected, source_ignored=reasons, held=held)
            result['expected'] += len(selected)
            result['held'] += len(held)
        check.require(result['expected'] > 0, 'empty aggregate selection')
        check.write_summary(args.json, result)
        for active in result['targets']:
            expected = active['expected']
            if not expected:
                active['execution_status'] = 'not run; held or platform-empty target'
                continue
            stage = active['target'] + ' execution'
            path = Path(active['executable'])
            check.verify_executable(path, active['executable_sha256'])
            active['executed'] = None
            tested = check.timed_invoke([str(path), '--format', 'pretty', '--color', 'never',
                                        '--test-threads=' + str(args.test_threads), '--exact', *expected], env, result, 'execution_seconds')
            active['test_returncode'] = tested.returncode
            check.verify_executable(path, active['executable_sha256'])
            accounted = check.execution(tested.stdout, tested.returncode, expected, len(active['discovered']))
            active.update(accounted, execution_complete=True)
            result['executed'] += accounted['executed']
            result['passed'] += accounted['passed']
            result['failed'] += accounted['failed']
            result['ignored_executed_selection'] += accounted['ignored']
            check.require(accounted['succeeded'], 'required tests failed:\n' + tested.stdout[-4000:] + tested.stderr[-2000:])
            print(f"{active['target']}: {accounted['passed']} passed", flush=True)
            check.write_summary(args.json, result)
        stage = 'final identity/accounting'
        for target in result['targets']:
            check.verify_executable(Path(target['executable']), target['executable_sha256'])
        check.verify_fixtures(result['source'])
        check.require(check.digest(Path(__file__)) == result['source']['batch_runner_sha256'], 'batch runner changed')
        check.require(result['expected'] == result['executed'] == result['passed'], 'batch accounting mismatch')
        check.require(result['discovered'] == result['expected'] + result['ignored_discovered'] + result['held'], 'discovery accounting mismatch')
        result['status'] = 'passed'
    except (check.CheckError, OSError, ValueError, TypeError, KeyError, AttributeError, KeyboardInterrupt) as error:
        result.update(status='failed', failure_stage=stage, error='interrupted' if isinstance(error, KeyboardInterrupt) else str(error))
        if active is not None and active['executed'] is None:
            result['executed'] = None
        print('BATCH FAILED (' + stage + '): ' + result['error'], flush=True)
    finally:
        result['total_seconds'] = time.perf_counter() - start
        try:
            check.write_summary(args.json, result)
        except OSError as error:
            print('Cannot write batch summary: ' + str(error), flush=True)
            result['status'] = 'failed'
    print(f"BATCH {result['status'].upper()}: {result['passed']}/{result['expected']} selected passed; "
          f"{result['ignored_discovered']} source-ignored; {result['held']} held. Summary: {args.json}", flush=True)
    return 0 if result['status'] == 'passed' else 1

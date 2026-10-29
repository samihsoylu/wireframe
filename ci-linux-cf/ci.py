#!/usr/bin/env python3
"""Exact CF verification controller. Run only after parent adoption/authorization."""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import signal
import stat
import subprocess
import sys
import time

from ci_common import Journal, require, sha
import supervise
import verify_package as package

P = Path(__file__).resolve().parent
COMMIT = 'cf7694af89823dcf3b55f2d8423c73700bc5de21'
PACKAGE = 'Wireframe-0.1.0-beta.3-linux-aarch64'
FIXED = {'CARGO_BUILD_TARGET', 'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS',
         'RUSTC', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER'}
GIB = 1024**3
PAYLOAD_CAP = 8 * GIB
FREE_GATE = 10 * GIB
RESERVE = GIB


def read(name):
    return json.loads((P / name).read_text())


def normalized(inherited, work, offline):
    removed = sorted(k for k in inherited if k in FIXED or k.startswith('CARGO_PROFILE_')
                     or k == 'WIREFRAME_INPUT_TRACE')
    require(len(removed) <= 65 and all(len(k) <= 256 for k in removed), 'removed-key bound')
    env = {k: v for k, v in inherited.items() if k not in removed}
    # The accepted six-key normalizer is retained. Refuse additional compiler
    # configuration rather than silently inventing a second profile policy.
    require(not any(k.endswith('_RUSTFLAGS') or k.startswith('CARGO_BUILD_')
                    for k in env if k != 'CARGO_BUILD_JOBS'), 'additional compiler override')
    env.update(CARGO_HOME=str(work / 'cargo'), CARGO_TARGET_DIR=str(work / 'payload/target'),
               CARGO_BUILD_JOBS='1', CARGO_INCREMENTAL='0', CARGO_NET_OFFLINE=str(offline).lower(),
               RUSTUP_TOOLCHAIN='1.99.1', RUSTUP_SKIP_UPDATE_CHECK='1',
               PYTHONDONTWRITEBYTECODE='1', CARGO_TERM_COLOR='never',
               TMPDIR=str(work / 'payload/tmp'),
               XDG_CONFIG_HOME=str(work / 'payload/home/config'),
               XDG_DATA_HOME=str(work / 'payload/home/data'),
               XDG_CACHE_HOME=str(work / 'payload/home/cache'))
    require('WIREFRAME_INPUT_TRACE' not in env and not any(k in env for k in FIXED)
            and not any(k.startswith('CARGO_PROFILE_') for k in env), 'normalization failed')
    return env, {'removedKeys': removed, 'removedCount': len(removed),
                 'traceAbsent': True, 'compilerEnvironmentNormalized': True,
                 'jobs': 1, 'incremental': 0, 'offline': offline}


def plan():
    rows = read('validation-commands.json')
    groups = read('groups.json')
    require(len(rows) == 19 and len(groups) == 5, 'required row/job count differs')
    require(len({r['label'] for r in rows}) == 19, 'duplicate command label')
    require(sorted(x for labels in groups.values() for x in labels)
            == sorted(r['label'] for r in rows), 'job partition differs')
    tests = [r for r in rows if 'expectedIdentities' in r]
    names = [x for r in tests for x in r['expectedIdentities']]
    require(len(tests) == 13 and len(names) == len(set(names)) == 101, '101 distinct scope differs')
    require(set(names) == set(read('expected-union.json')), 'required identity union differs')
    for row in tests:
        require(len(row['expectedIdentities']) == row['expectedPassed'], 'expected count differs')
        require(row['argv'][:4] == ['cargo', 'test', '--locked', '--offline']
                and row['argv'][-2:] == ['--', '--test-threads=1'], 'test semantics changed')
    require(next(r for r in rows if r['label'] == 'optimized-build')['argv']
            == ['cargo', 'build', '--locked', '--offline', '--release'], 'release semantics changed')
    require(next(r for r in rows if r['label'] == 'package')['argv']
            == ['python3', 'scripts/package.py'], 'package semantics changed')
    return rows, groups


def parse_tests(stdout, expected):
    # Plain libtest output: require each named success, exact totals, no ignored,
    # duplicate, missing, unexpected or ambiguous functions. Zero selection fails.
    matches = re.findall(r'^test (\S+) \.\.\. (\S+)\s*$', stdout, re.M)
    actual = []
    for name, status in matches:
        candidates = [x for x in expected if x.split('::', 1)[1] == name]
        require(status == 'ok' and len(candidates) == 1, 'unexpected/failed/ambiguous test: ' + name)
        actual.append(candidates[0])
    summaries = re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', stdout, re.M)
    require(bool(summaries) and sum(int(x[0]) for x in summaries) == len(expected)
            and all(x[1] == x[2] == '0' for x in summaries)
            and len(actual) == len(set(actual)) and set(actual) == set(expected),
            'executed test union/count differs')
    return sorted(actual)


def verify_source(repo):
    manifest = read('source-manifest.json')
    git_env = {**os.environ, 'GIT_OPTIONAL_LOCKS': '0'}
    require(len(manifest) == 1823 and sha(repo / 'Cargo.lock')
            == '662aa827119463a72cb7f1739420df7743b58fdab7f3f49abcfa718c03695cc1', 'source/lock count differs')
    require(subprocess.check_output(['git', '-C', str(repo), 'rev-parse', 'HEAD'], env=git_env).decode().strip()
            == COMMIT, 'product checkout is not exact CF')
    require(not subprocess.check_output(['git', '-C', str(repo), 'status', '--porcelain', '--untracked-files=all'], env=git_env),
            'product checkout is dirty')
    contents, modes = package.git_frozen(repo, P / 'source-manifest.json')
    for name, content in contents.items():
        path = repo / name
        require(not path.is_symlink() and path.is_file() and sha(path) == manifest[name]['sha256'],
                'product bytes/type differ: ' + name)
        require(stat.S_IMODE(path.stat().st_mode) == modes[name], 'product mode differs: ' + name)
    return contents, modes


def allocated(root):
    total, seen = 0, set()
    for directory, dirs, files in os.walk(root, followlinks=False):
        for name in ['.', *dirs, *files]:
            path = Path(directory) / name
            st = path.lstat()
            # This is called only after the supervisor proves kernel ECHILD.
            # Count symlink inode storage, never traverse its target. Source
            # byte/type checks separately prohibit source/package symlinks.
            key = (st.st_dev, st.st_ino)
            if key not in seen:
                seen.add(key)
                total += max(st.st_blocks * 512, ((st.st_size + 4095) // 4096) * 4096)
    return total


def check_abi(text):
    versions = {(int(a), int(b)) for a, b in re.findall(r'GLIBC_(\d+)\.(\d+)', text)}
    require(bool(versions) and max(versions) <= (2, 36), 'ELF requires newer than Bookworm glibc')
    require('GLIBC_PRIVATE' not in text, 'private glibc ABI requirement')
    return '.'.join(str(x) for x in max(versions))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('phase', choices=['prepare', 'verify'])
    parser.add_argument('--group', required=True, choices=list(read('groups.json')))
    parser.add_argument('--repo', required=True, type=Path)
    parser.add_argument('--work', required=True, type=Path)
    args = parser.parse_args()
    os.umask(0o022)
    def interrupted(number, frame):
        raise RuntimeError('owned CI wrapper interrupted: ' + str(number))
    for number in (signal.SIGTERM, signal.SIGHUP, signal.SIGINT):
        signal.signal(number, interrupted)
    rows, groups = plan()
    repo, work = args.repo.resolve(), args.work.resolve()
    require(repo != work and not work.is_relative_to(repo) and not repo.is_relative_to(work), 'work/source overlap')
    require(work.is_dir() and not work.is_symlink(), 'explicit work directory required')
    receipts = work / 'receipts'
    receipts.mkdir(exist_ok=True)
    journal = Journal(receipts / 'jobs.jsonl')
    stage = {'phase': args.phase, 'group': args.group, 'productCommit': COMMIT,
             'workflowCommit': os.environ.get('GITHUB_SHA'), 'runId': os.environ.get('GITHUB_RUN_ID'),
             'runAttempt': os.environ.get('GITHUB_RUN_ATTEMPT'), 'controllerSHA256': sha(P / 'implementation-manifest.json'),
             'success': False}
    stage['sourceBinding'] = read('source-binding.json')
    stage['buildImage'] = read('image-pin.json')['arm64ManifestDigest']
    try:
        for name, expected in read('implementation-manifest.json').items():
            require(sha(P / name) == expected, 'CI implementation drift: ' + name)
        contents, modes = verify_source(repo)
        env, proof = normalized(os.environ, work, args.phase == 'verify')
        stage['environment'] = proof
        if args.phase == 'prepare':
            require(not (work / 'payload').exists() and not (receipts / 'prepare.json').exists(), 'already attempted')
            require(shutil.disk_usage(work).free >= FREE_GATE, 'initial 10GiB free gate')
            for path in ['cargo', 'payload/target', 'payload/tmp', 'payload/home', 'payload/source']:
                (work / path).mkdir(parents=True, exist_ok=True)
            for name, content in contents.items():
                path = work / 'payload/source' / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(content)
                path.chmod(modes[name])
        else:
            previous = json.loads((receipts / 'prepare.json').read_text())
            require(previous['success'] and previous['group'] == args.group
                    and previous['productCommit'] == COMMIT and previous['workflowCommit'] == stage['workflowCommit'],
                    'missing/mismatched successful preparation')
            require(shutil.disk_usage(work).free >= FREE_GATE, 'post-fetch 10GiB free gate')
        source, target = work / 'payload/source', work / 'payload/target'
        package.verify_frozen_source(source, contents, modes, read('directory-modes.json'))

        def execute(label, argv, seconds=21600):
            minimum_free = shutil.disk_usage(work).free
            def free():
                nonlocal minimum_free
                observed = shutil.disk_usage(work).free
                minimum_free = min(minimum_free, observed)
                return observed
            result = supervise.run_owned(argv, source, env, receipts, label, journal,
                                          timeout=seconds, reserve=RESERVE, free_bytes=free)
            # Compiler/test temp trees are mutable. Do not walk them concurrently
            # or treat a process snapshot as child quiescence. The retained
            # supervisor above proves kernel ECHILD before this stable check.
            stable = allocated(work / 'payload')
            journal.record('stable-owned-payload', label=label, bytes=stable,
                           sampledMinimumFreeBytes=minimum_free, peakPayloadBytesUnknown=True)
            require(stable <= PAYLOAD_CAP, 'stable owned payload exceeds 8GiB; refuse subsequent command')
            result.update(stablePayloadBytes=stable, sampledMinimumFreeBytes=minimum_free,
                          peakPayloadBytesUnknown=True)
            return result

        # Same versions/commit dates as the retained native toolchain; executable
        # hashes may differ and are captured rather than falsely equated.
        if args.phase == 'prepare':
            expected = read('expected-toolchain.json')
            for name, argv, key in [('rustc', ['rustc', '--version'], 'rustcVersion'),
                                    ('cargo', ['cargo', '--version'], 'cargoVersion'),
                                    ('clippy', ['cargo', 'clippy', '--version'], 'clippyVersion')]:
                result = execute('tool-' + name, argv, 120)
                require(Path(result['stdout']).read_text() == expected[key], 'toolchain version differs: ' + name)
            result = execute('tool-sysroot', ['rustc', '--print', 'sysroot'], 120)
            sysroot = Path(Path(result['stdout']).read_text().strip())
            stage['toolBinaries'] = {n: {'sha256': sha(sysroot / 'bin' / n),
                                         'bytes': (sysroot / 'bin' / n).stat().st_size}
                                    for n in ['cargo', 'rustc', 'clippy-driver']}
            stage['osRelease'] = Path('/etc/os-release').read_text()
            require('VERSION_CODENAME=bookworm' in stage['osRelease'] and os.uname().machine == 'aarch64',
                    'native Bookworm aarch64 required')
            execute('system-packages', ['dpkg-query', '-W', 'libc6', 'libgcc-s1', 'gcc', 'binutils'], 120)
            execute('fetch-locked', ['cargo', 'fetch', '--locked', '--target', 'aarch64-unknown-linux-gnu'], 1800)
            # --target above limits dependency FETCH only. Actual builds use the
            # original native command and cache namespace, with no --target.
        else:
            actual = []
            stage['commands'] = []
            for label in groups[args.group]:
                row = next(r for r in rows if r['label'] == label)
                argv = row['argv'][:]
                if label in ('static-file', 'static-readelf', 'static-loader'):
                    argv[-1] = str(target / 'release/wireframe')
                result = execute(label, argv)
                if 'expectedIdentities' in row:
                    result['actualTestIdentities'] = parse_tests(Path(result['stdout']).read_text(), row['expectedIdentities'])
                    result['actualPassed'] = len(result['actualTestIdentities'])
                    actual += result['actualTestIdentities']
                stage['commands'].append(result)
                if label == 'optimized-build':
                    binary = target / 'release/wireframe'
                    (receipts / 'build-record.json').write_text(json.dumps({
                        'source_commit': COMMIT, 'workflow_commit': stage['workflowCommit'],
                        'profile': 'release', 'build_exit_code': 0, 'argv': argv,
                        'binary_sha256': sha(binary), 'environment': proof}, indent=2) + '\n')
            expected_names = [x for row in rows if row['label'] in groups[args.group]
                              for x in row.get('expectedIdentities', [])]
            require(sorted(actual) == sorted(expected_names), 'job affected-test scope differs')
            stage['actualTestIdentities'] = sorted(actual)
            stage['actualPassed'] = len(actual)
            if args.group == 'release':
                binary = target / 'release/wireframe'
                package.elf_identity(binary.read_bytes())
                versions = execute('static-symbol-versions', ['readelf', '--version-info', str(binary)], 120)
                stage['highestGLIBC'] = check_abi(Path(versions['stdout']).read_text())
                dynamic = execute('static-needed', ['readelf', '-d', str(binary)], 120)
                stage['neededLibraries'] = re.findall(r'\(NEEDED\).*?\[(.*?)\]', Path(dynamic['stdout']).read_text())
                require(set(stage['neededLibraries']) <= {'libgcc_s.so.1', 'libm.so.6', 'libc.so.6'},
                        'new ELF dependency needs compatibility review')
                require('not found' not in Path(receipts / 'static-loader.out').read_text(), 'build-container loader unresolved')
                (receipts / 'filesystem-modes.json').write_text(json.dumps({n: oct(modes[n]) for n in modes}) + '\n')
                dist = source / 'dist'
                archive = dist / (PACKAGE + '.tar.gz')
                checksum = dist / (PACKAGE + '.tar.gz.sha256')
                execute('verify-package', ['python3', str(P / 'verify_package.py'),
                    '--repo', str(repo), '--frozen-source', str(source),
                    '--source-manifest', str(P / 'source-manifest.json'),
                    '--frozen-filesystem-modes', str(receipts / 'filesystem-modes.json'),
                    '--frozen-directory-modes', str(P / 'directory-modes.json'),
                    '--archive', str(archive), '--checksum', str(checksum),
                    '--validated-binary', str(binary), '--build-record', str(receipts / 'build-record.json'),
                    '--output', str(work / 'payload/package-verification')], 1800)
                verification = work / 'payload/package-verification'
                report = json.loads((verification / 'verification.json').read_text())
                require(report['corresponding_source_files_exact'] == 1821, 'corresponding-source count differs')
                artifacts = work / 'artifacts'
                artifacts.mkdir(exist_ok=False)
                require(binary.stat().st_size < 64 * 1024**2 and archive.stat().st_size < 64 * 1024**2,
                        'artifact byte bound')
                for item in [binary, archive, checksum, receipts / 'build-record.json', P / 'source-manifest.json']:
                    shutil.copy2(item, artifacts / item.name)
                for item in verification.glob('*.json'):
                    shutil.copy2(item, artifacts / item.name)
                (artifacts / 'wireframe.sha256').write_text(sha(binary) + '  wireframe\n')
                stage['packageStaticReportSHA256'] = sha(artifacts / 'verification.json')
                stage['nativeAcceptance'] = 'pending parent local isolated execution and review'
        package.verify_frozen_source(source, contents, modes, read('directory-modes.json'))
        verify_source(repo)
        stage.update(success=True, ownedPayloadBytes=allocated(work / 'payload'), freeBytes=shutil.disk_usage(work).free)
    except BaseException as error:
        stage['failure'] = type(error).__name__ + ': ' + str(error)
        raise
    finally:
        destination = receipts / (args.phase + '.json')
        with destination.open('x') as stream:
            json.dump(stage, stream, indent=2)
            stream.write('\n')
        print(json.dumps({'phase': args.phase, 'group': args.group, 'success': stage['success'],
                          'productCommit': COMMIT, 'workflowCommit': stage['workflowCommit']}))


if __name__ == '__main__':
    main()

#!/usr/bin/env python3
"""Full ordinary Linux suite/controller. Requires exact reviewed product binding."""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import signal
import stat
import sys

from ci_common import Journal, require, sha
import account
import bind
import native
import runtime
import supervise
import verify_package as package

P = Path(__file__).resolve().parent
GIB = 1024**3
RESERVE, FREE_GATE, STABLE_CAP = GIB, 10 * GIB, 8 * GIB
PACKAGE = 'Wireframe-0.1.0-beta.4-linux-aarch64'


def source_identity(repo, binding):
    package.COMMIT = binding['productCommit']
    manifest = binding['sourceManifest']
    actual = bind.snapshot(repo, binding['productCommit'])
    require(actual == manifest, 'source snapshot differs from reviewed binding')
    contents = {n: (repo / n).read_bytes() for n in manifest}
    modes = {n: int(r['mode'], 8) for n, r in manifest.items()}
    directories = {str(parent): '0o755' for n in manifest for parent in Path(n).parents if str(parent) != '.'}
    return contents, modes, directories


def verify_controller():
    manifest = bind.read('implementation-manifest.json')
    require(set(manifest) == {p.name for p in P.iterdir() if p.is_file()} - {'implementation-manifest.json'},
            'controller file inventory differs')
    for name, expected in manifest.items():
        require(not (P / name).is_symlink() and sha(P / name) == expected, 'controller bytes/type differ: ' + name)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('phase', choices=['prepare', 'verify'])
    parser.add_argument('--group', required=True)
    parser.add_argument('--repo', required=True, type=Path)
    parser.add_argument('--work', required=True, type=Path)
    args = parser.parse_args()
    binding = bind.assert_armed()  # Refuse before source/copy/compiler/tool work.
    verify_controller()
    require(args.group in binding['groups'], 'unknown verification group')
    os.umask(0o022)
    def interrupted(number, frame):
        raise RuntimeError('owned CI wrapper interrupted: ' + str(number))
    for number in (signal.SIGTERM, signal.SIGHUP, signal.SIGINT):
        signal.signal(number, interrupted)
    repo, work = args.repo.resolve(), args.work.resolve()
    require(work.is_dir() and not work.is_symlink() and not repo.is_relative_to(work)
            and not work.is_relative_to(repo), 'source/work isolation')
    receipts = work / 'receipts'
    receipts.mkdir(exist_ok=True)
    journal = Journal(receipts / 'jobs.jsonl')
    stage = {'phase': args.phase, 'group': args.group, 'productCommit': binding['productCommit'],
             'controllerCommit': os.environ.get('GITHUB_SHA'), 'runId': os.environ.get('GITHUB_RUN_ID'),
             'runAttempt': os.environ.get('GITHUB_RUN_ATTEMPT'), 'bindingSHA256': sha(P / 'product.json'),
             'implementationSHA256': sha(P / 'implementation-manifest.json'), 'success': False,
             'harnesses': [], 'commands': []}
    try:
        contents, modes, directories = source_identity(repo, binding)
        source, target = work / 'payload/source', work / 'payload/target'
        env, proof = runtime.normalized(os.environ, work, args.phase == 'verify')
        stage['environment'] = proof
        if args.phase == 'prepare':
            require(not (work / 'payload').exists() and not (receipts / 'prepare.json').exists(), 'already attempted')
            require(shutil.disk_usage(work).free >= FREE_GATE, 'initial 10GiB free gate')
            for folder in ['cargo', 'payload/source', 'payload/target', 'payload/tmp', 'payload/home']:
                (work / folder).mkdir(parents=True, exist_ok=True)
            for name, data in contents.items():
                path = source / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(data)
                path.chmod(modes[name])
        else:
            previous = json.loads((receipts / 'prepare.json').read_text())
            require(previous['success'] and all(previous[k] == stage[k] for k in
                    ['group', 'productCommit', 'controllerCommit', 'bindingSHA256', 'implementationSHA256']),
                    'preparation receipt missing or mismatched')
            require(shutil.disk_usage(work).free >= FREE_GATE, 'post-fetch 10GiB free gate')
        package.verify_frozen_source(source, contents, modes, directories)
        attestation = native.source_attestation(source)
        (receipts / ('ocr-source-' + args.phase + '.json')).write_text(json.dumps(attestation, indent=2) + '\n')

        def execute(label, argv, seconds=21600):
            minimum = shutil.disk_usage(work).free
            def free():
                nonlocal minimum
                observed = shutil.disk_usage(work).free
                minimum = min(minimum, observed)
                return observed
            result = supervise.run_owned(argv, source, env, receipts, label, journal,
                                          timeout=seconds, reserve=RESERVE, free_bytes=free)
            stable = runtime.allocated(work / 'payload')  # Only after kernel ECHILD.
            journal.record('stable-owned-payload', label=label, bytes=stable,
                           sampledMinimumFreeBytes=minimum, peakPayloadBytesUnknown=True)
            require(stable <= STABLE_CAP, 'stable payload >8GiB; stop before next command')
            result.update(stablePayloadBytes=stable, sampledMinimumFreeBytes=minimum,
                          peakPayloadBytesUnknown=True)
            stage['commands'].append(result)
            return result

        if args.phase == 'prepare':
            require(os.uname().machine == 'aarch64' and 'VERSION_CODENAME=bookworm' in Path('/etc/os-release').read_text(),
                    'native Bookworm ARM64 container required')
            expected = bind.read('expected-toolchain.json')
            for label, argv, key in [('tool-rustc', ['rustc', '--version'], 'rustcVersion'),
                                     ('tool-cargo', ['cargo', '--version'], 'cargoVersion'),
                                     ('tool-clippy', ['cargo', 'clippy', '--version'], 'clippyVersion')]:
                record = execute(label, argv, 120)
                require(Path(record['stdout']).read_text() == expected[key], 'Rust toolchain differs')
            record = execute('tool-sysroot', ['rustc', '--print', 'sysroot'], 120)
            sysroot = Path(Path(record['stdout']).read_text().strip())
            stage['toolBinaries'] = {n: {'sha256': sha(sysroot / 'bin' / n),
                                         'bytes': (sysroot / 'bin' / n).stat().st_size}
                                    for n in ['cargo', 'rustc', 'clippy-driver']}
            for name in ['cmake', 'g++', 'make']:
                path = Path(shutil.which(name))
                stage['toolBinaries'][name] = {'sha256': sha(path), 'bytes': path.stat().st_size}
            for label, argv in [('tool-cmake', ['cmake', '--version']), ('tool-cxx', ['g++', '--version']),
                                 ('tool-make', ['make', '--version']),
                                 ('system-packages', ['dpkg-query', '-W', 'libc6', 'libstdc++6', 'gcc', 'g++', 'cmake', 'make'])]:
                record = execute(label, argv, 120)
                if label == 'tool-cmake':
                    version = re.search(r'cmake version (\d+)\.(\d+)', Path(record['stdout']).read_text())
                    require(version and tuple(map(int, version.groups())) >= (3, 22), 'CMake>=3.22 required')
            execute('fetch-root', ['cargo', 'fetch', '--locked', '--target', 'aarch64-unknown-linux-gnu'], 1800)
            if args.group in ('ocr', 'lint'):
                execute('fetch-ocr', ['cargo', 'fetch', '--locked', '--manifest-path',
                                    'vendor/wireframe-ocr/Cargo.toml', '--target', 'aarch64-unknown-linux-gnu'], 1800)
        elif args.group == 'lint':
            execute('lint-root', ['cargo', 'clippy', '--locked', '--offline', '--all-targets', '--', '-D', 'warnings'])
            execute('lint-ocr', ['cargo', 'clippy', '--locked', '--offline', '--manifest-path',
                                'vendor/wireframe-ocr/Cargo.toml', '--all-targets', '--', '-D', 'warnings'])
        elif args.group == 'release':
            execute('optimized-build', ['cargo', 'build', '--locked', '--offline', '--release'])
            binary = target / 'release/wireframe'
            build = {'source_commit': binding['productCommit'], 'controller_commit': stage['controllerCommit'],
                     'profile': 'release', 'build_exit_code': 0, 'binary_sha256': sha(binary),
                     'argv': ['cargo', 'build', '--locked', '--offline', '--release'], 'environment': proof}
            (receipts / 'build-record.json').write_text(json.dumps(build, indent=2) + '\n')
            execute('package', ['python3', 'scripts/package.py'])
            for label, argv in [('static-file', ['file', str(binary)]), ('static-header', ['readelf', '-h', str(binary)]),
                                 ('static-loader', ['ldd', str(binary)]),
                                 ('static-versions', ['readelf', '--version-info', str(binary)]),
                                 ('static-needed', ['readelf', '-d', str(binary)])]:
                execute(label, argv, 120)
            package.elf_identity(binary.read_bytes())
            stage['abiVersions'] = native.abi_versions((receipts / 'static-versions.out').read_text())
            stage['neededLibraries'] = re.findall(r'\(NEEDED\).*?\[(.*?)\]', (receipts / 'static-needed.out').read_text())
            require(set(stage['neededLibraries']) <= {'libgcc_s.so.1', 'libm.so.6', 'libc.so.6', 'libstdc++.so.6'},
                    'new dynamic library needs review')
            require('not found' not in (receipts / 'static-loader.out').read_text(), 'container loader unresolved')
            (receipts / 'source-manifest.json').write_text(json.dumps(binding['sourceManifest'], indent=2) + '\n')
            (receipts / 'filesystem-modes.json').write_text(json.dumps({n: oct(modes[n]) for n in modes}) + '\n')
            (receipts / 'directory-modes.json').write_text(json.dumps(directories) + '\n')
            archive = source / 'dist' / (PACKAGE + '.tar.gz')
            checksum = archive.with_suffix('.gz.sha256')
            execute('verify-package', ['python3', str(P / 'verify_package.py'), '--source-commit', binding['productCommit'],
                '--repo', str(repo), '--frozen-source', str(source), '--source-manifest', str(receipts / 'source-manifest.json'),
                '--frozen-filesystem-modes', str(receipts / 'filesystem-modes.json'),
                '--frozen-directory-modes', str(receipts / 'directory-modes.json'),
                '--archive', str(archive), '--checksum', str(checksum), '--validated-binary', str(binary),
                '--build-record', str(receipts / 'build-record.json'), '--output', str(work / 'payload/package-verification')], 1800)
            report = json.loads((work / 'payload/package-verification/verification.json').read_text())
            require(report['corresponding_source_files_exact'] == binding['packagedSourceFiles']
                    and report['dependency_notice_records'] == binding['noticeRecords'], 'package completeness differs')
            require(binary.stat().st_size < 64 * 1024**2 and archive.stat().st_size < 64 * 1024**2, 'artifact cap')
            artifacts = work / 'artifacts'
            artifacts.mkdir(exist_ok=False)
            for path in [binary, archive, checksum, receipts / 'build-record.json', receipts / 'source-manifest.json',
                         receipts / 'ocr-source-verify.json', *(work / 'payload/package-verification').glob('*.json')]:
                shutil.copy2(path, artifacts / path.name)
            (artifacts / 'wireframe.sha256').write_text(sha(binary) + '  wireframe\n')
            stage['packageReportSHA256'] = sha(artifacts / 'verification.json')
            stage['binarySHA256'], stage['archiveSHA256'] = sha(binary), sha(archive)
        else:
            for index, harness in enumerate(binding['groups'][args.group]):
                stem = 'harness-' + str(index)
                argv = ['cargo', 'test', '--locked', '--offline', *harness['selector']]
                listed = execute(stem + '-list', argv + ['--', '--list'])
                names = account.catalog(Path(listed['stdout']).read_text(), harness['allowEmpty'])
                (receipts / (stem + '-catalog.json')).write_text(json.dumps({
                    'identity': harness['identity'], 'compiledNames': names, 'listCommand': listed,
                    'allowEmpty': harness['allowEmpty']}, indent=2) + '\n')
                ran = execute(stem + '-run', argv + ['--', '--test-threads=1'])
                accounted = account.result(Path(ran['stdout']).read_text(), names, harness['identity'], binding['allowedIgnored'])
                stage['harnesses'].append({'identity': harness['identity'], **accounted,
                    'catalogSHA256': sha(receipts / (stem + '-catalog.json')),
                    'rawListSHA256': sha(listed['stdout']), 'rawResultSHA256': sha(ran['stdout'])})
        if args.phase == 'verify':
            stage['generatedNativeInputs'] = native.generated_attestation(target, source)
        package.verify_frozen_source(source, contents, modes, directories)
        require(bind.snapshot(repo, binding['productCommit']) == binding['sourceManifest'], 'product input drift')
        stage.update(success=True, stableOwnedPayloadBytes=runtime.allocated(work / 'payload'),
                     actualNativeUIAcceptance=False, peakPayloadBytesUnknown=True)
    except BaseException as error:
        stage['failure'] = type(error).__name__ + ': ' + str(error)
        raise
    finally:
        with (receipts / (args.phase + '.json')).open('x') as stream:
            json.dump(stage, stream, indent=2)
            stream.write('\n')
        print(json.dumps({k: stage[k] for k in ['phase', 'group', 'productCommit', 'controllerCommit', 'success']}))


if __name__ == '__main__':
    main()

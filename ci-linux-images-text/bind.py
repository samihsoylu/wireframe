"""Read-only source inspection and explicit final reviewed-SHA binding; no Cargo."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import tomllib

from ci_common import require, sha

P = Path(__file__).resolve().parent


def read(name):
    return json.loads((P / name).read_text())


def git(repo, *args, input=None):
    return subprocess.check_output(['git', '-C', str(repo), *args], input=input,
                                   env={**os.environ, 'GIT_OPTIONAL_LOCKS': '0'})


def test_names(text):
    """Only critical-module literal tests, not an invented complete Rust parser.

    The authoritative complete test catalog comes from the actual compiled
    Linux harness. Macro-generated tests are captured there, including old101.
    """
    rows = re.findall(r'#\[test\]((?:(?!\bfn\b).){0,2048})\bfn\s+(\w+)\s*\(', text, re.S)
    return sorted(n for attrs, n in rows if '#[ignore' not in attrs)


def snapshot(repo, commit):
    require(re.fullmatch(r'[0-9a-f]{40}', commit) is not None, 'exact product commit required')
    require(git(repo, 'rev-parse', 'HEAD').decode().strip() == commit, 'checkout HEAD differs')
    require(not git(repo, 'status', '--porcelain', '--untracked-files=all'), 'product checkout dirty')
    entries = []
    for row in git(repo, 'ls-tree', '-rz', '--full-tree', commit).split(b'\0'):
        if not row:
            continue
        header, raw = row.split(b'\t', 1)
        mode, kind, oid = header.split()
        name = raw.decode()
        require(kind == b'blob' and mode in (b'100644', b'100755') and
                not name.startswith('/') and all(p not in ('', '.', '..') for p in name.split('/')),
                'unsafe/nonregular tracked input')
        entries.append((name, int(mode, 8) & 0o777, oid))
    require(1000 <= len(entries) <= 10000, 'source count outside review bound')
    batch = io.BytesIO(git(repo, 'cat-file', '--batch', input=b''.join(oid + b'\n' for _, _, oid in entries)))
    manifest = {}
    for name, mode, oid in entries:
        actual, kind, size = batch.readline().rstrip(b'\n').split()
        require(actual == oid and kind == b'blob' and int(size) <= 128 * 1024**2, 'Git blob bound/identity')
        data = batch.read(int(size))
        require(len(data) == int(size) and batch.read(1) == b'\n', 'Git blob truncated')
        path = repo / name
        require(path.is_file() and not path.is_symlink() and path.read_bytes() == data
                and stat.S_IMODE(path.stat().st_mode) == mode, 'worktree differs: ' + name)
        manifest[name] = {'mode': oct(mode), 'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()}
    require(not batch.read(), 'Git batch tail')
    return manifest


def build_binding(repo, commit, extra_required=()):
    manifest = snapshot(repo, commit)
    cargo = tomllib.loads((repo / 'Cargo.toml').read_text())
    require(cargo['package']['name'] == 'wireframe' and cargo['package']['version'] == '0.1.0-beta.3',
            'package/version requires controller review')
    require(not any(k in cargo for k in ['test', 'example', 'bench', 'workspace'])
            and cargo['package'].get('autotests', True) and cargo['package'].get('autoexamples', True),
            'custom Cargo target configuration requires explicit review')
    require(not any(n.startswith('benches/') and n.endswith('.rs') for n in manifest), 'new benches require review')
    require('src/lib.rs' in manifest and 'src/main.rs' in manifest, 'root unit targets missing')
    policy = read('policy.json')
    integrations = sorted(Path(n).stem for n in manifest if re.fullmatch(r'tests/[^/]+\.rs', n))
    examples = sorted(Path(n).stem for n in manifest if re.fullmatch(r'examples/[^/]+\.rs', n))
    require(len(integrations) >= 166 and len(examples) >= 53, 'full target coverage unexpectedly shrank')
    for name in policy['knownLinuxEmptyIntegrationTargets']:
        require('#![cfg(target_os = "macos")]' in (repo / 'tests' / (name + '.rs')).read_text(),
                'platform-excluded target cfg changed; review required')
    require(all(re.fullmatch(r'[a-zA-Z0-9_-]+', n) for n in integrations + examples), 'target name bound')
    require(not any(re.fullmatch(r'examples/[^/]+/main\.rs', n) for n in manifest),
            'new directory examples require reviewed discovery')
    groups = {'unit': [harness('lib:wireframe', ['--lib']), harness('bin:wireframe', ['--bin', 'wireframe'])]}
    for i in range(policy['integrationShards']):
        groups[f'integration-{i + 1:02}'] = [harness('test:' + n, ['--test', n],
            empty=n in policy['knownLinuxEmptyIntegrationTargets']) for n in integrations[i::policy['integrationShards']]]
    for i in range(policy['exampleShards']):
        groups[f'examples-{i + 1:02}'] = [harness('example:' + n, ['--example', n],
            empty=not test_names((repo / 'examples' / (n + '.rs')).read_text()))
            for n in examples[i::policy['exampleShards']]]
    groups['examples-01'].append(harness('doc:wireframe', ['--doc'], empty=True))
    ocr = ['--manifest-path', 'vendor/wireframe-ocr/Cargo.toml']
    require('vendor/wireframe-ocr/Cargo.lock' in manifest, 'locked OCR test package required')
    groups['ocr'] = [harness('ocr-lib:wireframe_ocr', ocr + ['--lib']),
                     harness('ocr-test:inference', ocr + ['--test', 'inference']),
                     harness('ocr-doc:wireframe_ocr', ocr + ['--doc'], empty=True)]
    groups['lint'], groups['release'] = [], []
    # Surface native OCR/inference and full unit failures early. Shards remain
    # independent and fail-fast is disabled, preserving the complete evidence.
    order = ['unit', 'ocr', 'lint', *[f'integration-{i + 1:02}' for i in range(policy['integrationShards'])],
             *[f'examples-{i + 1:02}' for i in range(policy['exampleShards'])], 'release']
    groups = {name: groups[name] for name in order}
    required = list(read('legacy101.json'))
    for spec in policy['criticalModules']:
        names = test_names((repo / spec['path']).read_text())
        require(names, 'critical module has no literal test names: ' + spec['path'])
        required += [spec['target'] + '::' + spec['module'] + '::' + n for n in names]
    for name in policy['criticalIntegrationTargets']:
        require(name in integrations, 'critical integration target missing')
        required += ['test:' + name + '::' + n for n in test_names((repo / 'tests' / (name + '.rs')).read_text())]
    for target, path, module in [('ocr-lib:wireframe_ocr', 'vendor/wireframe-ocr/src/lib.rs', 'tests::'),
                                 ('ocr-test:inference', 'vendor/wireframe-ocr/tests/inference.rs', '')]:
        required += [target + '::' + module + n for n in test_names((repo / path).read_text())]
    required += list(extra_required)
    require(all('::' in n and len(n) < 2048 for n in required), 'required identity format')
    notices = json.loads((repo / 'licenses/manifest.json').read_text())
    require(len(notices) == 399, 'notice inventory changed; review required')
    return {'armed': False, 'productCommit': commit, 'version': cargo['package']['version'],
            'sourceManifest': manifest, 'sourceFiles': len(manifest),
            'packagedSourceFiles': len(set(manifest) - {'.gitignore', '.gitattributes'}),
            'rootLockSHA256': sha(repo / 'Cargo.lock'),
            'ocrLockSHA256': sha(repo / 'vendor/wireframe-ocr/Cargo.lock'),
            'noticeRecords': len(notices), 'groups': groups,
            'requiredPassedIdentities': sorted(set(required)),
            'integrationTargets': integrations, 'exampleTargets': examples,
            'allowedIgnored': policy['allowedIgnored'],
            'platformExcludedTargets': policy['knownLinuxEmptyIntegrationTargets']}


def harness(identity, selector, empty=False):
    return {'identity': identity, 'selector': selector, 'allowEmpty': empty}


def assert_armed():
    binding = read('product.json')
    require(binding.get('armed') is True and re.fullmatch(r'[0-9a-f]{40}', binding.get('productCommit') or '')
            is not None and binding.get('reviewEvidenceSHA256'), 'final reviewed product binding is UNARMED')
    require(len(binding['sourceManifest']) == binding['sourceFiles'], 'source binding count differs')
    return binding


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=['preview', 'arm', 'assert-armed'])
    parser.add_argument('--repo', type=Path)
    parser.add_argument('--reviewed-product-commit')
    parser.add_argument('--review-evidence', type=Path)
    parser.add_argument('--require-test', action='append', default=[])
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    if args.mode == 'assert-armed':
        binding = assert_armed()
        if args.output:
            with args.output.open('a') as stream:
                stream.write('product=' + binding['productCommit'] + '\n')
                stream.write('matrix=' + json.dumps({'group': list(binding['groups'])}, separators=(',', ':')) + '\n')
        print(json.dumps({'armed': True, 'productCommit': binding['productCommit'], 'groups': len(binding['groups'])}))
        return
    require(args.repo and args.output, 'explicit repo/output required')
    commit = args.reviewed_product_commit or git(args.repo, 'rev-parse', 'HEAD').decode().strip()
    result = build_binding(args.repo.resolve(), commit, args.require_test)
    if args.mode == 'arm':
        require(args.reviewed_product_commit and args.review_evidence and args.review_evidence.is_file(),
                'parent final reviewed SHA and review evidence required before arming')
        require(args.output.resolve() == P / 'product.json', 'armed binding must be the controller product.json')
        review = json.loads(args.review_evidence.read_text())
        require(review.get('source') == commit and review.get('independentReports'),
                'parent source acceptance does not bind this exact product/review')
        result.update(armed=True, reviewEvidenceSHA256=sha(args.review_evidence))
    require(not args.output.exists() or args.output.resolve() == P / 'product.json', 'refuse existing unrelated output')
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    if args.mode == 'arm':
        manifest = {p.name: sha(p) for p in sorted(P.iterdir()) if p.is_file() and p.name != 'implementation-manifest.json'}
        (P / 'implementation-manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(json.dumps({'armed': result['armed'], 'productCommit': commit, 'sourceFiles': result['sourceFiles'],
                      'harnesses': sum(len(v) for v in result['groups'].values()), 'groups': len(result['groups']),
                      'requiredNamedPasses': len(result['requiredPassedIdentities'])}))


if __name__ == '__main__':
    main()

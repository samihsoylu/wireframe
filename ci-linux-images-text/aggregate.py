"""Require every planned harness, named outcome, critical feature and old101."""
import argparse
import json
import os
from pathlib import Path
from ci_common import require, sha
import bind
import account


def aggregate(binding, records, controller_commit, run_id, binding_sha):
    require(len(records) == len(binding['groups']), 'missing/extra job receipts')
    groups, passed, ignored, harnesses = set(), set(), set(), set()
    for record in records:
        group = record['group']
        require(group in binding['groups'] and group not in groups and record['success'], 'duplicate/failed/unknown job')
        groups.add(group)
        require(record['productCommit'] == binding['productCommit'] and record['controllerCommit'] == controller_commit
                and str(record['runId']) == str(run_id) and record['bindingSHA256'] == binding_sha,
                'receipt source/controller/run/binding differs')
        expected = {h['identity'] for h in binding['groups'][group]}
        actual = [h['identity'] for h in record['harnesses']]
        require(set(actual) == expected and len(actual) == len(expected), 'planned harness missing/duplicated')
        for harness in record['harnesses']:
            require(harness['identity'] not in harnesses, 'duplicate harness across groups')
            harnesses.add(harness['identity'])
            good, skip = set(harness['passed']), set(harness['ignored'])
            require(len(good) == harness['passedCount'] and len(skip) == harness['ignoredCount']
                    and not good & skip and harness['compiledCount'] == len(good) + len(skip), 'named receipt counts differ')
            require(not passed & good and not ignored & skip and not passed & skip and not ignored & good
                    and skip <= set(binding['allowedIgnored']), 'duplicate/unapproved outcome')
            passed |= good
            ignored |= skip
        if group == 'lint':
            require({'lint-root', 'lint-ocr'} <= {c['label'] for c in record['commands']}, 'strict lint scope missing')
        if group == 'release':
            require(record.get('packageReportSHA256') and record.get('binarySHA256') and record.get('archiveSHA256'),
                    'release/static package proof missing')
    required = set(binding['requiredPassedIdentities'])
    require(required <= passed, 'required feature/old101 functions absent: ' + repr(sorted(required - passed)[:12]))
    require(groups == set(binding['groups']), 'incomplete group partition')
    return {'success': True, 'productCommit': binding['productCommit'], 'controllerCommit': controller_commit,
            'runId': run_id, 'bindingSHA256': binding_sha, 'groups': sorted(groups),
            'harnesses': len(harnesses), 'passedCount': len(passed), 'ignoredCount': len(ignored),
            'passedIdentities': sorted(passed), 'ignoredIdentities': sorted(ignored),
            'requiredNamedPasses': len(required), 'old101Included': True,
            'platformExcludedTargets': binding['platformExcludedTargets'],
            'limits': 'Ordinary Linux suite and static package only; existing opt-in external/native cases accounted separately; no UI/platform release acceptance'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--received', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    binding = bind.assert_armed()
    paths = sorted(args.received.glob('*/verify.json'))
    require(len(paths) == len(binding['groups']) and all(p.stat().st_size <= 8 * 1024**2 for p in paths),
            'bounded expected receipt set missing')
    records = [json.loads(p.read_text()) for p in paths]
    for path, record in zip(paths, records):
        require(record['group'] in binding['groups'], 'unknown receipt group')
        for index, (spec, row) in enumerate(zip(binding['groups'][record['group']], record['harnesses'], strict=True)):
            prefix = path.parent / ('harness-' + str(index))
            listed, ran = Path(str(prefix) + '-list.out'), Path(str(prefix) + '-run.out')
            catalog = Path(str(prefix) + '-catalog.json')
            require(sha(listed) == row['rawListSHA256'] and sha(ran) == row['rawResultSHA256']
                    and sha(catalog) == row['catalogSHA256'], 'raw receipt digest mismatch')
            names = account.catalog(listed.read_text(), spec['allowEmpty'])
            compiled = json.loads(catalog.read_text())
            require(compiled['identity'] == spec['identity'] and compiled['compiledNames'] == names,
                    'stored compiled catalog differs from raw list')
            actual = account.result(ran.read_text(), names, spec['identity'], binding['allowedIgnored'])
            require(row['identity'] == spec['identity'] and all(actual[k] == row[k] for k in actual),
                    'aggregate raw evidence differs from reported outcome')
    result = aggregate(binding, records, os.environ['GITHUB_SHA'],
                       os.environ['GITHUB_RUN_ID'], sha(bind.P / 'product.json'))
    require(not args.output.exists(), 'aggregate output must be fresh')
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({k: result[k] for k in ['success', 'productCommit', 'harnesses', 'passedCount', 'ignoredCount']}))


if __name__ == '__main__':
    main()

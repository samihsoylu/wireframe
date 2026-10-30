"""Named compiled-list/result accounting for exactly one libtest harness."""
import re
from ci_common import require


def catalog(text, allow_empty=False):
    names = re.findall(r'^(.+): test$', text, re.M)
    summaries = re.findall(r'^(\d+) tests?, (\d+) benchmarks?$', text, re.M)
    require(len(summaries) == 1 and int(summaries[0][0]) == len(names)
            and int(summaries[0][1]) == 0 and len(names) == len(set(names)),
            'compiled test list/count differs')
    require(len(names) <= 20000 and all(0 < len(n) <= 2048 for n in names), 'test-name bound')
    require(bool(names) or allow_empty, 'unexpected empty test harness')
    return sorted(names)


def result(text, names, prefix, allowed_ignored):
    rows = re.findall(r'^test (.+?) \.\.\. (ok|FAILED|ignored(?:, [^\n]*)?)\s*$', text, re.M)
    summaries = re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', text, re.M)
    require(len(summaries) == 1, 'one successful harness summary required')
    passed, failed, ignored = map(int, summaries[0])
    actual_names = [r[0] for r in rows]
    require(len(actual_names) == len(set(actual_names)) and sorted(actual_names) == sorted(names),
            'executed functions differ from compiled list')
    require(failed == 0 and passed + ignored == len(names), 'harness pass/ignore count differs')
    successes, skipped = [], []
    for name, status in rows:
        identity = prefix + '::' + name
        if status == 'ok':
            successes.append(identity)
        else:
            require(status.startswith('ignored') and identity in allowed_ignored,
                    'new/failed/unapproved ignored check: ' + identity)
            skipped.append(identity)
    require(passed == len(successes) and ignored == len(skipped), 'named status/count differs')
    return {'passed': sorted(successes), 'ignored': sorted(skipped),
            'compiledCount': len(names), 'passedCount': passed, 'ignoredCount': ignored}

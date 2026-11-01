"""Pure source regression check: only service_closure executes, with closed fakes.

No candidate module imports, process modules, /proc reads, signals or fixtures.
Parent runs after review with explicit --adapter and --previous source paths.
"""
import argparse
import ast
import json
from pathlib import Path
from types import SimpleNamespace


class FakePath:
    def __init__(self, case, parts=()):
        self.case, self.parts = case, parts

    def __truediv__(self, value):
        return FakePath(self.case, self.parts + Path(value).parts)

    def lstat(self):
        assert self.parts == ('bus-supervisor-result.json',)
        self.case.events.append(('terminal-check',))
        if self.case.terminal:
            return SimpleNamespace()
        raise FileNotFoundError('closed fake: no terminal result')


class Case:
    def __init__(self, *, terminal=False, supervisor_state='live', absent=False,
                 publish_after_first=False, publish_during_check=False):
        self.terminal = terminal
        self.supervisor_state = supervisor_state
        self.absent = absent
        self.publish_after_first = publish_after_first
        self.publish_during_check = publish_during_check
        self.events, self.signals = [], []
        self.supervisor = {'kind': 'supervisor', 'pid': 10, 'startTicks': 'original-reaper'}
        self.portals = {name: {'identity': {'kind': name, 'pid': pid, 'startTicks': name}}
                        for name, pid in [('frontend', 20), ('backend', 21)]}

    def read(self, path):
        assert path.parts == ('observations', 'portal-owners.json')
        return self.portals

    def closed(self, incomplete):
        assert incomplete is True
        self.events.append(('apps-closed',))

    def absent_pid(self, pid):
        assert pid in (20, 21)
        if not self.absent:
            raise RuntimeError('closed fake: portal still present')

    def live_identity(self, row):
        if row['kind'] == 'supervisor':
            assert row == self.supervisor
            self.events.append(('supervisor-live',))
            if self.supervisor_state == 'dead':
                raise FileNotFoundError('closed fake: original supervisor dead')
            if self.supervisor_state == 'drift':
                raise RuntimeError('closed fake: original supervisor identity drift')
            if self.publish_during_check:
                self.terminal = True
        else:
            assert row == self.portals[row['kind']]['identity']
            self.events.append(('portal-live', row['pid']))

    def kill(self, pid, sig):
        assert pid in (20, 21) and sig == 15  # Record only; never invokes a real signal.
        self.events.append(('kill', pid, sig))
        self.signals.append((pid, sig))
        if self.publish_after_first and len(self.signals) == 1:
            self.terminal = True

    def bindings(self):
        return {'ROOT': FakePath(self), 'signal': SimpleNamespace(SIGTERM=15),
                'os': SimpleNamespace(kill=self.kill),
                'component_pins': lambda incomplete: {'desktop': self.supervisor},
                'bus_child_closure': lambda: self.events.append(('stored-child-closure',)),
                'bus_supervisor_closure': lambda: self.events.append(('strict-supervisor-closure',)),
                'b': SimpleNamespace(read=self.read, all_started_sessions_closed=self.closed,
                                     prove_pid_absent=self.absent_pid, live_identity=self.live_identity)}


def invoke(source, case, stop=True):
    tree = ast.parse(source)
    node = next(n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == 'service_closure')
    assert not node.decorator_list and all(isinstance(n, ast.Constant) for n in node.args.defaults)
    assert not any(isinstance(n, (ast.Import, ast.ImportFrom, ast.Global, ast.Nonlocal))
                   for n in ast.walk(node))
    scope = case.bindings()
    scope['__builtins__'] = {'RuntimeError': RuntimeError, 'FileNotFoundError': FileNotFoundError}
    # Only this actual function definition is compiled; all names it can use are the closed fakes above.
    exec(compile(ast.Module(body=[node], type_ignores=[]), '<mocked-service-closure>', 'exec'), scope)
    try:
        return scope['service_closure'](stop), None
    except (RuntimeError, FileNotFoundError) as exc:
        return None, type(exc).__name__ + ': ' + str(exc)


def check(adapter, previous):
    current, old = Path(adapter).read_text(), Path(previous).read_text()
    passed = []
    case = Case(terminal=True)
    _, error = invoke(old, case)
    assert error is None and case.signals == [(20, 15), (21, 15)]
    assert ('supervisor-live',) not in case.events
    passed.append('old missing-guard regression reproduced with fake signals only')

    for state in ['terminal', 'dead', 'drift']:
        case = Case(terminal=state == 'terminal', supervisor_state=state if state != 'terminal' else 'live')
        _, error = invoke(current, case)
        assert error is not None and case.signals == [], state
        passed.append(state + ' supervisor refuses every signal')

    case = Case()
    result, error = invoke(current, case)
    assert error is None and case.signals == [(20, 15), (21, 15)]
    assert set(result.values()) == {'owned SIGTERM requested; absence still required'}
    for index, event in enumerate(case.events):
        if event[0] == 'kill':
            assert case.events[index-3:index] == [('portal-live', event[1]),
                                                  ('supervisor-live',), ('terminal-check',)]
    passed.append('live original supervisor checked immediately before each owned SIGTERM')

    case = Case(terminal=True, supervisor_state='dead', absent=True)
    result, error = invoke(current, case)
    assert error is None and result == {'frontend': 'absent', 'backend': 'absent'}
    assert case.signals == [] and ('supervisor-live',) not in case.events
    passed.append('already absent portals remain absent without any new signal')

    case = Case(publish_after_first=True)
    _, error = invoke(current, case)
    assert error is not None and case.signals == [(20, 15)]
    passed.append('terminal publication between portals refuses the second signal')

    case = Case(publish_during_check=True)
    _, error = invoke(current, case)
    assert error is not None and case.signals == []
    passed.append('terminal publication during live check refuses signal')

    case = Case()
    _, error = invoke(current, case, stop=False)
    assert error is not None and case.signals == []
    assert ('strict-supervisor-closure',) in case.events
    passed.append('read-only strict service absence still fails for a present portal')
    return {'pureCasesPassed': len(passed), 'cases': passed, 'realSignalsSent': 0,
            'candidateModuleImported': False, 'processOrNativeCalls': 0}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--adapter', type=Path, required=True)
    parser.add_argument('--previous', type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(check(args.adapter, args.previous), indent=2))

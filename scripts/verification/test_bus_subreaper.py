"""Opt-in Linux process tests. Never run as part of source-only verification.

Requires a dedicated UID/GID65534 Linux Python process and explicit
WIREFRAME_BUS_SUBREAPER_TEST_AUTHORIZED=1. No Docker/VM/tools or product launch.
All fixture descendants self-exit; tests send no cleanup signals.
"""
import importlib.util
import json
import os
from pathlib import Path
import signal
import shutil
import stat
import subprocess
import sys
import tempfile
import time
import unittest

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('owned_bus_subreaper', HERE / 'bus_subreaper.py')
supervisor = importlib.util.module_from_spec(spec)
spec.loader.exec_module(supervisor)
STUDY = 'a' * 32
AUTHORIZED = (sys.platform == 'linux' and os.getuid() == os.getgid() == 65534 and
              os.environ.get('WIREFRAME_BUS_SUBREAPER_TEST_AUTHORIZED') == '1')


def emit(path, value):
    with path.open('x') as output:
        json.dump(value, output)
        output.flush()
        os.fsync(output.fileno())


def wait_file(path, timeout=3):
    end = time.monotonic() + timeout
    while not path.exists():
        if time.monotonic() >= end:
            raise TimeoutError(str(path))
        time.sleep(0.01)
    return json.loads(path.read_text())


def real_bus_hold_fixture(root):
    """A real session holder: observe only, then return0 on the original stop request."""
    holder=supervisor.process_identity(os.getpid())
    bus=supervisor.process_identity(holder['ppid'])
    end=time.monotonic()+2
    while True:
        daemons=[]
        for path in Path('/proc').iterdir():
            if not path.name.isdigit():
                continue
            try:
                fields={line.split(':',1)[0]:line.split(':',1)[1].split()
                        for line in (path/'status').read_text().splitlines() if ':' in line}
                if int(fields['PPid'][0])!=bus['pid'] or fields['Uid']!=['65534']*4:
                    continue
                executable=(path/'exe').resolve(strict=True)
                if executable.name!='dbus-daemon':
                    continue
                identity=supervisor.process_identity(int(path.name))
                if '--session' not in identity['argv']:
                    continue
                daemons.append({'identity':identity,'executable':str(executable),
                                'executableSHA256':supervisor.hashlib.sha256((path/'exe').read_bytes()).hexdigest()})
            except (FileNotFoundError,ProcessLookupError):
                continue
        if len(daemons)==1:
            break
        if len(daemons)>1:
            raise RuntimeError('ambiguous daemon under the actual dbus-run-session child')
        if time.monotonic()>=end:
            raise TimeoutError('no actual session daemon under the bus child')
        time.sleep(0.01)
    daemon=daemons[0]
    for identity in [holder,bus,daemon['identity']]:
        if identity['uids']!=[65534]*4 or identity['gids']!=[65534]*4 or identity['state']=='Z':
            raise RuntimeError('live same-UID/GID bus hierarchy required')
    if holder['ppid']!=bus['pid'] or daemon['identity']['ppid']!=bus['pid']:
        raise RuntimeError('holder/daemon outside the actual direct-child hierarchy')
    # Recheck all three original start ticks/argv/UIDs and parents before retaining live evidence.
    for identity in [holder,bus,daemon['identity']]:
        again=supervisor.process_identity(identity['pid'])
        for key in ['pid','ppid','startTicks','uids','gids','argv','argvSHA256']:
            if again[key]!=identity[key]:
                raise RuntimeError('live bus hierarchy changed before recording')
    address=os.environ.get('DBUS_SESSION_BUS_ADDRESS','')
    if not address.startswith('unix:'):
        raise RuntimeError('dbus-run-session did not provide a private Unix bus')
    if (root/'bus-stop-request.json').exists() or (root/'bus-stop-request.json').is_symlink():
        raise RuntimeError('stop requested before live ancestry proof')
    emit(root/'real-bus-ancestry.json',{'holder':holder,'busChild':bus,'daemon':daemon,
         'privateBusAddressSHA256':supervisor.hashlib.sha256(address.encode()).hexdigest(),
         'liveBeforeStopRequest':True})
    request=wait_file(root/'bus-stop-request.json',timeout=4)
    if request!={'studyRunID':STUDY,'request':'normal-owned-bus-stop'}:
        raise ValueError('original stop-request protocol mismatch')
    emit(root/'real-bus-holder-return.json',{'pid':holder['pid'],'startTicks':holder['startTicks'],
         'ordinaryStopRequest':request,'returnCode':0,'rawHolderWaitClaimed':False})
    return 0


def fixture(mode, directory):
    if not AUTHORIZED:
        raise RuntimeError('fixture process requires explicit Linux UID65534 authorization')
    root = Path(directory)
    if mode == 'real-bus-hold':
        return real_bus_hold_fixture(root)
    if mode == 'timeout':
        emit(root / 'fixture-live.json', {'pid': os.getpid()})
        time.sleep(0.8)  # Finite self-exit, even after its supervisor times out.
        return 0
    if mode in ['exit7', 'signal']:
        emit(root / 'fixture-live.json', {'pid': os.getpid()})
        wait_file(root / 'bus-stop-request.json')
        if mode == 'signal':
            signal.raise_signal(signal.SIGTERM)  # This fixture's own signal only.
        return 7
    if mode != 'double-fork':
        raise ValueError(mode)
    expected_reaper = os.getppid()
    intermediate = os.fork()
    if intermediate == 0:
        descendant = os.fork()
        if descendant == 0:
            end = time.monotonic() + 3
            while os.getppid() != expected_reaper:
                if time.monotonic() >= end:
                    os._exit(90)
                time.sleep(0.01)
            emit(root / 'adopted.json', supervisor.process_identity(os.getpid()))
            try:
                wait_file(root / 'release-descendant.json')
            except TimeoutError:
                os._exit(91)
            os._exit(23)
        os._exit(11)
    pid, status = os.waitpid(intermediate, 0)
    emit(root / 'intermediate-wait.json', {'pid': pid, 'rawWaitStatus': status})
    wait_file(root / 'bus-stop-request.json')
    return 0


@unittest.skipUnless(AUTHORIZED, 'requires explicit dedicated Linux UID/GID65534 test authorization')
class BusSubreaperTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        supplied=os.environ.get('WIREFRAME_BUS_SUBREAPER_TEST_ARTIFACTS')
        if not supplied:
            raise RuntimeError('WIREFRAME_BUS_SUBREAPER_TEST_ARTIFACTS is required')
        cls.artifacts=Path(supplied)
        if not cls.artifacts.is_absolute() or cls.artifacts.resolve(strict=True)!=cls.artifacts:
            raise RuntimeError('canonical absolute test artifact directory required')
        row=cls.artifacts.lstat()
        if not stat.S_ISDIR(row.st_mode) or (row.st_uid,row.st_gid,stat.S_IMODE(row.st_mode))!=(65534,65534,0o700):
            raise RuntimeError('test artifact directory must be same UID/GID65534 mode0700')
        # Adopt only these finite fixtures if a timeout makes the inner reaper exit.
        supervisor.enable_subreaper()
        try:
            os.waitid(os.P_ALL, 0, os.WEXITED | os.WNOHANG | os.WNOWAIT)
        except ChildProcessError:
            return
        raise RuntimeError('dedicated test runner must start without existing children')

    def setUp(self):
        self.root=Path(tempfile.mkdtemp(prefix=self._testMethodName+'-',dir=self.artifacts))
        emit(self.root/'test-description.json',{'test':self.id(),'syntheticFixtureOnly':True,'studyRunID':STUDY})
        self.process = None

    def tearDown(self):
        # No kill/terminate calls: every fixture self-exits within its fixed bound.
        if self.process is not None and self.process.returncode is None:
            self.process.wait(timeout=5)
        end = time.monotonic() + 5
        extra_waits=[]
        while True:
            try:
                event = os.waitid(os.P_ALL, 0, os.WEXITED | os.WNOHANG | os.WNOWAIT)
            except ChildProcessError:
                break
            if event is not None:
                pid,raw=os.waitpid(event.si_pid,0)
                extra_waits.append({'pid':pid,'rawWaitStatus':raw})
                continue
            if time.monotonic() >= end:
                self.fail('finite owned fixture remained alive; no signal sent')
            time.sleep(0.01)
        emit(self.root/'test-parent-remaining-reaps.json',extra_waits)
        # Retain every synthetic record for parent archive and independent review.

    def start(self, mode, timeout=1.5, missing_exec=False):
        command = ([str(self.root / 'absent-executable')] if missing_exec else
                   [sys.executable, '-I', '-B', str(Path(__file__).resolve()),
                    '--fixture', mode, str(self.root)])
        argv = [sys.executable, '-I', '-B', str(Path(__file__).resolve()),
                '--supervisor', str(self.root), str(timeout)] + command
        self.process = subprocess.Popen(argv)
        return wait_file(self.root / 'bus-supervisor-launch.json')

    def stop_request(self):
        emit(self.root / 'bus-stop-request.json',
             {'studyRunID': STUDY, 'request': 'normal-owned-bus-stop'})

    def result(self):
        self.process.wait(timeout=5)  # Timeout does not automatically signal the process.
        result = wait_file(self.root / 'bus-supervisor-result.json')
        emit(self.root/'test-supervisor-observed-exit.json',{'method':'subprocess.Popen.wait','returncode':self.process.returncode})
        log = (self.root / 'bus-supervisor-waits.jsonl').read_bytes()
        self.assertEqual(supervisor.hashlib.sha256(log).hexdigest(), result['waitLogSHA256'])
        self.assertEqual(result['signalsSent'], [])
        return result

    def test_double_fork_is_adopted_reaped_and_pid_absent(self):
        launch = self.start('double-fork')
        adopted = wait_file(self.root / 'adopted.json')
        self.assertEqual(adopted['ppid'], launch['supervisor']['pid'])
        self.assertEqual(adopted['uids'], [65534] * 4)
        emit(self.root / 'release-descendant.json', {'release': True})
        self.stop_request()
        result = self.result()
        self.assertEqual(self.process.returncode, 0)
        self.assertTrue(result['allChildrenReaped'])
        self.assertFalse(result['timedOut'])
        self.assertEqual(result['errors'], [])
        row = next(r for r in result['waits'] if r['pid'] == adopted['pid'])
        self.assertEqual(row['identity']['startTicks'], adopted['startTicks'])
        self.assertEqual(row['identity']['state'], 'Z')  # Z was observed before wait, never treated as absent.
        self.assertEqual(row['exitCode'], 23)
        self.assertTrue(os.WIFEXITED(row['rawWaitStatus']))
        self.assertEqual(os.WEXITSTATUS(row['rawWaitStatus']), 23)
        self.assertFalse(result['allChildExitCodesClaimedZero'])
        for row in result['waits']:
            self.assertFalse((Path('/proc') / str(row['pid'])).exists())
        self.assertFalse((Path('/proc') / str(launch['supervisor']['pid'])).exists())

    def test_original_bus_child_nonzero_is_preserved(self):
        self.start('exit7')
        wait_file(self.root / 'fixture-live.json')
        self.stop_request()
        result = self.result()
        self.assertEqual(self.process.returncode, 7)
        self.assertEqual(result['directChildWait']['exitCode'], 7)
        self.assertEqual(os.WEXITSTATUS(result['directChildWait']['rawWaitStatus']), 7)
        self.assertTrue(result['allChildrenReaped'])

    def test_signal_is_raw_and_outer_encoding_is_explicit(self):
        self.start('signal')
        wait_file(self.root / 'fixture-live.json')
        self.stop_request()
        result = self.result()
        self.assertEqual(self.process.returncode, 128 + signal.SIGTERM)
        self.assertEqual(result['directChildWait']['exitCode'], -signal.SIGTERM)
        self.assertEqual(result['directChildWait']['signal'], signal.SIGTERM)
        self.assertTrue(os.WIFSIGNALED(result['directChildWait']['rawWaitStatus']))

    def test_deadline_is_nonzero_and_does_not_claim_absence(self):
        launch = self.start('timeout', timeout=0.1)
        live = wait_file(self.root / 'fixture-live.json')
        self.stop_request()
        result = self.result()
        self.assertEqual(self.process.returncode, 124)
        self.assertTrue(result['timedOut'])
        self.assertFalse(result['allChildrenReaped'])
        self.assertIsNone(result['directChildWait'])
        self.assertIn(live['pid'], [r['pid'] for r in result['remainingDirectChildren']])
        # Outer test subreaper now owns the finite child and consumes its real wait.
        pid, raw = os.waitpid(live['pid'], 0)
        emit(self.root/'timeout-child-parent-wait.json',{'pid':pid,'rawWaitStatus':raw,'consumedByExplicitTestParent':True})
        self.assertEqual(pid, launch['directChild']['pid'])
        self.assertTrue(os.WIFEXITED(raw))
        self.assertFalse((Path('/proc') / str(pid)).exists())

    def test_exec_failure_retains_127_and_error(self):
        self.start('unused', missing_exec=True)
        result = self.result()
        self.assertEqual(self.process.returncode, 127)
        self.assertEqual(result['directChildWait']['exitCode'], 127)
        self.assertTrue(result['allChildrenReaped'])
        self.assertTrue(any(e.startswith('exec: ') for e in result['errors']))

    def test_real_dbus_session_daemon_is_adopted_reaped_and_absent(self):
        installed=shutil.which('dbus-run-session')
        self.assertIsNotNone(installed,'installed dbus-run-session is required; no install or skip-PASS')
        executable=Path(installed).resolve(strict=True)
        holder_argv=[sys.executable,'-I','-B',str(Path(__file__).resolve()),
                     '--fixture','real-bus-hold',str(self.root)]
        command=[str(executable),'--']+holder_argv
        emit(self.root/'real-bus-command.json',{'argv':command,'executable':str(executable),
             'executableSHA256':supervisor.hashlib.sha256(executable.read_bytes()).hexdigest()})
        argv=[sys.executable,'-I','-B',str(Path(__file__).resolve()),
              '--supervisor',str(self.root),'2.0']+command
        self.process=subprocess.Popen(argv)
        try:
            launch=wait_file(self.root/'bus-supervisor-launch.json')
            ancestry=wait_file(self.root/'real-bus-ancestry.json')
            bus=ancestry['busChild'];holder=ancestry['holder'];daemon=ancestry['daemon']['identity']
            self.assertTrue(ancestry['liveBeforeStopRequest'])
            self.assertEqual(bus['pid'],launch['directChild']['pid'])
            self.assertEqual(bus['startTicks'],launch['directChild']['startTicks'])
            self.assertEqual(bus['ppid'],launch['supervisor']['pid'])
            self.assertEqual(bus['argv'],command)
            self.assertEqual(holder['argv'],holder_argv)
            self.assertEqual(holder['ppid'],bus['pid'])
            self.assertEqual(daemon['ppid'],bus['pid'])
            self.assertIn('--session',daemon['argv'])
            for identity in [bus,holder,daemon]:
                self.assertEqual(identity['uids'],[65534]*4)
                self.assertEqual(identity['gids'],[65534]*4)
                self.assertNotEqual(identity['state'],'Z')
        finally:
            # Ordinary request only, including if a live-evidence assertion fails.
            if not (self.root/'bus-stop-request.json').exists():
                self.stop_request()
        result=self.result()
        self.assertEqual(self.process.returncode,0)
        self.assertTrue(result['allChildrenReaped'])
        self.assertFalse(result['timedOut'])
        self.assertEqual(result['errors'],[])
        direct=result['directChildWait']
        returned=wait_file(self.root/'real-bus-holder-return.json')
        self.assertEqual(returned['pid'],holder['pid'])
        self.assertEqual(returned['startTicks'],holder['startTicks'])
        self.assertEqual(returned['returnCode'],0)
        self.assertFalse(returned['rawHolderWaitClaimed'])
        self.assertEqual(direct['pid'],bus['pid'])
        self.assertEqual(direct['identity']['startTicks'],bus['startTicks'])
        self.assertTrue(direct['reaped'])
        self.assertEqual(direct['identity']['ppid'],launch['supervisor']['pid'])
        self.assertEqual(direct['identity']['uids'],[65534]*4)
        self.assertEqual(direct['identity']['gids'],[65534]*4)
        self.assertEqual(direct['waitidUID'],65534)
        self.assertEqual(direct['rawWaitStatus'],0)
        self.assertEqual(direct['exitCode'],0)
        self.assertIsNone(direct['signal'])
        adopted=next(row for row in result['waits'] if row['pid']==daemon['pid'])
        self.assertFalse(adopted['directChild'])
        self.assertEqual(adopted['identity']['startTicks'],daemon['startTicks'])
        self.assertEqual(adopted['identity']['ppid'],launch['supervisor']['pid'])
        self.assertEqual(adopted['identity']['uids'],[65534]*4)
        self.assertEqual(adopted['identity']['gids'],[65534]*4)
        self.assertEqual(adopted['identity']['state'],'Z')
        self.assertEqual(adopted['waitidUID'],65534)
        self.assertTrue(adopted['reaped'])
        self.assertIsNone(adopted['identityError'])
        # Keep the actual daemon exit/signal status; normal bus shutdown need not return daemon0.
        self.assertEqual(adopted['exitCode'],os.waitstatus_to_exitcode(adopted['rawWaitStatus']))
        emit(self.root/'real-bus-terminal-observations.json',{'busChildWait':direct,
             'adoptedDaemonWait':adopted,'holderReturnWitness':returned,'holderAbsent':not (Path('/proc')/str(holder['pid'])).exists(),
             'allChildrenReaped':result['allChildrenReaped'],'remainingDirectChildren':result['remainingDirectChildren']})
        for identity in [holder,bus,daemon,launch['supervisor']]:
            self.assertFalse((Path('/proc')/str(identity['pid'])).exists())
        self.assertEqual(result['remainingDirectChildren'],[])

    def test_same_uid_failure_happens_before_fork_or_records(self):
        before={p.name:p.read_bytes() for p in self.root.iterdir()}
        with self.assertRaises(ValueError):
            supervisor.supervise([sys.executable], self.root, study_run_id=STUDY,
                                 expected_uid=65533, expected_gid=65534)
        self.assertEqual({p.name:p.read_bytes() for p in self.root.iterdir()},before)
        self.assertFalse(any(p.name.startswith('bus-supervisor-') for p in self.root.iterdir()))


if __name__ == '__main__':
    os.umask(0o077)
    if len(sys.argv) > 1 and sys.argv[1] == '--fixture':
        sys.exit(fixture(sys.argv[2], sys.argv[3]))
    if len(sys.argv) > 1 and sys.argv[1] == '--supervisor':
        if not AUTHORIZED:
            raise RuntimeError('explicit Linux fixture authorization required')
        result = supervisor.supervise(sys.argv[4:], Path(sys.argv[2]), study_run_id=STUDY,
                                      expected_uid=65534, expected_gid=65534,
                                      shutdown_timeout=float(sys.argv[3]))
        sys.exit(result['supervisorExitCode'])
    unittest.main()

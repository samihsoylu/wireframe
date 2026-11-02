"""Opt-in finite credential fixtures for the core library, never the production CLI.

Requires a dedicated isolated Linux root process, explicit authorization and an
existing root-owned0700 artifact directory. No installs, setuid executables,
account changes, cleanup signals or artifact removal. Every fixture self-exits
within four seconds if its ordinary stop request never arrives. A pre-opened
append descriptor carries evidence from children that cannot traverse Root0700.
"""
import importlib.util
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import tempfile
import time
import unittest

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('credential_fixture_core', HERE / 'bus_subreaper.py')
core = importlib.util.module_from_spec(spec)
spec.loader.exec_module(core)
STUDY = 'c' * 32
SHORT_DEADLINE = 0.25
SURVIVAL_WINDOW = 0.75
FIXTURE_LIFETIME = 4.0
MODES = ('adopted-effective-uid', 'adopted-saved-uid',
         'adopted-effective-gid', 'adopted-saved-gid',
         'adopted-real-uid', 'adopted-real-gid',
         'direct-effective-uid', 'direct-saved-uid',
         'direct-effective-gid', 'direct-saved-gid')
AUTHORIZED = (sys.platform == 'linux' and os.getresuid() == (0, 0, 0) and
              os.getresgid() == (0, 0, 0) and
              os.environ.get('WIREFRAME_BUS_SUBREAPER_CREDENTIAL_TEST_AUTHORIZED') == '1')


def emit(path, value):
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    try:
        core.write_all(fd, (json.dumps(value, sort_keys=True) + '\n').encode())
        os.fsync(fd)
    finally:
        os.close(fd)


def marker(fd, kind, **facts):
    # One small O_APPEND write; no path lookup after a child's credential drop.
    raw = (json.dumps({'kind': kind, **facts}, sort_keys=True) + '\n').encode()
    if len(raw) > 4096 or os.write(fd, raw) != len(raw):
        raise RuntimeError('bounded fixture marker write failed')
    os.fsync(fd)


def apply_credentials(mode):
    # Called only in the owned, finite fixture child, never in the supervisor.
    change = mode.split('-', 1)[1]
    if change == 'effective-uid':
        os.setresuid(0, 65534, 0)
    elif change == 'saved-uid':
        os.setresuid(0, 0, 65534)
    elif change == 'effective-gid':
        os.setresgid(0, 65534, 0)
    elif change == 'saved-gid':
        os.setresgid(0, 0, 65534)
    elif change == 'real-uid':
        os.setresuid(65534, 65534, 65534)
    elif change == 'real-gid':
        os.setresgid(65534, 65534, 65534)
    else:
        raise ValueError(mode)


def core_identity(row):
    return (row.st_dev, row.st_ino, stat.S_IFMT(row.st_mode))


def fixture(mode, root, marker_fd):
    if not AUTHORIZED or mode not in MODES:
        raise RuntimeError('explicit isolated Linux root fixture authorization required')
    # The only inherited evidence descriptor must identify the private regular file.
    state = root.lstat()
    opened = os.fstat(marker_fd)
    expected = (root / 'fixture-events.jsonl').lstat()
    if (core_identity(opened) != core_identity(expected) or not stat.S_ISREG(expected.st_mode)):
        raise RuntimeError('fixture marker descriptor does not match its retained file')
    if not (stat.S_ISDIR(state.st_mode) and stat.S_IMODE(state.st_mode) == 0o700 and
            state.st_uid == state.st_gid == 0 and stat.S_ISREG(opened.st_mode) and
            opened.st_uid == opened.st_gid == 0 and stat.S_IMODE(opened.st_mode) == 0o600 and
            opened.st_nlink == 1 and os.get_inheritable(marker_fd)):
        raise RuntimeError('private fixture descriptor/state boundary')
    reaper = os.getppid()
    marker(marker_fd, 'direct-start', identity=core.process_identity(os.getpid()))
    if mode.startswith('direct-'):
        apply_credentials(mode)
        marker(marker_fd, 'variant-live', identity=core.process_identity(os.getpid()))
        time.sleep(0.05)
        return 23
    intermediate = os.fork()
    if intermediate == 0:
        try:
            adopted = os.fork()
            if adopted == 0:
                limit = time.monotonic() + 1.0
                while os.getppid() != reaper:
                    if time.monotonic() >= limit:
                        os._exit(90)
                    time.sleep(0.01)
                apply_credentials(mode)
                marker(marker_fd, 'variant-live', identity=core.process_identity(os.getpid()))
                time.sleep(0.05)
                os._exit(23)
            os._exit(11)
        except BaseException:
            os._exit(92)
    pid, raw = os.waitpid(intermediate, 0)
    marker(marker_fd, 'intermediate-wait', pid=pid, rawWaitStatus=raw)
    end = time.monotonic() + FIXTURE_LIFETIME
    while time.monotonic() < end:
        request = root / 'bus-stop-request.json'
        try:
            value = json.loads(request.read_text())
        except FileNotFoundError:
            time.sleep(0.01)
            continue
        if value != {'studyRunID': STUDY, 'request': 'normal-owned-bus-stop'}:
            return 93
        marker(marker_fd, 'direct-ordinary-return', pid=os.getpid(), returnCode=0)
        return 0
    # No parent cleanup signal is needed even if supervision/tests fail.
    marker(marker_fd, 'direct-finite-timeout', pid=os.getpid(), returnCode=91)
    return 91


@unittest.skipUnless(AUTHORIZED, 'requires explicit isolated Linux root credential-fixture authorization')
class BusSubreaperCredentialTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        supplied = os.environ.get('WIREFRAME_BUS_SUBREAPER_CREDENTIAL_TEST_ARTIFACTS')
        if not supplied:
            raise RuntimeError('explicit retained artifact directory required')
        cls.artifacts = Path(supplied)
        if not cls.artifacts.is_absolute() or cls.artifacts.resolve(strict=True) != cls.artifacts:
            raise RuntimeError('canonical absolute artifact directory required')
        row = cls.artifacts.lstat()
        if not stat.S_ISDIR(row.st_mode) or (row.st_uid, row.st_gid, stat.S_IMODE(row.st_mode)) != (0, 0, 0o700):
            raise RuntimeError('artifact directory must be root-owned0700')
        core.enable_subreaper()  # Retain finite fixtures if an inner supervisor fails.
        try:
            os.waitid(os.P_ALL, 0, os.WEXITED | os.WNOHANG | os.WNOWAIT)
        except ChildProcessError:
            return
        raise RuntimeError('dedicated test runner must start without existing children')

    def setUp(self):
        self.root = Path(tempfile.mkdtemp(prefix=self._testMethodName + '-', dir=self.artifacts))
        self.process = None
        emit(self.root / 'test-description.json', {'test': self.id(), 'studyRunID': STUDY,
             'syntheticRootLibraryFixtureOnly': True, 'coreSHA256': core.hashlib.sha256(
                 (HERE / 'bus_subreaper.py').read_bytes()).hexdigest()})

    def tearDown(self):
        # No kill/terminate calls. Original fixture hierarchy has a finite self-exit.
        if self.process is not None and self.process.returncode is None:
            self.process.wait(timeout=8)
        end = time.monotonic() + 8
        extra = []
        while True:
            try:
                event = os.waitid(os.P_ALL, 0, os.WEXITED | os.WNOHANG | os.WNOWAIT)
            except ChildProcessError:
                break
            if event is not None:
                pid, raw = os.waitpid(event.si_pid, 0)
                extra.append({'pid': pid, 'rawWaitStatus': raw,
                              'identity': None, 'fixtureParentNaturalWaitOnly': True})
                continue
            if time.monotonic() >= end:
                self.fail('finite fixture failed to self-exit; no signal sent')
            time.sleep(0.01)
        emit(self.root / 'test-parent-remaining-reaps.json', extra)
        self.assertEqual(stat.S_IMODE(self.root.stat().st_mode), 0o700)

    def wait_until(self, predicate, seconds=3):
        end = time.monotonic() + seconds
        while time.monotonic() < end:
            value = predicate()
            if value:
                return value
            time.sleep(0.01)
        self.fail('bounded fixture observation missing')

    def rows(self, name):
        try:
            raw = (self.root / name).read_bytes()
        except FileNotFoundError:
            return []
        # Readers may see a writer in progress; only complete newline records count.
        return [json.loads(row) for row in raw.splitlines(keepends=True) if row.endswith(b'\n')]

    def event(self, kind):
        return self.wait_until(lambda: next((row for row in self.rows('fixture-events.jsonl')
                                            if row['kind'] == kind), None))

    def start(self, mode, timeout=SHORT_DEADLINE):
        fd = os.open(self.root / 'fixture-events.jsonl', os.O_WRONLY | os.O_CREAT |
                     os.O_EXCL | os.O_APPEND | os.O_NOFOLLOW, 0o600)
        output = os.open(self.root / 'supervisor-output.txt', os.O_WRONLY | os.O_CREAT |
                         os.O_EXCL | os.O_NOFOLLOW, 0o600)
        try:
            argv = [sys.executable, '-I', '-B', str(Path(__file__).resolve()),
                    '--supervisor', str(self.root), str(timeout), mode, str(fd)]
            emit(self.root / 'test-supervisor-intent.json', {'argv': argv,
                 'inheritedMarkerFD': fd, 'markerMode': 'pre-opened private O_APPEND descriptor',
                 'expectedUID': 0, 'expectedGID': 0})
            self.process = subprocess.Popen(argv, pass_fds=(fd,), stdout=output, stderr=subprocess.STDOUT)
        finally:
            os.close(fd)
            os.close(output)
        return self.wait_until(lambda: next(iter(self.rows('bus-supervisor-launch.json')), None))

    def variant_wait(self, variant):
        return self.wait_until(lambda: next((row for row in self.rows('bus-supervisor-waits.jsonl')
                                            if row['pid'] == variant['pid']), None))

    def request_stop(self):
        # This dedicated controller is the only stop-file writer. Publish fully
        # fsynced bytes with Linux RENAME_NOREPLACE: readers never see an empty
        # request, an existing target is never overwritten, and nlink stays1.
        raw = (json.dumps({'studyRunID': STUDY, 'request': 'normal-owned-bus-stop'},
                          sort_keys=True) + '\n').encode()
        directory = os.open(self.root, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
        try:
            parent = os.fstat(directory)
            self.assertEqual(core_identity(parent), core_identity(self.root.lstat()))
            self.assertEqual((parent.st_uid, parent.st_gid, stat.S_IMODE(parent.st_mode)), (0, 0, 0o700))
            try:
                os.stat('bus-stop-request.json', dir_fd=directory, follow_symlinks=False)
            except FileNotFoundError:
                pass
            else:
                self.fail('ordinary stop target already exists; no overwrite permitted')
            pending = 'test-stop-request-pending.json'
            fd = os.open(pending, os.O_WRONLY | os.O_CREAT | os.O_EXCL |
                         os.O_NOFOLLOW | os.O_CLOEXEC, 0o600, dir_fd=directory)
            try:
                core.write_all(fd, raw)
                os.fsync(fd)
                prepared = os.fstat(fd)
                self.assertEqual((prepared.st_nlink, prepared.st_size), (1, len(raw)))
                self.assertEqual((prepared.st_uid, prepared.st_gid, stat.S_IMODE(prepared.st_mode)), (0, 0, 0o600))
            finally:
                os.close(fd)
            emit(self.root / 'test-stop-request-byte-evidence.json',
                 {'exactUTF8': raw.decode(), 'bytes': len(raw),
                  'sha256': core.hashlib.sha256(raw).hexdigest(),
                  'soleWriterPID': os.getpid(), 'preparedIdentity': core_identity(prepared)})
            libc = core.ctypes.CDLL(None, use_errno=True)
            rename = libc.renameat2  # No weaker fallback if the kernel/libc lacks it.
            rename.argtypes = [core.ctypes.c_int, core.ctypes.c_char_p,
                               core.ctypes.c_int, core.ctypes.c_char_p, core.ctypes.c_uint]
            rename.restype = core.ctypes.c_int
            if rename(directory, pending.encode(), directory, b'bus-stop-request.json', 1) != 0:
                error = core.ctypes.get_errno()
                raise OSError(error, os.strerror(error))
            os.fsync(directory)
            published = os.stat('bus-stop-request.json', dir_fd=directory, follow_symlinks=False)
            self.assertEqual(core_identity(published), core_identity(prepared))
            self.assertEqual((published.st_nlink, published.st_size), (1, len(raw)))
            self.assertEqual(core_identity(os.fstat(directory)), core_identity(self.root.lstat()))
            emit(self.root / 'test-stop-request-publication.json',
                 {'atomicRenameNoReplace': True, 'publishedNlink': published.st_nlink,
                  'publishedIdentity': core_identity(published),
                  'bytes': len(raw), 'sha256': core.hashlib.sha256(raw).hexdigest()})
        finally:
            os.close(directory)

    def result(self):
        actual = self.process.wait(timeout=8)  # Original Popen object, no synthetic status.
        emit(self.root / 'test-original-supervisor-wait.json',
             {'method': 'subprocess.Popen.wait', 'returncode': actual})
        value = json.loads((self.root / 'bus-supervisor-result.json').read_text())
        self.assertEqual(actual, value['supervisorExitCode'])
        self.assertEqual(value['signalsSent'], [])
        self.assertEqual(value['waits'], self.rows('bus-supervisor-waits.jsonl'))
        self.assertEqual(value['waitLogSHA256'], core.hashlib.sha256(
            (self.root / 'bus-supervisor-waits.jsonl').read_bytes()).hexdigest())
        self.assertEqual(value['remainingDirectChildren'], [])
        self.assertTrue(value['allChildrenReaped'])
        self.assertFalse(value['timedOut'])
        self.assertFalse(value['allChildExitCodesClaimedZero'])
        for row in value['waits']:
            self.assertFalse(row['signalAuthorityFromWaitRecord'])
            self.assertTrue(row['reaped'])
            self.assertEqual(row['exitCode'], os.waitstatus_to_exitcode(row['rawWaitStatus']))
        return value

    def assert_original_absence(self, launch, result):
        identities = [launch['supervisor'], launch['directChild']]
        identities += [row['identity'] for row in result['waits'] if row['identity'] is not None]
        identities += [row['identity'] for row in self.rows('fixture-events.jsonl') if 'identity' in row]
        pids = {row['pid'] for row in identities}
        pids.update(row['pid'] for row in self.rows('fixture-events.jsonl') if row['kind'] == 'intermediate-wait')
        observed = {str(pid): not (Path('/proc') / str(pid)).exists() for pid in pids}
        emit(self.root / 'test-original-pid-absence.json', observed)
        self.assertTrue(all(observed.values()))

    def accepted_variant(self, mode, key, expected):
        launch = self.start(mode)
        variant = self.event('variant-live')['identity']
        row = self.variant_wait(variant)
        self.assertEqual(row['pid'], variant['pid'])
        self.assertTrue(variant['startTicks'].isdecimal())
        self.assertGreater(int(variant['startTicks']), 0)
        self.assertEqual(variant['ppid'], launch['supervisor']['pid'])
        self.assertFalse(row['directChild'])
        self.assertEqual(row['identity']['startTicks'], variant['startTicks'])
        self.assertEqual(row['identity'][key], expected)
        self.assertEqual(row['identity']['ppid'], launch['supervisor']['pid'])
        self.assertEqual(row['identity']['state'], 'Z')
        self.assertEqual(row['identity']['uids'][0], 0)
        self.assertEqual(row['identity']['gids'][0], 0)
        self.assertEqual(row['waitidUID'], 0)
        for name in ['eventPIDMatchesSnapshot', 'kernelParentIsSupervisor', 'exitedZombie',
                     'waitidRealUIDMatches', 'realUIDMatches', 'realGIDMatches']:
            self.assertTrue(row['identityChecks'][name], name)
        changed_check = 'allUIDsMatch' if key == 'uids' else 'allGIDsMatch'
        self.assertFalse(row['identityChecks'][changed_check])
        self.assertTrue(row['adoptedReapOnlyCredentialVariant'])
        self.assertFalse(row['signalAuthorityFromWaitRecord'])
        self.assertIsNone(row['identityError'])
        self.assertEqual(row['rawWaitStatus'], 23 << 8)
        self.assertEqual(row['exitCode'], 23)
        self.assertIsNone(row['signal'])
        # acb would start its fatal deadline at this wait. Keep the ordinary direct
        # holder active for three times that deadline, with no stop file yet.
        begin = time.monotonic()
        while time.monotonic() - begin < SURVIVAL_WINDOW:
            self.assertIsNone(self.process.poll(), 'supervisor exited before ordinary stop')
            self.assertFalse((self.root / 'bus-supervisor-result.json').exists())
            self.assertFalse((self.root / 'bus-stop-request.json').exists())
            time.sleep(0.01)
        emit(self.root / 'test-before-stop-survival.json',
             {'alivePastSeconds': time.monotonic() - begin, 'shortDeadline': SHORT_DEADLINE,
              'originalSupervisorHandlePending': True, 'terminalResultAbsent': True,
              'ordinaryStopNotYetRequested': True})
        self.request_stop()
        result = self.result()
        self.assertEqual(self.process.returncode, 0)
        self.assertEqual(result['errors'], [])
        direct = result['directChildWait']
        self.assertEqual(direct['pid'], launch['directChild']['pid'])
        self.assertEqual(direct['identity']['startTicks'], launch['directChild']['startTicks'])
        self.assertEqual(direct['rawWaitStatus'], 0)
        self.assertEqual(direct['exitCode'], 0)
        self.assertEqual(direct['identity']['uids'], [0] * 4)
        self.assertEqual(direct['identity']['gids'], [0] * 4)
        self.assertFalse(direct['adoptedReapOnlyCredentialVariant'])
        returned = self.event('direct-ordinary-return')
        self.assertEqual(returned['pid'], direct['pid'])
        self.assertEqual(returned['returnCode'], 0)
        self.assert_original_absence(launch, result)

    def rejected_variant(self, mode, failed):
        launch = self.start(mode, timeout=1.5)
        variant = self.event('variant-live')['identity']
        row = self.variant_wait(variant)
        self.assertEqual(row['pid'], variant['pid'])
        self.assertTrue(variant['startTicks'].isdecimal())
        self.assertGreater(int(variant['startTicks']), 0)
        self.assertEqual(variant['ppid'], launch['supervisor']['pid'])
        self.assertEqual(row['directChild'], mode.startswith('direct-'))
        if row['directChild']:
            self.assertEqual(row['pid'], launch['directChild']['pid'])
            self.assertEqual(variant['startTicks'], launch['directChild']['startTicks'])
        self.assertIsNotNone(row['identity'])
        self.assertEqual(row['identity']['startTicks'], variant['startTicks'])
        self.assertEqual(row['identity']['uids'], variant['uids'])
        self.assertEqual(row['identity']['gids'], variant['gids'])
        self.assertEqual(row['identity']['ppid'], launch['supervisor']['pid'])
        self.assertEqual(row['identity']['state'], 'Z')
        for name in failed:
            self.assertFalse(row['identityChecks'][name], name)
            self.assertIn(name, row['identityError'])
        self.assertFalse(row['adoptedReapOnlyCredentialVariant'])
        self.assertFalse(row['signalAuthorityFromWaitRecord'])
        self.assertEqual(row['rawWaitStatus'], 23 << 8)
        self.assertEqual(row['exitCode'], 23)
        if mode.startswith('adopted-'):
            self.request_stop()  # Close the direct holder before its existing error deadline.
        result = self.result()
        expected_exit = 125 if mode.startswith('adopted-') else 23
        self.assertEqual(self.process.returncode, expected_exit)
        self.assertTrue(any('waitable identity:' in message for message in result['errors']))
        self.assertEqual(result['directChildWait']['rawWaitStatus'],
                         0 if mode.startswith('adopted-') else 23 << 8)
        self.assert_original_absence(launch, result)

    def test_adopted_effective_uid_and_fsuid_are_reap_only(self):
        self.accepted_variant('adopted-effective-uid', 'uids', [0, 65534, 0, 65534])

    def test_adopted_saved_uid_is_reap_only(self):
        self.accepted_variant('adopted-saved-uid', 'uids', [0, 0, 65534, 0])

    def test_adopted_effective_gid_and_fsgid_are_reap_only(self):
        self.accepted_variant('adopted-effective-gid', 'gids', [0, 65534, 0, 65534])

    def test_adopted_saved_gid_is_reap_only(self):
        self.accepted_variant('adopted-saved-gid', 'gids', [0, 0, 65534, 0])

    def test_adopted_changed_real_uid_keeps_snapshot_and_fails(self):
        self.rejected_variant('adopted-real-uid', ['realUIDMatches', 'waitidRealUIDMatches'])

    def test_adopted_changed_real_gid_keeps_snapshot_and_fails(self):
        self.rejected_variant('adopted-real-gid', ['realGIDMatches'])

    def test_direct_effective_uid_keeps_strict_full_credentials(self):
        self.rejected_variant('direct-effective-uid', ['allUIDsMatch'])

    def test_direct_saved_uid_keeps_strict_full_credentials(self):
        self.rejected_variant('direct-saved-uid', ['allUIDsMatch'])

    def test_direct_effective_gid_keeps_strict_full_credentials(self):
        self.rejected_variant('direct-effective-gid', ['allGIDsMatch'])

    def test_direct_saved_gid_keeps_strict_full_credentials(self):
        self.rejected_variant('direct-saved-gid', ['allGIDsMatch'])


if __name__ == '__main__':
    os.umask(0o077)
    if len(sys.argv) > 1 and sys.argv[1] == '--fixture':
        sys.exit(fixture(sys.argv[2], Path(sys.argv[3]), int(sys.argv[4])))
    if len(sys.argv) > 1 and sys.argv[1] == '--supervisor':
        if not AUTHORIZED or sys.argv[4] not in MODES:
            raise RuntimeError('explicit isolated root fixture authorization required')
        root = Path(sys.argv[2])
        command = [sys.executable, '-I', '-B', str(Path(__file__).resolve()),
                   '--fixture', sys.argv[4], str(root), sys.argv[5]]
        value = core.supervise(command, root, study_run_id=STUDY, expected_uid=0,
                               expected_gid=0, shutdown_timeout=float(sys.argv[3]))
        sys.exit(value['supervisorExitCode'])
    unittest.main()

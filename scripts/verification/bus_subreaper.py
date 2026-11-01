"""Reap only an owned private-bus hierarchy; no signals or resource lifecycle.

Production CLI is UID/GID 65534 and the fixed verification Root.  The library
entry point is also used by separately authorized Linux process-fixture tests.
Importing this module never enables a subreaper or launches a child.
"""
import argparse
import ctypes
import hashlib
import json
import math
import os
from pathlib import Path
import re
import signal
import stat
import sys
import time

ROOT = Path('/workspace/friendly-pdf-unprivileged-native')
PR_SET_CHILD_SUBREAPER = 36
PR_GET_CHILD_SUBREAPER = 37
MAX_WAITS = 256
MAX_LOG_BYTES = 1048576


def require(value, message):
    if not value:
        raise ValueError(message)


def strict_json(data):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    return json.loads(data, object_pairs_hook=unique,
                      parse_constant=lambda value: require(False, 'nonfinite JSON'))


def same_credentials(uid, gid):
    require(os.getresuid() == (uid, uid, uid), 'real/effective/saved UID mismatch')
    require(os.getresgid() == (gid, gid, gid), 'real/effective/saved GID mismatch')


def enable_subreaper():
    require(sys.platform == 'linux', 'Linux subreaper required')
    require(hasattr(os, 'waitid') and hasattr(os, 'WNOWAIT'), 'waitid WNOWAIT required')
    require(signal.getsignal(signal.SIGCHLD) == signal.SIG_DFL,
            'SIGCHLD must retain waitable children')
    libc = ctypes.CDLL(None, use_errno=True)
    prctl = libc.prctl
    prctl.argtypes = [ctypes.c_int] + [ctypes.c_ulong] * 4
    prctl.restype = ctypes.c_int
    for option, argument in [(PR_SET_CHILD_SUBREAPER, 1)]:
        if prctl(option, argument, 0, 0, 0) != 0:
            raise OSError(ctypes.get_errno(), 'PR_SET_CHILD_SUBREAPER')
    enabled = ctypes.c_int()
    if prctl(PR_GET_CHILD_SUBREAPER, ctypes.addressof(enabled), 0, 0, 0) != 0:
        raise OSError(ctypes.get_errno(), 'PR_GET_CHILD_SUBREAPER')
    require(enabled.value == 1, 'subreaper was not enabled')


def process_identity(pid):
    """Snapshot kernel identity before waitpid removes a zombie's /proc entry."""
    path = Path('/proc') / str(pid)
    before = (path / 'stat').read_text().rsplit(')', 1)[1].split()
    fields = {line.split(':', 1)[0]: line.split(':', 1)[1].split()
              for line in (path / 'status').read_text().splitlines() if ':' in line}
    raw_argv = (path / 'cmdline').read_bytes()
    after = (path / 'stat').read_text().rsplit(')', 1)[1].split()
    require((before[1], before[19]) == (after[1], after[19]), 'process identity changed')
    return {'pid': pid, 'ppid': int(after[1]), 'startTicks': after[19],
            'state': after[0], 'uids': [int(v) for v in fields['Uid']],
            'gids': [int(v) for v in fields['Gid']],
            'argv': [v.decode() for v in raw_argv.split(b'\0') if v],
            'argvSHA256': hashlib.sha256(raw_argv).hexdigest()}


def write_all(fd, data):
    while data:
        count = os.write(fd, data)
        require(count > 0, 'short metadata write')
        data = data[count:]


class Records:
    """Four exclusive files inside an already owned private directory."""
    names = ('bus-supervisor-launch.json', 'bus-supervisor-waits.jsonl',
             'bus-supervisor-child-exit.json', 'bus-supervisor-result.json')

    def __init__(self, directory, uid, gid):
        path = Path(directory)
        require(path.is_absolute() and path.resolve(strict=True) == path,
                'canonical private state directory required')
        self.fd = os.open(path, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
        self.log = None
        try:
            self.st = os.fstat(self.fd)
            require((self.st.st_uid, self.st.st_gid, stat.S_IMODE(self.st.st_mode)) ==
                    (uid, gid, 0o700), 'private state directory ownership/mode')
            for name in self.names:
                try:
                    os.stat(name, dir_fd=self.fd, follow_symlinks=False)
                except FileNotFoundError:
                    continue
                raise FileExistsError(name)
            self.log = os.open(self.names[1], os.O_WRONLY | os.O_CREAT | os.O_EXCL |
                               os.O_NOFOLLOW | os.O_CLOEXEC, 0o600, dir_fd=self.fd)
            self.log_hash = hashlib.sha256()
            self.log_bytes = 0
        except BaseException:
            self.close()
            raise

    def emit(self, name, value):
        require(name in self.names and name != self.names[1], 'fixed record filename')
        data = (json.dumps(value, sort_keys=True) + '\n').encode()
        fd = os.open(name, os.O_WRONLY | os.O_CREAT | os.O_EXCL |
                     os.O_NOFOLLOW | os.O_CLOEXEC, 0o600, dir_fd=self.fd)
        try:
            write_all(fd, data)
            os.fsync(fd)
        finally:
            os.close(fd)
        os.fsync(self.fd)

    def append(self, value):
        data = (json.dumps(value, sort_keys=True) + '\n').encode()
        require(len(data) <= 16384 and self.log_bytes + len(data) <= MAX_LOG_BYTES,
                'raw wait log bound exceeded')
        write_all(self.log, data)
        os.fsync(self.log)
        self.log_hash.update(data)
        self.log_bytes += len(data)

    def close(self):
        if self.log is not None:
            os.close(self.log)
            self.log = None
        if getattr(self, 'fd', None) is not None:
            os.close(self.fd)
            self.fd = None


def stop_requested(records, name, study, uid, gid):
    """The original private stop-request protocol; never creates or edits it."""
    try:
        fd = os.open(name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC,
                     dir_fd=records.fd)
    except FileNotFoundError:
        return False
    try:
        before = os.fstat(fd)
        require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and
                before.st_dev == records.st.st_dev and
                (before.st_uid, before.st_gid) == (uid, gid) and
                not before.st_mode & 0o022 and 0 < before.st_size <= 8192,
                'stop-request file boundary')
        raw = os.read(fd, 8193)
        after = os.fstat(fd)
        current = os.stat(name, dir_fd=records.fd, follow_symlinks=False)
        key = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
        require(key(before) == key(after) == key(current) and len(raw) == before.st_size,
                'stop-request changed')
        require(strict_json(raw) == {'studyRunID': study, 'request': 'normal-owned-bus-stop'},
                'stop-request binding')
        return True
    finally:
        os.close(fd)


def remaining_children():
    result = []
    for path in Path('/proc').iterdir():
        if not path.name.isdigit():
            continue
        try:
            row = (path / 'stat').read_text().rsplit(')', 1)[1].split()
            if int(row[1]) == os.getpid():
                result.append(process_identity(int(path.name)))
        except (FileNotFoundError, ProcessLookupError):
            continue
    return result


def supervise(command, state_dir, *, study_run_id, expected_uid, expected_gid,
              shutdown_timeout=15.0, stop_name='bus-stop-request.json'):
    """Return a truthful outcome; no signal is sent, including on timeout.

    A successful outer code describes the direct bus child's exit plus complete
    reaping. Adopted child raw statuses stay distinct; it does not mean all child
    exits were zero or that native/service/archive acceptance has occurred.
    """
    require(not sys.flags.optimize, 'assertions-enabled interpreter required')
    require(isinstance(command, list) and command and
            all(isinstance(v, str) and v and '\0' not in v for v in command), 'exact argv required')
    require(re.fullmatch('[0-9a-f]{32}', study_run_id), 'finite study ID required')
    require(math.isfinite(shutdown_timeout) and 0 < shutdown_timeout <= 120, 'bounded shutdown deadline')
    require(stop_name == 'bus-stop-request.json', 'fixed stop-request filename')
    same_credentials(expected_uid, expected_gid)
    enable_subreaper()
    records = Records(state_dir, expected_uid, expected_gid)
    gate_read, gate_write = os.pipe2(os.O_CLOEXEC)
    error_read, error_write = os.pipe2(os.O_CLOEXEC)
    child = None
    errors = []
    def add_error(message):
        if message not in errors and len(errors) < 16:
            errors.append(message)
    waits = []
    direct = None
    deadline = None
    timed_out = False
    all_reaped = False
    exec_error = bytearray()
    try:
        child = os.fork()
        if child == 0:
            os.close(gate_write)
            os.close(error_read)
            try:
                require(os.read(gate_read, 1) == b'G', 'launch not committed')
                os.close(gate_read)
                same_credentials(expected_uid, expected_gid)
                os.execvpe(command[0], command, dict(os.environ))
            except BaseException as exc:
                try:
                    write_all(error_write, str(type(exc).__name__ + ': ' + str(exc)).encode()[:4096])
                finally:
                    os._exit(127)
        os.close(gate_read)
        gate_read = None
        os.close(error_write)
        error_write = None
        os.set_blocking(error_read, False)
        supervisor = process_identity(os.getpid())
        fork_identity = process_identity(child)  # Child blocks on gate: start ticks cannot be missed/reused.
        require(fork_identity['ppid'] == os.getpid() and
                fork_identity['uids'] == [expected_uid] * 4 and
                fork_identity['gids'] == [expected_gid] * 4, 'fork identity/credentials')
        launch = {'kind': 'owned-bus-subreaper-launch-v1', 'studyRunID': study_run_id,
                  'supervisor': supervisor, 'directChild': fork_identity,
                  'expectedArgv': command, 'childNotYetExecAtSnapshot': True,
                  'subreaperVerified': True, 'expectedUID': expected_uid, 'expectedGID': expected_gid}
        try:
            records.emit(records.names[0], launch)
            write_all(gate_write, b'G')
        except (OSError, ValueError) as exc:
            add_error('launch record: ' + str(exc))
            deadline = time.monotonic() + shutdown_timeout
        finally:
            os.close(gate_write)  # EOF makes an uncommitted child exit127, never launch descendants.
            gate_write = None

        while True:
            now = time.monotonic()
            if deadline is None:
                try:
                    if stop_requested(records, stop_name, study_run_id, expected_uid, expected_gid):
                        deadline = now + shutdown_timeout
                except (OSError, ValueError) as exc:
                    add_error('stop request: ' + str(exc))
                    deadline = now + shutdown_timeout
            try:
                part = os.read(error_read, 4096 - len(exec_error)) if len(exec_error) < 4096 else b''
                exec_error.extend(part)
            except BlockingIOError:
                pass
            try:
                event = os.waitid(os.P_ALL, 0, os.WEXITED | os.WNOHANG | os.WNOWAIT)
            except ChildProcessError:
                all_reaped = True
                break
            except InterruptedError:
                continue
            if event is not None:
                identity_error = None
                try:
                    identity = process_identity(event.si_pid)
                    require(identity['ppid'] == os.getpid() and identity['state'] == 'Z' and
                            identity['uids'] == [expected_uid] * 4 and
                            identity['gids'] == [expected_gid] * 4, 'waitable child identity/credentials')
                except (OSError, ValueError) as exc:
                    identity = None
                    identity_error = str(exc)
                    add_error('waitable identity: ' + identity_error)
                pid, raw_status = os.waitpid(event.si_pid, 0)
                require(pid == event.si_pid, 'waitpid identity mismatch')
                value = {'pid': pid, 'identity': identity, 'identityError': identity_error,
                         'directChild': pid == child, 'rawWaitStatus': raw_status,
                         'exitCode': os.waitstatus_to_exitcode(raw_status),
                         'signal': os.WTERMSIG(raw_status) if os.WIFSIGNALED(raw_status) else None,
                         'waitidCode': event.si_code, 'waitidStatus': event.si_status,
                         'waitidUID': event.si_uid,
                         'reaped': True}
                if pid == child:
                    if identity is None or identity['startTicks'] != fork_identity['startTicks']:
                        add_error('direct child start ticks mismatch')
                    direct = value
                    if deadline is None:
                        deadline = time.monotonic() + shutdown_timeout
                    try:
                        records.emit(records.names[2], {'kind': 'owned-bus-child-exit-v1',
                                     'studyRunID': study_run_id, 'supervisorPID': os.getpid(),
                                     'supervisorStartTicks': supervisor['startTicks'], 'wait': value})
                    except (OSError, ValueError) as exc:
                        add_error('child-exit record: ' + str(exc))
                if len(waits) < MAX_WAITS:
                    waits.append(value)
                    try:
                        records.append(value)
                    except (OSError, ValueError) as exc:
                        add_error('wait log: ' + str(exc))
                else:
                    add_error('child count bound exceeded')
                if errors and deadline is None:
                    deadline = time.monotonic() + shutdown_timeout
                if deadline is not None and time.monotonic() >= deadline:
                    timed_out = True
                    break
                continue
            if deadline is not None and now >= deadline:
                timed_out = True
                break
            time.sleep(0.02)

        # Complete the exec-error pipe after reaping; successful exec closed its write end.
        try:
            exec_error.extend(os.read(error_read, max(0, 4096 - len(exec_error))))
        except BlockingIOError:
            pass
        if exec_error:
            add_error('exec: ' + exec_error.decode(errors='replace'))
        if direct is None:
            code = 124 if timed_out else 125
        elif timed_out:
            code = 124
        elif direct['exitCode'] != 0:
            original_code = direct['exitCode']
            code = original_code if original_code >= 0 else 128 - original_code
        elif errors or not all_reaped:
            code = 125
        else:
            code = 0
        require(all_reaped or timed_out, 'missing terminal supervision state')
        result = {'kind': 'owned-bus-subreaper-result-v1', 'studyRunID': study_run_id,
                  'supervisor': supervisor, 'directChildPID': child, 'directChildWait': direct,
                  'waits': waits, 'waitLogSHA256': records.log_hash.hexdigest(),
                  'waitLogBytes': records.log_bytes, 'allChildrenReaped': all_reaped,
                  'timedOut': timed_out, 'errors': errors, 'supervisorExitCode': code,
                  'remainingDirectChildren': [] if all_reaped else remaining_children(),
                  'signalsSent': [], 'nativeAccepted': False,
                  'allChildExitCodesClaimedZero': False, 'strictPIDAbsenceStillRequired': True}
        records.emit(records.names[3], result)
        return result
    finally:
        for fd in [gate_read, gate_write, error_read, error_write]:
            if fd is not None:
                os.close(fd)
        records.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--state-dir', type=Path, required=True)
    parser.add_argument('--study-run-id', required=True)
    parser.add_argument('--shutdown-timeout', type=float, default=15.0)
    args = parser.parse_args()
    try:
        require(args.state_dir == ROOT, 'fixed verification Root required')
        command = ['dbus-run-session', '--', '/usr/bin/python3', '-B', str(ROOT / 'direct_guest.py'), 'hold']
        result = supervise(command, args.state_dir, study_run_id=args.study_run_id,
                           expected_uid=65534, expected_gid=65534,
                           shutdown_timeout=args.shutdown_timeout)
        return result['supervisorExitCode']
    except (OSError, ValueError) as exc:
        print(type(exc).__name__ + ': ' + str(exc), file=sys.stderr, flush=True)
        return 125


if __name__ == '__main__':
    sys.exit(main())

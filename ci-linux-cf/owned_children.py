"""Linux process-local subreaper + stable pidfd ownership. No global PID signals.
Construct only in a dedicated single-thread guest wrapper, before any spawn.
/proc discovery may miss churn; only kernel ECHILD grants quiescence.
"""
from pathlib import Path
import ctypes,errno,hashlib,os,signal,sys,time
import ci_common as b
WAIT_ALL=0x40000000  # Linux __WALL: include clone/non-SIGCHLD children too.

class LinuxChildren:
    def __init__(self):
        b.require(sys.platform=='linux' and sys.implementation.name=='cpython','Linux CPython required')
        b.require(hasattr(os,'pidfd_open') and hasattr(signal,'pidfd_send_signal'),'pidfd API unavailable; refuse before cache removal')
        b.require(len(list(Path('/proc/self/task').iterdir()))==1,'dedicated single-thread wrapper required')
        self.pid=os.getpid();self.bindings=[];self.reaped=[];self.reaped_count=0;self.reaped_hash=hashlib.sha256();self.sequence=0
        # CPython installs sigaction with SA_ONSTACK, without SA_NOCLDWAIT;
        # no SIGCHLD-ignore/reaper thread may race waitpid or pidfd binding.
        signal.signal(signal.SIGCHLD,signal.SIG_DFL)
        libc=ctypes.CDLL(None,use_errno=True);libc.prctl.restype=ctypes.c_int
        result=libc.prctl(ctypes.c_int(36),ctypes.c_ulong(1),ctypes.c_ulong(0),ctypes.c_ulong(0),ctypes.c_ulong(0))
        b.require(result==0,'PR_SET_CHILD_SUBREAPER failed errno='+str(ctypes.get_errno()))
        got=ctypes.c_int();result=libc.prctl(ctypes.c_int(37),ctypes.byref(got),ctypes.c_ulong(0),ctypes.c_ulong(0),ctypes.c_ulong(0))
        b.require(result==0 and got.value==1,'subreaper state verification')
        fd=os.pidfd_open(self.pid,0)
        try:signal.pidfd_send_signal(fd,0,None,0)
        finally:os.close(fd)
        try:
            observed=os.waitpid(-1,os.WNOHANG|WAIT_ALL)
        except ChildProcessError:observed=None
        except OSError as e:
            if e.errno==errno.ECHILD:observed=None
            else:raise
        b.require(observed is None,'wrapper has pre-existing children; ownership contract violated')
        self.contract={'linuxSubreaperVerified':True,'pidfdOpenAndSignalZeroVerified':True,'singleThreadVerified':True,'initialKernelECHILD':True,'waitAllChildrenVerified':True,'wrapperPID':self.pid}
    def wait_empty(self,proc):
        for _ in range(4096):
            try:pid,status=os.waitpid(-1,os.WNOHANG|WAIT_ALL)
            except ChildProcessError:return True
            except OSError as e:
                if e.errno==errno.ECHILD:return True
                raise
            if pid==0:return False
            code=os.waitstatus_to_exitcode(status);event={'pid':pid,'exit':code};self.reaped_count+=1;self.reaped_hash.update(b.encoded(event))
            if len(self.reaped)<16:self.reaped.append(event)
            if proc is not None and pid==proc.pid and proc.returncode is None:proc.returncode=code
        return False  # Batch bound is not ECHILD; cleanup must keep checking.
    def reap_summary(self):return {'count':self.reaped_count,'sha256':self.reaped_hash.hexdigest(),'first16':list(self.reaped)}
    def identity(self,pid):
        text=Path('/proc/'+str(pid)+'/stat').read_text();tail=text[text.rfind(')')+2:].split()
        return {'pid':pid,'ppid':int(tail[1]),'startTicks':tail[19]}
    def bind(self,pid):
        fd=None
        try:
            before=self.identity(pid);b.require(before['ppid']==self.pid,'candidate is not this wrapper child')
            fd=os.pidfd_open(pid,0);after=self.identity(pid)
            b.require(before==after and after['ppid']==self.pid,'child changed while binding')
            self.sequence+=1;binding={'fd':fd,'identity':after,'bindingSequence':self.sequence};self.bindings.append(binding);return binding
        except (FileNotFoundError,ProcessLookupError):
            if fd is not None:os.close(fd)
            return None
        except Exception:
            if fd is not None:os.close(fd)
            raise
    def direct_bindings(self):
        path=Path('/proc/self/task')/str(self.pid)/'children'
        # This list is discovery only. An empty/churned read never proves empty.
        values=path.read_text().split();b.require(len(values)<=4096,'owned child bound')
        result=[]
        for value in values:
            binding=self.bind(int(value))
            if binding is not None:result.append(binding)
        return result
    def send(self,binding,sig):
        try:signal.pidfd_send_signal(binding['fd'],sig,None,0)
        except ProcessLookupError:pass  # Stable handle is dead, never retargeted.
    def release(self,binding):
        if binding in self.bindings:self.bindings.remove(binding);os.close(binding['fd'])
    def close(self):
        for binding in list(self.bindings):self.release(binding)

def drain_owned(owner,proc,journal,clock=time.monotonic,sleep=time.sleep,grace=3,timeout=30):
    start=clock();sent=[];signal_count=0;signal_hash=hashlib.sha256()
    while True:
        if owner.wait_empty(proc):return {'kernelECHILD':True,'signalsFirst16':sent,'signalCount':signal_count,'signalSHA256':signal_hash.hexdigest(),'reaped':owner.reap_summary(),'contract':owner.contract}
        if clock()-start>=timeout:return {'kernelECHILD':False,'signalsFirst16':sent,'signalCount':signal_count,'signalSHA256':signal_hash.hexdigest(),'reaped':owner.reap_summary(),'contract':owner.contract,'reason':'owned child drain deadline; recovery ownership required'}
        sig=signal.SIGTERM if clock()-start<grace else signal.SIGKILL
        for binding in owner.direct_bindings():
            identity={**binding['identity'],'bindingSequence':binding['bindingSequence']}
            try:
                journal.record('owned-signal-intent',identity=identity,signal=int(sig));owner.send(binding,sig);event={'identity':identity,'signal':int(sig)};signal_count+=1;signal_hash.update(b.encoded(event))
                if len(sent)<16:sent.append(event)
            finally:owner.release(binding)
        sleep(.05)

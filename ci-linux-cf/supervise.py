"""One owned Cargo/package child tree. Kernel ECHILD, never leader/PID-set exit,
authorizes further file work. Pure fixtures inject a fake kernel boundary.
"""
from pathlib import Path
import os,subprocess,time
import ci_common as b
from owned_children import LinuxChildren,drain_owned

def run_owned(argv,cwd,env,logs,label,journal,timeout=21600,reserve=1024**3,
              free_bytes=None,spawn=subprocess.Popen,clock=time.monotonic,
              sleep=time.sleep,children=None):
    import shutil
    cwd=Path(cwd);logs=Path(logs);b.require(cwd.is_dir() and not cwd.is_symlink(),'owned source cwd')
    b.require(label and label.replace('-','').isalnum(),'job label');out=logs/(label+'.out');err=logs/(label+'.err');b.require(not out.exists() and not err.exists(),'job already attempted; no blind retry')
    if free_bytes is None:free_bytes=lambda:shutil.disk_usage(logs).free
    owner=children if children is not None else LinuxChildren()
    proc=None;reason=None;started=clock();code=None;proof={'kernelECHILD':False};failure=None;leader_binding=None
    journal.record('job-intent',label=label,argv=argv,cwd=str(cwd),childContract=owner.contract)
    with out.open('xb') as stdout,err.open('xb') as stderr:
        try:
            b.require(free_bytes()>=reserve,'guest reserve before owned child')
            proc=spawn(argv,cwd=cwd,env=env,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,start_new_session=True)
            leader_binding=owner.bind(proc.pid);b.require(leader_binding is not None,'leader identity unavailable; prove quiescence before file work')
            journal.record('job-started',label=label,pid=proc.pid,identity=leader_binding['identity'],bindingSequence=leader_binding['bindingSequence'])
            while proc.poll() is None:
                if clock()-started>timeout:reason='owned child deadline';break
                if free_bytes()<reserve:reason='owned child guest reserve';break
                if out.stat().st_size+err.stat().st_size>64*1024**2 or sum(p.stat().st_size for p in logs.iterdir() if p.is_file())>128*1024**2:reason='owned logs bound';break
                sleep(.25)
            b.require(reason is None,reason or 'owned child guard')
            code=proc.returncode;b.require(code==0,'owned command failed: '+label)
        except BaseException as e:failure=e
        finally:
            try:
                # Even successful leader exit is insufficient. Kernel wait covers
                # adopted descendants, including double-forks/session changes.
                empty=owner.wait_empty(proc)
                if empty:proof={'kernelECHILD':True,'signalsFirst16':[],'signalCount':0,'reaped':owner.reap_summary(),'contract':owner.contract}
                else:
                    if failure is None:failure=b.Refused('owned descendants survived leader success')
                    proof=drain_owned(owner,proc,journal,clock,sleep)
            except BaseException as e:
                if failure is None:failure=e
                proof={'kernelECHILD':False,'error':type(e).__name__+': '+str(e),'contract':owner.contract}
            finally:
                owner.close();stdout.flush();stderr.flush();os.fsync(stdout.fileno());os.fsync(stderr.fileno())
            if proc is not None:code=proc.returncode
            journal.record('job-terminal',label=label,pid=proc.pid if proc else None,exit=code,reason=reason,failure=str(failure) if failure else None,quiescence=proof)
    b.require(proof.get('kernelECHILD') is True,'kernel child quiescence unavailable; recovery ownership retained')
    if failure is not None:raise failure
    return {'label':label,'argv':argv,'cwd':str(cwd),'pid':proc.pid,'exit':code,'seconds':round(clock()-started,3),'stdout':str(out),'stderr':str(err),'quiescence':proof}

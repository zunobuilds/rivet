"""Qualify only a temporary, user-systemd-owned Rivet cgroup. No host reconfiguration."""
import errno
import json
import os
import pathlib
import signal
import subprocess
import sys
import time
import uuid

ROOT = pathlib.Path(__file__).resolve().parent.parent
TMP = ROOT / '.build/tmp'
TMP.mkdir(parents=True, exist_ok=True)

def counters(path):
    return {k: int(v) for k, v in (line.split() for line in path.read_text().splitlines())}

def run_child(group, mode):
    # Probe is single-threaded. Enroll the fork before executing probe workload.
    def enroll():
        (group / 'cgroup.procs').write_text(str(os.getpid()))
    return subprocess.run(['/usr/bin/python3', str(__file__), mode], env={'PATH':'/usr/bin','TMPDIR':str(TMP)}, preexec_fn=enroll, capture_output=True, timeout=12)

def inside(unit):
    rel = pathlib.Path(pathlib.Path('/proc/self/cgroup').read_text().strip().split('::',1)[1])
    assert rel.name == unit + '.service'
    group = pathlib.Path('/sys/fs/cgroup') / str(rel).lstrip('/')
    assert f'user@{os.getuid()}.service' in str(group)
    result = {'unit':unit,'group':str(group),'controllers':(group/'cgroup.controllers').read_text().split()}
    assert {'cpu','memory','pids'} <= set(result['controllers'])
    supervisor = group / 'supervisor'
    worker = group / 'worker'
    supervisor.mkdir()
    (supervisor/'cgroup.procs').write_text(str(os.getpid()))
    try:
        (group/'cgroup.subtree_control').write_text('+cpu +memory +pids')
        worker.mkdir()
        (worker/'cpu.max').write_text('20000 100000')
        (worker/'memory.max').write_text(str(64*1024*1024))
        (worker/'memory.swap.max').write_text('0')
        (worker/'pids.max').write_text('4')
        result['limits'] = {n:(worker/n).read_text().strip() for n in ['cpu.max','memory.max','memory.swap.max','pids.max']}
        before = counters(worker/'cpu.stat')
        cpu = run_child(worker,'--cpu')
        after = counters(worker/'cpu.stat')
        assert cpu.returncode == 0, cpu.stderr.decode()
        assert after['nr_throttled'] > before['nr_throttled']
        result['cpu_throttles'] = after['nr_throttled']-before['nr_throttled']
        result['cpu_usage_usec'] = after['usage_usec']-before['usage_usec']
        before = counters(worker/'pids.events')
        pids = run_child(worker,'--pids')
        assert pids.returncode == 0, pids.stderr.decode()
        assert counters(worker/'pids.events')['max'] > before['max']
        result['pids'] = json.loads(pids.stdout)
        before = counters(worker/'memory.events')
        memory = run_child(worker,'--memory')
        assert memory.returncode == -signal.SIGKILL, memory.returncode
        assert counters(worker/'memory.events')['oom_kill'] > before['oom_kill']
        result['oom_kills'] = counters(worker/'memory.events')['oom_kill']-before['oom_kill']
        def enroll():
            (worker/'cgroup.procs').write_text(str(os.getpid()))
        detached = subprocess.Popen(['/usr/bin/python3',str(__file__),'--descendants'],env={'PATH':'/usr/bin','TMPDIR':str(TMP)},preexec_fn=enroll,stdout=subprocess.PIPE,start_new_session=True)
        owned = json.loads(detached.stdout.readline())
        assert set(owned) <= {int(p) for p in (worker/'cgroup.procs').read_text().split()}
        started = time.monotonic()
        (worker/'cgroup.kill').write_text('1')
        detached.wait(timeout=2)
        until = time.monotonic()+2
        while counters(worker/'cgroup.events')['populated'] and time.monotonic()<until:
            time.sleep(0.01)
        assert not counters(worker/'cgroup.events')['populated']
        result['kill_elapsed_ms'] = round((time.monotonic()-started)*1000,2)
        result['detached_process_count'] = len(owned)
        result['worker_empty'] = True
        print(json.dumps(result),flush=True)
    finally:
        if worker.exists():
            (worker/'cgroup.kill').write_text('1')
            worker.rmdir()
        (group/'cgroup.subtree_control').write_text('-cpu -memory -pids')
        (group/'cgroup.procs').write_text(str(os.getpid()))
        supervisor.rmdir()

if len(sys.argv)>1:
    mode = sys.argv[1]
    if mode == '--inside':
        inside(sys.argv[2])
    elif mode == '--cpu':
        until=time.monotonic()+1.5
        while time.monotonic()<until:
            pass
    elif mode == '--pids':
        children=[]
        try:
            for _ in range(12):
                try:
                    pid=os.fork()
                except OSError as error:
                    assert error.errno==errno.EAGAIN
                    print(json.dumps({'fork_denied':True,'children':len(children)}),flush=True)
                    break
                if pid==0:
                    time.sleep(10)
                    os._exit(0)
                children.append(pid)
            else:
                raise AssertionError('PID controller did not deny bounded fork probe')
        finally:
            for pid in children:
                os.kill(pid,signal.SIGKILL)
                os.waitpid(pid,0)
    elif mode == '--memory':
        allocation=bytearray(128*1024*1024)
        raise AssertionError('memory controller did not kill allocation')
    elif mode == '--descendants':
        children=[]
        for _ in range(2):
            pid=os.fork()
            if pid==0:
                os.setsid()
                time.sleep(20)
                os._exit(0)
            children.append(pid)
        print(json.dumps([os.getpid()]+children),flush=True)
        time.sleep(20)
    else:
        raise AssertionError('unknown probe mode')
else:
    unit='rivet-cgroup-'+uuid.uuid4().hex
    command=['/usr/bin/systemd-run','--user','--unit='+unit,'--collect','--wait','--pipe','--service-type=exec',
             '--property=Delegate=yes','--property=MemoryMax=256M','--property=TasksMax=32','--property=CPUQuota=50%',
             '--property=MemorySwapMax=0','--property=OOMPolicy=continue','--property=RuntimeMaxSec=45s','--property=NoNewPrivileges=yes',
             '/usr/bin/env','-i','PATH=/usr/bin','TMPDIR='+str(TMP),'/usr/bin/python3',str(__file__),'--inside',unit]
    try:
        completed=subprocess.run(command,capture_output=True,text=True,timeout=55)
        print(completed.stdout,end='')
        print(completed.stderr,end='',file=sys.stderr)
        assert completed.returncode==0,completed.returncode
        evidence=json.loads(completed.stdout.strip())
        (ROOT/'.build/cgroup-evidence.json').write_text(json.dumps(evidence,indent=2)+'\n')
    finally:
        subprocess.run(['/usr/bin/systemctl','--user','stop',unit+'.service'],capture_output=True,timeout=10)
        check=subprocess.run(['/usr/bin/systemctl','--user','show',unit+'.service','--property=LoadState','--value'],capture_output=True,text=True,timeout=5)
        assert check.stdout.strip()=='not-found',check.stdout
    check=subprocess.run(['/usr/bin/systemctl','--user','show',unit+'.service','--property=LoadState','--value'],capture_output=True,text=True,timeout=5)
    assert check.stdout.strip()=='not-found',check.stdout
    print('PASS: CPU/memory/PID enforcement, detached descendant kill, transient-unit cleanup')

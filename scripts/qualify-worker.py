"""Run a bounded synthetic namespace/Landlock probe in an owned transient unit."""
import json
import pathlib
import subprocess
import tempfile
import uuid

root=pathlib.Path(__file__).resolve().parent.parent
build_digest=subprocess.check_output([str(root/'.build/target/debug/rivet'),'--version'],text=True).strip().split()[-1]
assert len(build_digest)==64 and all(c in '0123456789abcdef' for c in build_digest)
unit='rivet-boundary-'+uuid.uuid4().hex
with tempfile.TemporaryDirectory(dir=root/'.build/tmp') as directory:
    inputs=pathlib.Path(directory)
    (inputs/'probe.py').write_text('''import os,pathlib,socket,json
assert pathlib.Path('/work/input/probe.py').is_file()
assert not pathlib.Path('/home').exists()
assert not pathlib.Path('/run').exists()
assert not pathlib.Path('/sys/fs/cgroup').exists()
assert not pathlib.Path('/work/.git').exists()
assert not pathlib.Path('/work/.rivet-state').exists()
assert not pathlib.Path('/etc/passwd').exists()
assert 'HOME' not in os.environ and 'SSH_AUTH_SOCK' not in os.environ
assert all(name=='lo' for _,name in socket.if_nameindex())
for target in ['/work/input/mutation','/usr/bin/mutation','/proc/sys/kernel/mutation']:
    try: pathlib.Path(target).write_text('denied')
    except OSError: pass
    else: raise AssertionError(target)
s=socket.socket()
try: s.connect(('127.0.0.1',9))
except OSError: pass
else: raise AssertionError('TCP allowed')
pathlib.Path('/work/tmp/allowed').write_text('scratch')
assert pathlib.Path('/work/tmp/allowed').read_text()=='scratch'
print(json.dumps({'landlock':'fully-enforced','filesystem':'restricted','network':'denied','scratch':'bounded-private-tmpfs'}))
''')
    command=['/usr/bin/systemd-run','--user','--unit='+unit,'--collect','--wait','--pipe','--quiet','--service-type=exec',
        '--property=Delegate=cpu memory pids','--property=MemoryMax=256M','--property=MemorySwapMax=0','--property=CPUQuota=50%',
        '--property=TasksMax=32','--property=RuntimeMaxSec=15s','--property=NoNewPrivileges=yes','/usr/bin/env','-i','PATH=/usr/bin',
        '/usr/bin/bwrap','--unshare-all','--unshare-user','--disable-userns','--die-with-parent','--new-session','--cap-drop','ALL']
    for path in ['/usr/bin','/usr/lib','/usr/lib64','/usr/libexec','/usr/include']:
        command+=['--ro-bind',path,path]
    command+=['--symlink','usr/bin','/bin','--symlink','usr/lib','/lib','--symlink','usr/lib64','/lib64',
        '--ro-bind','/usr/bin/ld.bfd','/etc/alternatives/ld','--proc','/proc','--dev','/dev',
        '--ro-bind',str(inputs),'/work/input','--ro-bind',str(root/'.build/target/debug/rivet'),'/rivet-worker',
        '--size','67108864','--tmpfs','/work/tmp','--symlink','work/tmp','/tmp','--chdir','/work/input',
        '--clearenv','--setenv','PATH','/usr/bin','--remount-ro','/','--','/rivet-worker','__worker-bootstrap',build_digest,'--',
        '/usr/bin/python3','-B','/work/input/probe.py']
    try:
        result=subprocess.run(command,capture_output=True,text=True,timeout=25)
        print(result.stdout,end='')
        print(result.stderr,end='')
        assert result.returncode==0,result.returncode
        evidence=json.loads(result.stdout)
        (root/'.build/worker-evidence.json').write_text(json.dumps(evidence,indent=2)+'\n')
    finally:
        subprocess.run(['/usr/bin/systemctl','--user','stop',unit+'.service'],capture_output=True,timeout=10)
        state=subprocess.run(['/usr/bin/systemctl','--user','show',unit+'.service','--property=LoadState','--value'],capture_output=True,text=True,timeout=5)
        assert state.stdout.strip()=='not-found',state.stdout
print('PASS: real user-unit namespace/Landlock restrictions and cleanup')

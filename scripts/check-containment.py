"""Run inside the offline wrapper. Uses no personal-file probes or external calls."""
import os
import pathlib
import socket
import subprocess

assert pathlib.Path(os.getcwd()).is_relative_to('/work')
assert 'RIVET_SENTINEL_CREDENTIAL' not in os.environ
assert 'HOME' not in os.environ
assert 'SSH_AUTH_SOCK' not in os.environ
if 'OUT_DIR' not in os.environ:
    allowed = {
        'PATH', 'CARGO_HOME', 'CARGO_TARGET_DIR', 'TMPDIR', 'CARGO_NET_OFFLINE',
        'RUST_BACKTRACE', 'LC_CTYPE', 'PWD', 'RIVET_SOURCE_ROOT',
    }
    assert set(os.environ) <= allowed
assert os.stat('/tmp').st_ino == os.stat(os.environ['TMPDIR']).st_ino
assert os.stat('/tmp').st_dev == os.stat(os.environ['TMPDIR']).st_dev
assert not pathlib.Path('/home').exists()
assert not pathlib.Path('/root').exists()
assert not pathlib.Path('/.cargo').exists()
assert not pathlib.Path('/work/.cargo').exists()
assert not pathlib.Path('/run').exists()
assert not pathlib.Path('/etc/passwd').exists()
if pathlib.Path('/sys').exists():
    assert os.listdir('/sys') == []
try:
    pathlib.Path('/usr/.rivet-containment-probe').write_text('forbidden')
except OSError:
    pass
else:
    raise AssertionError('system mount writable')
# Network namespace must contain only loopback, with no usable external route.
interfaces = socket.if_nameindex()
assert all(name == 'lo' for _, name in interfaces), interfaces
s = socket.socket()
s.settimeout(0.1)
assert s.connect_ex(('192.0.2.1', 9)) != 0
s.close()
for tool in (['rustc', '--version'], ['cargo', '--version'], ['cc', '--version']):
    output = subprocess.check_output(tool, text=True)
    if tool[0] in {'rustc', 'cargo'}:
        assert output.startswith(tool[0] + ' 1.99.1 '), output
    print(output, end='')
print('PASS: isolated ancestors/home, allowlisted environment, read-only system, offline network')

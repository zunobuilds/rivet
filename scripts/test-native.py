"""Compile offline, then run the explicit native user-systemd qualifications.

The contained developer namespace intentionally has no user bus or host cgroups.
Only run this script under the owner's corresponding temporary-unit permission.
"""
import json
import pathlib
import subprocess

root = pathlib.Path(__file__).resolve().parent.parent
subprocess.run([str(root / 'scripts/cargo-contained'), 'offline', 'build', '--locked', '--offline'], cwd=root, check=True)
build = subprocess.run([
    str(root / 'scripts/cargo-contained'), 'offline', 'test', '--locked', '--offline',
    '--test', 'runtime', '--test', 'git_runtime', '--test', 'solo_runtime', '--test', 'mcp_runtime', '--test', 'recovery_runtime', '--lib', '--no-run', '--message-format=json',
], cwd=root, capture_output=True, text=True, check=True)
executables = {}
for line in build.stdout.splitlines():
    record = json.loads(line)
    if record.get('reason') == 'compiler-artifact' and record.get('executable'):
        name = record['target']['name']
        if name == 'rivet' and record['target']['kind'] == ['lib']:
            name = 'launch-tests'
        if name == 'launch-tests' or name in ['runtime', 'git_runtime', 'solo_runtime', 'mcp_runtime', 'recovery_runtime']:
            path = pathlib.Path(record['executable'])
            executables[name] = root / path.relative_to('/work')
assert set(executables) == {'runtime', 'git_runtime', 'solo_runtime', 'mcp_runtime', 'recovery_runtime', 'launch-tests'}
results = {}
for name, path in executables.items():
    result = subprocess.run([str(path), '--ignored', '--test-threads=1', '--nocapture'], cwd=root, capture_output=True, text=True, timeout=90)
    print(result.stdout, end='')
    print(result.stderr, end='')
    results[name] = {'exit_code': result.returncode, 'stdout': result.stdout, 'stderr': result.stderr}
    (root / '.build/native-evidence.json').write_text(json.dumps(results, indent=2) + '\n')
    assert result.returncode == 0, name

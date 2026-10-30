"""Verify the locked source archives and summarize metadata without network access."""
import hashlib
import json
import pathlib
import tomllib

root = pathlib.Path(__file__).resolve().parent.parent
lock = tomllib.loads((root / 'Cargo.lock').read_text())
metadata = json.loads((root / '.build/metadata.json').read_text())
packages = { (p['name'], p['version']): p for p in metadata['packages'] }
cache = root / '.build/cargo-home/registry/cache'
licenses = set()
maximum = (0,)
count = 0
for p in lock['package']:
    if p['name'] == 'rivet':
        continue
    assert p['source'] == 'registry+https://github.com/rust-lang/crates.io-index', p['source']
    archives = list(cache.glob(f"*/{p['name']}-{p['version']}.crate"))
    assert len(archives) == 1, p['name']
    assert hashlib.sha256(archives[0].read_bytes()).hexdigest() == p['checksum'], p['name']
    info = packages[(p['name'], p['version'])]
    assert info['license'], p['name']
    licenses.add(info['license'])
    version = info.get('rust_version')
    if version:
        version = tuple(map(int, version.split('.')))
        assert version <= (1, 99, 1), (p['name'], version)
        maximum = max(maximum, version)
    count += 1
print(f'PASS: {count} registry archives match Cargo.lock SHA-256 checksums; no git/private sources')
print('Maximum declared dependency Rust minimum:', '.'.join(map(str, maximum)))
print('Declared license expressions:')
for license_expression in sorted(licenses):
    print(' ', license_expression)
print('Direct resolved dependencies:')
project_version = tomllib.loads((root / 'Cargo.toml').read_text())['package']['version']
for d in packages[('rivet', project_version)]['dependencies']:
    candidates = [p for p in metadata['packages'] if p['name'] == d['name']]
    assert len(candidates) == 1
    p = candidates[0]
    print(p['name'], p['version'], p['license'], 'default_features='+str(d['uses_default_features']), 'features='+','.join(d['features']))

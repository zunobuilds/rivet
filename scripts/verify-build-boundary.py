"""Exercise the offline boundary from an actual Cargo build script and its child."""
import os
import pathlib
import subprocess

root = pathlib.Path(__file__).resolve().parent.parent
fixture = root / '.build/containment-fixture'
(fixture / 'src').mkdir(parents=True, exist_ok=True)
(fixture / 'Cargo.toml').write_text('[package]\nname="containment-probe"\nversion="0.0.0"\nedition="2024"\n[workspace]\n')
(fixture / 'src/lib.rs').write_text('// No dependencies or runtime behavior.\n')
(fixture / 'build.rs').write_text(r'''
use std::{env, path::Path, process::Command};
fn main() {
    assert!(env::var_os("RIVET_SENTINEL_CREDENTIAL").is_none());
    assert!(env::var_os("HOME").is_none());
    assert!(env::var_os("SSH_AUTH_SOCK").is_none());
    assert_eq!(env::var("CARGO_NET_OFFLINE").unwrap(), "true");
    assert!(env::var("OUT_DIR").unwrap().starts_with("/work/.build/"));
    assert!(!Path::new("/home").exists());
    assert!(!Path::new("/etc/passwd").exists());
    assert!(!Path::new("/.cargo").exists());
    assert_eq!(std::fs::read_dir("/work/.git").unwrap().count(), 0);
    assert!(std::fs::write("/usr/.rivet-containment-probe", "denied").is_err());
    assert!(std::net::TcpStream::connect_timeout(&"192.0.2.1:9".parse().unwrap(), std::time::Duration::from_millis(100)).is_err());
    assert!(Command::new("python3").arg("/work/scripts/check-containment.py").status().unwrap().success());
}
''')
# A fake sentinel tests scrubbing without inspecting any actual credential.
environment = os.environ.copy()
environment['RIVET_SENTINEL_CREDENTIAL'] = 'synthetic-do-not-inherit'
subprocess.run([str(root / 'scripts/cargo-contained'), 'offline', 'check', '--offline', '--manifest-path', '/work/.build/containment-fixture/Cargo.toml'], env=environment, check=True)
print('PASS: actual Cargo build script and child inherit filesystem/environment/network restrictions')

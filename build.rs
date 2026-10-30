// Bind tool approvals to this exact source/manifest build, independent of checkout location.
#[path = "src/digest.rs"]
mod digest;
use std::{fs, path::{Path, PathBuf}};
fn visit(path: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(path).expect("source directory") {
        let path = entry.expect("source entry").path();
        assert!(!fs::symlink_metadata(&path).expect("source metadata").file_type().is_symlink(), "linked source refused");
        if path.is_dir() { visit(&path, files); } else { files.push(path); }
    }
}
fn main() {
    let mut files = vec![PathBuf::from("build.rs"), PathBuf::from("Cargo.toml"), PathBuf::from("Cargo.lock")];
    visit(Path::new("src"), &mut files);
    files.sort();
    let mut bytes = Vec::new();
    for path in files {
        println!("cargo:rerun-if-changed={}", path.display());
        let content = fs::read(&path).expect("build input");
        bytes.extend_from_slice(path.to_str().expect("UTF-8 source path").as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(&(content.len() as u64).to_be_bytes());
        bytes.extend_from_slice(&content);
    }
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rustc-env=RIVET_BUILD_DIGEST={}", digest::sha256(&bytes));
}

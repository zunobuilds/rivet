//! One inspectable source-only recipe. It never installs dependencies or executes code.
use crate::{config::Project,error::{Error,Result}};
use cap_std::fs::{OpenOptions,OpenOptionsExt,Permissions,PermissionsExt};
use std::io::Write;
use serde::Serialize;
#[derive(Serialize)]
pub struct RecipeFile {pub path:&'static str,pub text:String}
pub fn recipe(name:&str)->Result<Vec<RecipeFile>> {
    if name.is_empty()||name.len()>48||!name.starts_with(|c:char|c.is_ascii_lowercase())||!name.chars().all(|c|c.is_ascii_lowercase()||c.is_ascii_digit()||c=='_'){return Err(Error::Invalid);}
    Ok(vec![RecipeFile{path:"Cargo.toml",text:format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\npublish = false\n\n[dependencies]\n")},RecipeFile{path:"src/main.rs",text:"fn main() {\n    println!(\"Hello, world!\");\n}\n".into()}])
}
pub fn create(project:&Project,name:&str,reviewed_digest:&str)->Result<()> {
    let files=recipe(name)?;let encoded=serde_json::to_vec(&files).map_err(|_|Error::Invalid)?;
    if crate::tools::sha256(&encoded)!=reviewed_digest{return Err(Error::Denied);}
    project.dir.create_dir(name)?;
    project.dir.set_permissions(name,Permissions::from_mode(0o700))?;
    let root=project.open_directory(std::path::Path::new(name))?;
    root.create_dir("src")?;
    // Newly created files only; partial failures remain inspectable, never overwrite a user project.
    for file in files {
        let mut options=OpenOptions::new();options.write(true).create_new(true).mode(0o600).custom_flags(0o400000);
        let mut output=if file.path=="src/main.rs"{root.open_dir("src")?.open_with("main.rs",&options)?.into_std()}else{root.open_with(file.path,&options)?.into_std()};
        output.write_all(file.text.as_bytes())?;output.sync_all()?;
    }
    project.open_directory(&std::path::Path::new(name).join("src"))?.into_std_file().sync_all()?;
    root.into_std_file().sync_all()?;
    let fd=rustix::fs::openat2(&project.dir,".",rustix::fs::OFlags::RDONLY|rustix::fs::OFlags::DIRECTORY|rustix::fs::OFlags::CLOEXEC,rustix::fs::Mode::empty(),rustix::fs::ResolveFlags::BENEATH|rustix::fs::ResolveFlags::NO_SYMLINKS|rustix::fs::ResolveFlags::NO_XDEV).map_err(|_|Error::State)?;
    std::fs::File::from(fd).sync_all()?;Ok(())
}
pub fn digest(files:&[RecipeFile])->Result<String>{serde_json::to_vec(files).map(|v|crate::tools::sha256(&v)).map_err(|_|Error::Invalid)}

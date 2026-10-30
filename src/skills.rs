//! Explicitly enabled, content-pinned text instructions; never an approval source.
use crate::error::{Error,Result};
use serde::{Serialize,Deserialize};
use std::{path::{PathBuf,Component},collections::BTreeSet};
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Skill {pub id:String,#[serde(default)]pub enabled:bool,pub path:PathBuf,pub expected_sha256:String}
pub fn validate(skills:&[Skill])->Result<()> {
    let mut ids=BTreeSet::new();if skills.iter().filter(|s|s.enabled).count()>4{return Err(Error::Unsupported);}
    for s in skills.iter().filter(|s|s.enabled){
        if s.id.is_empty()||s.id.len()>64||!s.id.chars().all(|c|c.is_ascii_alphanumeric()||"_-".contains(c))||!ids.insert(&s.id)
            ||s.expected_sha256.len()!=64||!s.expected_sha256.bytes().all(|b|b.is_ascii_hexdigit())||s.path.file_name().and_then(|s|s.to_str())!=Some("SKILL.md")||s.path.components().any(|c|!matches!(c,Component::Normal(_))){return Err(Error::Invalid);}
    }Ok(())
}

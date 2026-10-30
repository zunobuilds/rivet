pub mod git;
pub mod process;
pub mod files;
use crate::{domain::{ActorId, OperationId, TaskId}, error::{Error, Result}};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum EffectClass { ReadOnly, Inert, Mutation, External }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolDescriptor {
    pub id: String,
    pub version: u32,
    pub build_digest: String,
    pub input_schema: String,
    pub output_limit: u64,
    pub effect: EffectClass,
    pub permissions: Vec<String>,
    pub platform: String,
    pub cancellable: bool,
    pub reconcilable: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolProposal {
    pub schema_version: u32,
    pub operation: OperationId,
    pub attempt: u32,
    pub task: TaskId,
    pub actor: ActorId,
    pub descriptor_digest: String,
    pub arguments: serde_json::Value,
    pub resources: Vec<String>,
    pub preconditions: Vec<String>,
    pub budget_reservation: String,
}
impl ToolProposal {
    /// Stable SHA-256 of the typed, normalized proposal encoding.
    pub fn digest(&self) -> Result<String> { serde_json::to_vec(self).map(|bytes| sha256(&bytes)).map_err(|_| Error::Invalid) }
}
#[derive(Default)]
pub struct Registry { entries: HashMap<(String, u32), ToolDescriptor> }
impl Registry {
    pub fn register(&mut self, descriptor: ToolDescriptor) -> Result<()> {
        if !descriptor.id.split_once(':').is_some_and(|(namespace, name)| !namespace.is_empty() && !name.is_empty())
            || !descriptor.id.chars().all(|c| c.is_ascii_alphanumeric() || ":._-".contains(c))
            || descriptor.version == 0 || descriptor.build_digest.is_empty() { return Err(Error::Invalid); }
        let key = (descriptor.id.clone(), descriptor.version);
        if self.entries.contains_key(&key) { return Err(Error::Invalid); }
        self.entries.insert(key, descriptor); Ok(())
    }
    pub fn get(&self, id: &str, version: u32) -> Option<&ToolDescriptor> { self.entries.get(&(id.to_owned(), version)) }
}

pub(crate) use crate::digest::sha256;
#[cfg(test)]
mod digest_tests {
    use super::sha256;
    #[test]
    fn published_sha256_vectors_cover_padding_and_multiple_blocks() {
        assert_eq!(sha256(b""),"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(sha256(b"abc"),"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(sha256(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),"248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1");
    }
}

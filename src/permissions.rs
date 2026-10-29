use crate::{error::{Error, Result}, tools::ToolProposal};
use std::collections::HashMap;
use uuid::Uuid;

/// Fields are private; providers cannot mint or alter a grant.
#[derive(Debug)]
pub struct Grant { id: Uuid }
#[cfg(test)]
impl Grant { pub(crate) fn reference(&self) -> String { self.id.to_string() } }
#[derive(Debug)]
pub enum BrokerDecision { Deny(&'static str), Ask(String), Allow(Grant) }
struct Binding { digest: String, proposal: ToolProposal, epoch: u64, expiry: u64 }
#[derive(Default)]
pub struct Broker { epoch: u64, grants: HashMap<Uuid, Binding> }
impl Broker {
    pub fn new(epoch: u64) -> Self { Self { epoch, grants: HashMap::new() } }
    pub fn decide(&self, _proposal: &ToolProposal) -> BrokerDecision { BrokerDecision::Deny("no production executor in P1a") }
    pub fn revoke_all(&mut self) { self.epoch += 1; self.grants.clear(); }
    pub fn epoch(&self) -> u64 { self.epoch }
    pub fn consume(&mut self, grant: Grant, proposal: &ToolProposal, now: u64) -> Result<()> {
        let binding = self.grants.remove(&grant.id).ok_or(Error::Denied)?;
        if binding.epoch != self.epoch || binding.expiry <= now || binding.digest != proposal.digest()? || binding.proposal != *proposal { return Err(Error::Denied); }
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn approve_inert(&mut self, proposal: &ToolProposal, expiry: u64) -> Result<Grant> {
        if proposal.schema_version != 1 || proposal.descriptor_digest != "test:inert:v1" { return Err(Error::Denied); }
        let id = Uuid::new_v4();
        self.grants.insert(id, Binding { digest: proposal.digest()?, proposal: proposal.clone(), epoch: self.epoch, expiry });
        Ok(Grant { id })
    }
}

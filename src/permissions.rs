use crate::{error::{Error, Result}, tools::ToolProposal};
use std::collections::HashMap;
use uuid::Uuid;

/// Fields are private; providers cannot mint or alter a grant.
#[derive(Debug)]
pub struct Grant { id: Uuid, expiry: u64 }
impl Grant {
    pub(crate) fn reference(&self) -> String { self.id.to_string() }
    pub(crate) fn expiry(&self) -> u64 { self.expiry }
}
#[derive(Debug)]
pub enum BrokerDecision { Deny(&'static str), Ask(String), Allow(Grant) }
struct Binding { digest: String, proposal: ToolProposal, epoch: u64, expiry: u64 }
#[derive(Default)]
pub struct Broker { epoch: u64, grants: HashMap<Uuid, Binding>, revoked: tokio_util::sync::CancellationToken }
impl Broker {
    pub fn new(epoch: u64) -> Self { Self { epoch, grants: HashMap::new(), revoked: tokio_util::sync::CancellationToken::new() } }
    pub fn decide(&self, _proposal: &ToolProposal) -> BrokerDecision { BrokerDecision::Deny("explicit user approval required") }
    pub fn revoke_all(&mut self) { self.revoked.cancel(); self.epoch = self.epoch.saturating_add(1); self.grants.clear(); self.revoked=tokio_util::sync::CancellationToken::new(); }
    pub fn revocation(&self)->tokio_util::sync::CancellationToken {self.revoked.clone()}
    pub fn epoch(&self) -> u64 { self.epoch }
    pub fn consume(&mut self, grant: Grant, proposal: &ToolProposal, now: u64) -> Result<()> {
        let binding = self.grants.remove(&grant.id).ok_or(Error::Denied)?;
        if self.revoked.is_cancelled() || binding.epoch != self.epoch || binding.expiry <= now || binding.digest != proposal.digest()? || binding.proposal != *proposal { return Err(Error::Denied); }
        Ok(())
    }
    /// Called by trusted user-interface code with the exact scope the user reviewed.
    /// ProviderPort has no Broker handle and cannot manufacture this approval.
    pub fn approve_file(&mut self, proposal: &ToolProposal, reviewed_resources: &[String], expiry: u64, now: u64) -> Result<Grant> {
        if self.revoked.is_cancelled() || proposal.schema_version != 1 || proposal.attempt == 0
            || proposal.descriptor_digest != crate::tools::files::DESCRIPTOR
            || proposal.resources != reviewed_resources || reviewed_resources.is_empty()
            || expiry <= now || expiry.saturating_sub(now) > 3600 { return Err(Error::Denied); }
        let id = Uuid::new_v4();
        self.grants.insert(id, Binding { digest: proposal.digest()?, proposal: proposal.clone(), epoch: self.epoch, expiry });
        Ok(Grant { id, expiry })
    }
    /// Explicit trusted UI approval for an immutable isolated-check request.
    pub fn approve_process(&mut self, proposal:&ToolProposal, reviewed_resources:&[String], expiry:u64, now:u64)->Result<Grant> {
        if self.revoked.is_cancelled() || proposal.schema_version!=1 || proposal.attempt==0 || proposal.descriptor_digest!=crate::tools::process::DESCRIPTOR
            || proposal.resources!=reviewed_resources || reviewed_resources.is_empty() || expiry<=now || expiry.saturating_sub(now)>3600{return Err(Error::Denied);}
        let id=Uuid::new_v4();
        self.grants.insert(id,Binding{digest:proposal.digest()?,proposal:proposal.clone(),epoch:self.epoch,expiry});
        Ok(Grant{id,expiry})
    }
    pub fn approve_git(&mut self,proposal:&ToolProposal,reviewed_resources:&[String],expiry:u64,now:u64)->Result<Grant>{
        if self.revoked.is_cancelled()||proposal.schema_version!=1||proposal.attempt==0||proposal.descriptor_digest!=crate::tools::git::DESCRIPTOR||proposal.resources!=reviewed_resources||reviewed_resources.is_empty()||expiry<=now||expiry.saturating_sub(now)>3600{return Err(Error::Denied);}
        let id=Uuid::new_v4();self.grants.insert(id,Binding{digest:proposal.digest()?,proposal:proposal.clone(),epoch:self.epoch,expiry});Ok(Grant{id,expiry})
    }
    #[cfg(test)]
    pub(crate) fn approve_inert(&mut self, proposal: &ToolProposal, expiry: u64) -> Result<Grant> {
        if proposal.schema_version != 1 || proposal.descriptor_digest != "test:inert:v1" { return Err(Error::Denied); }
        let id = Uuid::new_v4();
        self.grants.insert(id, Binding { digest: proposal.digest()?, proposal: proposal.clone(), epoch: self.epoch, expiry });
        Ok(Grant { id, expiry })
    }
}

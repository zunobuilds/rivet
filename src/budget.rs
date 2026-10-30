//! Durable admission before a provider send; uncertainty never releases reservations.
use crate::{domain::{RequestId, RunId}, error::{Error, Result}, ledger::{Ledger, Transition}};
use serde::{Serialize, Deserialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BudgetLimits {pub tokens:u64, pub requests:u32, pub cost_micros:u64}
impl BudgetLimits {
    pub fn validate(&self)->Result<()> {if self.tokens==0 || self.tokens>10_000_000 || self.requests==0 || self.requests>100 || self.cost_micros>1_000_000_000{return Err(Error::Invalid);}Ok(())}
}
impl Ledger {
    pub async fn init_budget(&self,run:RunId,limits:BudgetLimits)->Result<()> {limits.validate()?;self.transition(0,Transition::InitBudget{run,limits}).await}
    pub async fn reserve_inference(&self,run:RunId,request:RequestId,tokens:u64,cost:u64,digest:String,permission_epoch:u64)->Result<()> {
        self.transition(0,Transition::ReserveInference{run,request,tokens,cost,digest,permission_epoch}).await
    }
    pub async fn settle_inference(&self,request:RequestId,usage:Option<(u64,u64)>)->Result<()> {
        self.transition(0,Transition::SettleInference{request,usage}).await
    }
}

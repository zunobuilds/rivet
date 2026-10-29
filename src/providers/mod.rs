use crate::domain::{RequestId, OperationId};
use crate::tools::ToolProposal;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Capabilities { pub text: bool, pub tool_calls: bool, pub usage: bool }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InferRequest { pub schema_version: u32, pub id: RequestId, pub prompt: String }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ProviderEvent {
    TextDelta { request: RequestId, text: String },
    ToolCallDelta { request: RequestId, call: OperationId, arguments: String },
    ToolCallComplete { request: RequestId, proposal: ToolProposal },
    Usage { request: RequestId, input: u64, output: u64 },
    Finish { request: RequestId, reason: String },
    Error { request: RequestId, code: String },
}
/// A provider receives no broker, dispatcher or ledger authority.
pub trait ProviderPort: Send + Sync {
    fn capabilities(&self) -> Capabilities;
    fn infer(&self, request: InferRequest, cancel: CancellationToken) -> mpsc::Receiver<ProviderEvent>;
}

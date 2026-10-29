use serde::Serialize;
#[derive(Debug, Serialize)]
pub struct Session { pub id: String, pub created_at: i64, pub config_version: u32 }

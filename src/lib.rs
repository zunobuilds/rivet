#![forbid(unsafe_code)]
#[cfg(not(target_os = "linux"))]
compile_error!("Rivet P1a is qualified for Linux only");
pub mod cli;
pub mod config;
pub mod domain;
pub mod error;
pub mod ledger;
pub mod permissions;
pub mod providers;
pub mod sessions;
pub mod sandbox;
pub mod supervisor;
pub mod tools;

mod digest;
pub mod budget;
pub mod context;
pub mod chat;
pub mod scaffold;
pub mod terminal;
pub mod mcp;
pub mod skills;
pub mod recovery;
pub mod artifacts;

pub mod lifecycle;

pub mod scheduler;

pub mod coordination;

mod agent;

pub mod hierarchy;

pub mod model_pool;

pub mod cookbook;

pub mod adaptive;

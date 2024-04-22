pub mod cache;
pub mod core;
pub mod error;
pub mod filter;
pub mod log;
pub mod message;
pub mod parse;
pub mod sender;
pub mod target;
pub mod upload;
pub mod watcher;

// this feature/module contains some experimental simulation, not related to the actual project
#[cfg(feature = "golem")]
pub mod golem;

pub use crate::core::get_log_dir;

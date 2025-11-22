// Core daemon library modules

pub mod config;
pub mod startup;
pub mod scan;
pub mod stable;
pub mod probe;
pub mod classify;
pub mod gates;
pub mod encode;
pub mod validate;
pub mod size_gate;
pub mod replace;
pub mod sidecars;
pub mod jobs;
pub mod daemon_loop;

// Re-export commonly used types
pub use config::DaemonConfig;
pub use jobs::{Job, JobStatus};
pub use daemon_loop::run_daemon_loop;

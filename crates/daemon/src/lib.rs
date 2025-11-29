// Core daemon library modules

pub mod classify;
pub mod config;
pub mod daemon_loop;
pub mod encode;
pub mod gates;
pub mod jobs;
pub mod probe;
pub mod replace;
pub mod scan;
pub mod sidecars;
pub mod size_gate;
pub mod stable;
pub mod startup;
pub mod validate;

// Re-export commonly used types
pub use config::DaemonConfig;
pub use daemon_loop::run_daemon_loop;
pub use jobs::{Job, JobStatus};

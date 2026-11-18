pub(crate) mod admin;
pub mod doctor;
pub mod health;
pub(crate) mod metrics;
pub(crate) mod model;
pub(crate) mod ports;
pub mod upgrade;

pub use admin::{SystemActor, SystemAdminUseCases};
pub use health::{CheckStatus, HealthCheck, Readiness, ReadinessThresholds, ReadinessUseCases};
pub use model::*;
pub use ports::*;

pub const WORKER_ROLE: &str = "worker";
pub const APHEL_VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) mod updates;

pub mod admin;
pub mod doctor;
pub mod health;
pub mod metrics;
pub mod model;
pub mod ports;
pub mod upgrade;

pub use admin::{SystemActor, SystemAdminUseCases};
pub use health::{CheckStatus, HealthCheck, Readiness, ReadinessThresholds, ReadinessUseCases};
pub use model::*;
pub use ports::*;

pub const WORKER_ROLE: &str = "worker";
pub const APHEL_VERSION: &str = env!("CARGO_PKG_VERSION");

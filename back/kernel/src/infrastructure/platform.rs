use std::sync::Arc;

use crate::application::secrets::SecretVault;
use crate::application::shared::{AuditLog, EventPublisher, JobQueue};
use crate::database::transaction::Tx;

pub type SharedEventPublisher = Arc<dyn EventPublisher<Tx>>;
pub type SharedAuditLog = Arc<dyn AuditLog<Tx>>;
pub type SharedJobQueue = Arc<dyn JobQueue<Tx>>;
pub type SharedSecretVault = Arc<dyn SecretVault<Tx>>;

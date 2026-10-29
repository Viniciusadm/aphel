use std::sync::Arc;

use crate::application::secrets::SecretVault;
use crate::application::shared::{AuditLog, EventPublisher, JobQueue};
use crate::database::transaction::MySqlTx;

pub type SharedEventPublisher = Arc<dyn EventPublisher<MySqlTx>>;
pub type SharedAuditLog = Arc<dyn AuditLog<MySqlTx>>;
pub type SharedJobQueue = Arc<dyn JobQueue<MySqlTx>>;
pub type SharedSecretVault = Arc<dyn SecretVault<MySqlTx>>;

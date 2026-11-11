mod audit;
mod check;
mod console;
mod control;
mod directory;
mod local;
mod provisioning;
mod router;
mod runtime;
mod scope;
mod tenant;
mod worker;

use std::sync::OnceLock;

pub use audit::{OperatorActor, record_operator_action};
pub use check::{CheckOutcome, IsolationReport, run_isolation_check};
pub use console::{
    Operator, create_operator, openapi as console_openapi, openapi_json as console_openapi_json,
};
pub use control::ControlPlane;
pub use directory::TenantDirectory;
pub use local::TenantLocal;
pub use provisioning::{
    MigrationSummary, TenantHost, TenantRegistration, TenantSummary, add_host, begin_deletion,
    create_tenant, delete_tenant, export_available, list_tenants, migrate_all, migrate_tenant,
    provision_tenant, register_tenant, remove_host, resume_tenant, set_primary_host,
    suspend_tenant, tenant_summary, validate_slug,
};
pub use router::{router, spawn_directory_refresh};
pub use runtime::{TenantRuntime, TenantRuntimes};
pub use scope::{bind, current, current_state, scope, spawn};
pub use tenant::{Tenant, TenantStatus, normalize_host};
pub use worker::run_workers;

use crate::config::Tenancy;
use crate::errors::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TenancyMode {
    Single,
    Multi,
}

impl From<&Tenancy> for TenancyMode {
    fn from(tenancy: &Tenancy) -> Self {
        match tenancy {
            Tenancy::Single => Self::Single,
            Tenancy::Multi(_) => Self::Multi,
        }
    }
}

static MODE: OnceLock<TenancyMode> = OnceLock::new();

const PROCESS_STATE_ISOLATED: bool = true;

pub fn install_mode(mode: TenancyMode) -> Result<(), AppError> {
    let installed = *MODE.get_or_init(|| mode);
    if installed == mode {
        Ok(())
    } else {
        Err(AppError::Configuration(format!(
            "the tenancy mode is already {installed:?} and cannot change to {mode:?}"
        )))
    }
}

pub fn mode() -> TenancyMode {
    MODE.get().copied().unwrap_or(TenancyMode::Single)
}

pub fn is_multi() -> bool {
    mode() == TenancyMode::Multi
}

pub fn ensure_multi_tenant_ready() -> Result<(), AppError> {
    if PROCESS_STATE_ISOLATED {
        Ok(())
    } else {
        Err(AppError::Configuration(
            "serving many tenants (CONTROL_DATABASE_URL) is not available in this build yet: process-wide state is not isolated per tenant. Use DATABASE_URL to serve a single organization".to_owned(),
        ))
    }
}

pub(crate) fn outside_scope() -> AppError {
    AppError::Internal("tenant state was reached outside a tenant scope".to_owned())
}

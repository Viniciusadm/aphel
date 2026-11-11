use std::any::Any;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use axum::Router;
use axum::extract::Request;
use axum::response::Response;
use sqlx::MySqlPool;
use tokio::sync::OnceCell;
use tokio::task::AbortHandle;
use tower::ServiceExt;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::infrastructure::organization_profile::spawn_config_refresh;
use crate::infrastructure::secrets::{EncryptionKeyring, build_secret_vault};
use crate::state::AppState;
use crate::tenancy::scope::scoped_response;
use crate::tenancy::{ControlPlane, Tenant, TenantDirectory, scope};

type Local = Arc<dyn Any + Send + Sync>;

pub struct TenantRuntime {
    tenant: Arc<Tenant>,
    state: OnceLock<AppState>,
    router: OnceLock<Router>,
    locals: RwLock<HashMap<u64, Local>>,
    tasks: Mutex<Vec<AbortHandle>>,
    stopped: AtomicBool,
}

impl TenantRuntime {
    pub(super) async fn start(
        control: &ControlPlane,
        config: &AppConfig,
        tenant: Arc<Tenant>,
        serves_http: bool,
    ) -> Result<Arc<Self>, AppError> {
        let pool = control.connect_tenant(&tenant).await?;
        let runtime = Arc::new(Self {
            tenant,
            state: OnceLock::new(),
            router: OnceLock::new(),
            locals: RwLock::new(HashMap::new()),
            tasks: Mutex::new(Vec::new()),
            stopped: AtomicBool::new(false),
        });
        match scope(
            runtime.clone(),
            runtime.initialize(pool, config, serves_http),
        )
        .await
        {
            Ok(()) => {
                tracing::info!(tenant = runtime.tenant.slug, "tenant runtime started");
                Ok(runtime)
            }
            Err(error) => {
                runtime.stop();
                Err(error)
            }
        }
    }

    async fn initialize(
        &self,
        pool: MySqlPool,
        config: &AppConfig,
        serves_http: bool,
    ) -> Result<(), AppError> {
        let secrets = build_secret_vault(&pool, &config.encryption)?;
        let state = AppState::new(Some(pool.clone()))
            .with_secrets(secrets)
            .with_encryption(EncryptionKeyring::from_config(&config.encryption)?)
            .with_outbound_http(config.outbound_http.clone())
            .with_mysql_platform()
            .with_organization_storage();
        state.organization_config.refresh().await;
        state.app.enabled().refresh(&pool).await?;
        let _ = self.state.set(state.clone());
        spawn_config_refresh(state.organization_config.clone());
        state.app.enabled().clone().spawn_refresh(pool);
        if serves_http {
            let _ = self.router.set(crate::api::tenant_router(state));
        }
        Ok(())
    }

    pub fn tenant(&self) -> &Tenant {
        &self.tenant
    }

    pub fn state(&self) -> Option<&AppState> {
        self.state.get()
    }

    pub(super) fn local<T: Send + Sync + 'static>(&self, id: u64, init: fn() -> T) -> Arc<T> {
        let found = self
            .locals
            .read()
            .ok()
            .and_then(|locals| locals.get(&id).cloned());
        let local = match found {
            Some(local) => local,
            None => {
                let mut locals = self
                    .locals
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                locals
                    .entry(id)
                    .or_insert_with(|| Arc::new(init()) as Local)
                    .clone()
            }
        };
        local.downcast::<T>().unwrap_or_else(|_| Arc::new(init()))
    }

    pub(super) fn track(&self, task: AbortHandle) {
        if self.stopped.load(Ordering::SeqCst) {
            task.abort();
            return;
        }
        let mut tasks = self
            .tasks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        tasks.retain(|task| !task.is_finished());
        tasks.push(task);
    }

    pub fn stop(&self) {
        if self.stopped.swap(true, Ordering::SeqCst) {
            return;
        }
        let tasks = std::mem::take(
            &mut *self
                .tasks
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        );
        for task in tasks {
            task.abort();
        }
        tracing::info!(tenant = self.tenant.slug, "tenant runtime stopped");
    }

    pub async fn serve(self: &Arc<Self>, request: Request) -> Response {
        let Some(router) = self.router.get().cloned() else {
            return crate::api::error::app_error(AppError::unavailable_code(
                "tenant_unavailable",
                "this organization is not available",
            ));
        };
        match scope(self.clone(), router.oneshot(request)).await {
            Ok(response) => scoped_response(self.clone(), response),
            Err(infallible) => match infallible {},
        }
    }
}

type Slot = Arc<OnceCell<Arc<TenantRuntime>>>;

pub struct TenantRuntimes {
    control: ControlPlane,
    config: AppConfig,
    slots: Mutex<HashMap<Uuid, (Arc<Tenant>, Slot)>>,
}

impl TenantRuntimes {
    pub fn new(control: ControlPlane, config: AppConfig) -> Arc<Self> {
        Arc::new(Self {
            control,
            config,
            slots: Mutex::new(HashMap::new()),
        })
    }

    pub async fn get(&self, tenant: &Arc<Tenant>) -> Result<Arc<TenantRuntime>, AppError> {
        if !tenant.status.serves_requests() {
            return Err(AppError::unavailable_code(
                "tenant_unavailable",
                "this organization is not available",
            ));
        }
        let slot = {
            let mut slots = self
                .slots
                .lock()
                .map_err(|_| AppError::Internal("tenant runtimes lock poisoned".to_owned()))?;
            let current = slots
                .entry(tenant.id)
                .or_insert_with(|| (tenant.clone(), Slot::default()));
            if !current.0.is_served_by(tenant) {
                let replaced = std::mem::replace(current, (tenant.clone(), Slot::default()));
                stop_slot(&replaced.1);
            }
            current.1.clone()
        };
        slot.get_or_try_init(|| {
            TenantRuntime::start(&self.control, &self.config, tenant.clone(), true)
        })
        .await
        .cloned()
    }

    pub fn retain(&self, directory: &TenantDirectory) {
        let Ok(mut slots) = self.slots.lock() else {
            return;
        };
        slots.retain(|id, (tenant, slot)| {
            let keep = directory
                .tenant(*id)
                .is_some_and(|current| tenant.is_served_by(&current));
            if !keep {
                stop_slot(slot);
            }
            keep
        });
    }

    pub fn clear(&self) {
        if let Ok(mut slots) = self.slots.lock() {
            for (_, slot) in slots.values() {
                stop_slot(slot);
            }
            slots.clear();
        }
    }
}

fn stop_slot(slot: &Slot) {
    if let Some(runtime) = slot.get() {
        runtime.stop();
    }
}

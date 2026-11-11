use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::Semaphore;
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::infrastructure::jobs::{Shutdown, ShutdownTrigger, shutdown_channel};
use crate::modules::WorkerEnv;
use crate::tenancy::{ControlPlane, Tenant, TenantDirectory, TenantRuntime, scope};
use crate::worker::WorkerRuntime;

const RECONCILE_INTERVAL: Duration = Duration::from_secs(5);
const RETRY_AFTER_FAILURE: Duration = Duration::from_secs(60);

struct RunningTenant {
    tenant: Arc<Tenant>,
    runtime: Arc<TenantRuntime>,
    trigger: ShutdownTrigger,
    task: JoinHandle<()>,
}

impl RunningTenant {
    fn stop(self) -> JoinHandle<()> {
        self.trigger.trigger();
        tracing::info!(tenant = self.tenant.slug, "stopping the tenant worker");
        tokio::spawn(async move {
            if let Err(join_error) = self.task.await {
                tracing::error!(
                    tenant = self.tenant.slug,
                    "tenant worker stopped unexpectedly: {join_error}"
                );
            }
            self.runtime.stop();
        })
    }
}

struct Supervisor {
    control: ControlPlane,
    config: AppConfig,
    directory: Arc<TenantDirectory>,
    slots: Arc<Semaphore>,
    running: HashMap<Uuid, RunningTenant>,
    failed: HashMap<Uuid, Instant>,
    stopping: Vec<JoinHandle<()>>,
}

pub async fn run_workers(
    control: ControlPlane,
    config: AppConfig,
    mut shutdown: Shutdown,
) -> Result<(), AppError> {
    let directory = TenantDirectory::load(control.pool().clone()).await?;
    let concurrency = config.worker.concurrency.max(1);
    tracing::info!(concurrency, "tenant workers started");
    let mut supervisor = Supervisor {
        control,
        config,
        directory,
        slots: Arc::new(Semaphore::new(concurrency)),
        running: HashMap::new(),
        failed: HashMap::new(),
        stopping: Vec::new(),
    };
    loop {
        supervisor.reconcile().await;
        tokio::select! {
            () = shutdown.wait() => break,
            () = tokio::time::sleep(RECONCILE_INTERVAL) => {}
        }
        if let Err(error) = supervisor.directory.refresh().await {
            tracing::warn!("could not refresh the tenant directory: {error}");
        }
    }
    supervisor.shutdown().await;
    tracing::info!("tenant workers stopped");
    Ok(())
}

impl Supervisor {
    async fn reconcile(&mut self) {
        let stale: Vec<Uuid> = self
            .running
            .iter()
            .filter(|(id, running)| {
                !self
                    .directory
                    .tenant(**id)
                    .is_some_and(|current| running.tenant.is_served_by(&current))
            })
            .map(|(id, _)| *id)
            .collect();
        for id in stale {
            if let Some(running) = self.running.remove(&id) {
                self.stopping.push(running.stop());
            }
        }
        self.stopping.retain(|task| !task.is_finished());

        let mut pending: Vec<Arc<Tenant>> = self
            .directory
            .tenants()
            .into_iter()
            .filter(|tenant| tenant.status.serves_requests())
            .filter(|tenant| !self.running.contains_key(&tenant.id))
            .filter(|tenant| {
                self.failed
                    .get(&tenant.id)
                    .is_none_or(|failed_at| failed_at.elapsed() >= RETRY_AFTER_FAILURE)
            })
            .collect();
        pending.sort_by(|left, right| left.slug.cmp(&right.slug));
        for tenant in pending {
            match self.start(tenant.clone()).await {
                Ok(running) => {
                    self.failed.remove(&tenant.id);
                    self.running.insert(tenant.id, running);
                }
                Err(error) => {
                    tracing::error!(
                        tenant = tenant.slug,
                        "could not start the tenant worker: {error}"
                    );
                    self.failed.insert(tenant.id, Instant::now());
                }
            }
        }
    }

    async fn start(&self, tenant: Arc<Tenant>) -> Result<RunningTenant, AppError> {
        let runtime =
            TenantRuntime::start(&self.control, &self.config, tenant.clone(), false).await?;
        let (trigger, shutdown) = shutdown_channel();
        let task = match scope(runtime.clone(), self.spawn_worker(&runtime, shutdown)).await {
            Ok(task) => task,
            Err(error) => {
                runtime.stop();
                return Err(error);
            }
        };
        tracing::info!(tenant = tenant.slug, "tenant worker started");
        Ok(RunningTenant {
            tenant,
            runtime,
            trigger,
            task,
        })
    }

    async fn spawn_worker(
        &self,
        runtime: &TenantRuntime,
        shutdown: Shutdown,
    ) -> Result<JoinHandle<()>, AppError> {
        let state = runtime
            .state()
            .ok_or_else(|| AppError::Internal("the tenant state is not ready".to_owned()))?;
        let env = WorkerEnv::from_state(state)?;
        let worker = WorkerRuntime::from_app(crate::modules::app(), &env, &self.config.worker)
            .with_shared_slots(self.slots.clone());
        let slug = runtime.tenant().slug.clone();
        Ok(crate::tenancy::spawn(async move {
            if let Err(error) = worker.run(shutdown).await {
                tracing::error!(
                    tenant = slug,
                    "tenant worker stopped with an error: {error}"
                );
            }
        }))
    }

    async fn shutdown(mut self) {
        let mut stopping = std::mem::take(&mut self.stopping);
        stopping.extend(self.running.drain().map(|(_, running)| running.stop()));
        for task in stopping {
            if let Err(join_error) = task.await {
                tracing::error!("tenant worker shutdown failed: {join_error}");
            }
        }
    }
}

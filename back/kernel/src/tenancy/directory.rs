use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use sqlx::MySqlPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::tenancy::{Tenant, TenantStatus, normalize_host};

#[derive(Default)]
struct Snapshot {
    hosts: HashMap<String, Arc<Tenant>>,
    tenants: HashMap<Uuid, Arc<Tenant>>,
}

pub struct TenantDirectory {
    pool: MySqlPool,
    snapshot: RwLock<Arc<Snapshot>>,
}

type HostRow = (String, String, String, String, String, Option<String>);

impl TenantDirectory {
    pub async fn load(pool: MySqlPool) -> Result<Arc<Self>, AppError> {
        let directory = Arc::new(Self {
            pool,
            snapshot: RwLock::new(Arc::default()),
        });
        directory.refresh().await?;
        Ok(directory)
    }

    pub async fn refresh(&self) -> Result<(), AppError> {
        let rows: Vec<HostRow> = sqlx::query_as(
            "SELECT t.id, t.slug, t.name, t.database_name, t.status, h.host \
             FROM tenants t LEFT JOIN tenant_hosts h ON h.tenant_id = t.id",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut snapshot = Snapshot::default();
        for (id, slug, name, database_name, status, host) in rows {
            let Some(tenant) = parse_tenant(&id, slug, name, database_name, &status) else {
                tracing::warn!(tenant = id, status, "skipping a tenant with an invalid row");
                continue;
            };
            let tenant = snapshot
                .tenants
                .entry(tenant.id)
                .or_insert_with(|| Arc::new(tenant))
                .clone();
            if let Some(host) = host.as_deref().and_then(normalize_host) {
                snapshot.hosts.insert(host, tenant);
            }
        }
        let mut current = self
            .snapshot
            .write()
            .map_err(|_| AppError::Internal("tenant directory lock poisoned".to_owned()))?;
        *current = Arc::new(snapshot);
        Ok(())
    }

    fn snapshot(&self) -> Arc<Snapshot> {
        self.snapshot
            .read()
            .map(|snapshot| snapshot.clone())
            .unwrap_or_default()
    }

    pub fn resolve(&self, host: &str) -> Option<Arc<Tenant>> {
        self.snapshot().hosts.get(host).cloned()
    }

    pub fn tenant(&self, id: Uuid) -> Option<Arc<Tenant>> {
        self.snapshot().tenants.get(&id).cloned()
    }

    pub fn tenants(&self) -> Vec<Arc<Tenant>> {
        self.snapshot().tenants.values().cloned().collect()
    }
}

fn parse_tenant(
    id: &str,
    slug: String,
    name: String,
    database_name: String,
    status: &str,
) -> Option<Tenant> {
    Some(Tenant {
        id: Uuid::parse_str(id).ok()?,
        slug,
        name,
        database_name,
        status: TenantStatus::parse(status)?,
    })
}

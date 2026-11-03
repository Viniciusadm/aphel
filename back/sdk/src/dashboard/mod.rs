pub use aphel_kernel::domain::period::{
    PeriodPreset, PeriodSelection, ResolvedPeriod, previous_period, resolve_period,
};
pub mod dto;
pub mod period;
pub mod ports;
pub mod provider;
pub mod subject;
use aphel_kernel::infrastructure::query::MySqlRecordQueryPorts;
pub use period::*;
pub use ports::*;
pub use provider::*;
use std::sync::Arc;
pub use subject::*;
pub struct WidgetPorts {
    pool: sqlx::MySqlPool,
    query: MySqlRecordQueryPorts,
    repository: Arc<dyn DashboardRepository>,
}
impl WidgetPorts {
    pub fn new(
        pool: sqlx::MySqlPool,
        query: MySqlRecordQueryPorts,
        repository: Arc<dyn DashboardRepository>,
    ) -> Self {
        Self {
            pool,
            query,
            repository,
        }
    }
}
impl DashboardPorts for WidgetPorts {
    fn pool(&self) -> &sqlx::MySqlPool {
        &self.pool
    }
    type Query = MySqlRecordQueryPorts;
    type Dashboard = dyn DashboardRepository;
    fn query(&self) -> &Self::Query {
        &self.query
    }
    fn dashboard(&self) -> &Self::Dashboard {
        self.repository.as_ref()
    }
}

pub mod source;
pub use source::*;

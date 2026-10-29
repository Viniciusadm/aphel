use async_trait::async_trait;

use crate::application::dashboard::SubjectDirectory;
use crate::application::query::{AggregatePorts, QueryPlan};
use crate::domain::access::AccessScope;
use crate::errors::AppError;

#[async_trait]
pub trait DashboardRepository: Send + Sync {
    async fn currency(&self) -> Result<String, AppError>;
    async fn count(&self, scope: &AccessScope, plan: &QueryPlan) -> Result<u64, AppError>;
    async fn subject_directory(&self) -> Result<SubjectDirectory, AppError>;
}

pub trait DashboardPorts: Send + Sync {
    type Query: AggregatePorts;
    type Dashboard: DashboardRepository;

    fn query(&self) -> &Self::Query;
    fn dashboard(&self) -> &Self::Dashboard;
}

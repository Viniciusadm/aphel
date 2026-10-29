pub mod dto;
pub(crate) mod http;
pub(crate) mod layout;
pub(crate) mod queries;
pub(crate) mod section;
pub(crate) mod service;
pub(crate) mod store;
pub(crate) mod widgets;

use std::sync::Arc;

use aphel_kernel::modules::Migration;

use crate::widgets::{
    DealsByStageWidget, PendingCenterWidget, RecentActivityWidget, ReportTotalsWidget,
    SalesIndicatorsWidget,
};
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::{APHEL_VERSION, DashboardWidget};
use aphel_kernel::modules::{AppBuilder, Manifest, Module};

pub const KEY: &str = "dashboard";

pub struct DashboardModule;

impl Module for DashboardModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Dashboard", APHEL_VERSION)
            .description(
                "Home dashboard: widget catalog and data, per-user layout and the organization default layout",
            )
            .enabled_by_default()
            .migration(Migration::new(
                1,
                "create dashboard layouts",
                include_str!("../migrations/0001_create_dashboard_layouts.sql"),
            ))
            .migration(Migration::new(
                2,
                "create dashboard default layouts",
                include_str!("../migrations/0002_create_dashboard_default_layouts.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute(std::sync::Arc::new(aphel_kernel::infrastructure::config_packages::ConfigSectionFactory::new(
            130,
            |pool, registry| {
                let _ = registry;
                std::sync::Arc::new(crate::section::DashboardSection::new(aphel_kernel::application::dashboard::WidgetRegistry::<aphel_kernel::infrastructure::repositories::mysql_dashboard::MySqlDashboardPorts>::builtin().providers().map(|provider| provider.key().to_owned()).collect(), crate::store::LayoutStore::new(pool.clone()), pipelines::store::PipelineStore::new(pool.clone()), aphel_kernel::infrastructure::config_packages::MySqlConfigKeyRepository::new(pool.clone())))
            },
        )));
        app.openapi_tag(
            http::TAG,
            "Dashboard widgets, their data and the per-user layout",
        )
        .routes(|_| http::router());
        let widgets: [DashboardWidget; 5] = [
            Arc::new(SalesIndicatorsWidget),
            Arc::new(PendingCenterWidget),
            Arc::new(RecentActivityWidget),
            Arc::new(ReportTotalsWidget),
            Arc::new(DealsByStageWidget),
        ];
        for widget in widgets {
            app.widget(widget);
        }
        Ok(())
    }
}

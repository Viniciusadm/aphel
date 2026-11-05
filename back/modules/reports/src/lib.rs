pub(crate) mod domain;
pub(crate) mod email;
pub(crate) mod exposed_operations;
pub(crate) mod http;
pub(crate) mod runtime;
pub(crate) mod service;
pub(crate) mod store;
pub(crate) mod table;
pub(crate) mod widget;

use std::sync::Arc;

use aphel_kernel::modules::Migration;

use crate::runtime::{
    ReportDeliveryHandler, ReportScheduleDispatchHandler, ReportsRuntime,
    report_schedules_dispatch_schedule,
};
use crate::widget::SavedReportWidget;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module, WorkerEnv, WorkerParts};

pub(crate) const KEY: &str = "reports";

pub struct ReportsModule;

impl Module for ReportsModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Reports", APHEL_VERSION)
            .description("Saved entity reports, exports and e-mail schedules")
            .enabled_by_default()
            .owned_tables(&[
                "report_deliveries",
                "report_schedule_recipients",
                "report_schedules",
                "reports",
            ])
            .migration(Migration::new(
                1,
                "create reports",
                include_str!("../migrations/0001_create_reports.sql"),
            ))
            .migration(Migration::new(
                2,
                "create report schedules",
                include_str!("../migrations/0002_create_report_schedules.sql"),
            ))
            .migration(Migration::new(
                3,
                "create report schedule recipients",
                include_str!("../migrations/0003_create_report_schedule_recipients.sql"),
            ))
            .migration(Migration::new(
                4,
                "create report deliveries",
                include_str!("../migrations/0004_create_report_deliveries.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute(std::sync::Arc::new(
            aphel_kernel::infrastructure::custom_objects::RecordTypeScopedTable("reports"),
        ));
        app.openapi_tag(
            http::custom::TAG,
            "Saved custom reports, their results, exports and e-mail schedules",
        )
        .routes(|_| http::custom::router())
        .contribute::<dyn aphel_sdk::dashboard::WidgetProvider<aphel_sdk::dashboard::WidgetPorts>>(
            Arc::new(SavedReportWidget),
        )
        .worker(worker);
        exposed_operations::register(app);
        Ok(())
    }
}

fn worker(env: &WorkerEnv) -> WorkerParts {
    let runtime = ReportsRuntime::new(env.pool.clone(), env.record_types.clone());
    WorkerParts::default()
        .handler(Arc::new(ReportScheduleDispatchHandler(runtime.clone())))
        .handler(Arc::new(ReportDeliveryHandler {
            runtime,
            mailer: env.mailer.clone(),
            public_url: env.public_url.clone(),
        }))
        .schedule(report_schedules_dispatch_schedule())
}

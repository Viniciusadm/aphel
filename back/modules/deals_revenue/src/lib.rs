mod calculator;
mod recurring;
pub mod widgets;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};
use std::sync::Arc;
pub const KEY: &str = "deals_revenue";
pub struct DealsRevenueModule;
impl Module for DealsRevenueModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(
            KEY,
            "Deal revenue",
            aphel_sdk::platform::modules::builder::APHEL_VERSION,
        )
        .description("Recurring revenue projections for deals")
        .enabled_by_default()
        .depends_on("deals", "*")
        .depends_on(aphel_sdk::line_items::KEY, "*")
    }
    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.provide::<dyn aphel_sdk::deals::RevenueSummary>(Arc::new(widgets::Summary));
        app.provide::<dyn aphel_sdk::deals::RevenueCalculator>(Arc::new(
            calculator::ItemsAmountCalculator,
        ))
        .contribute::<dyn aphel_sdk::dashboard::WidgetProvider<aphel_sdk::dashboard::WidgetPorts>>(
            Arc::new(widgets::RecurringRevenueWidget),
        );
        Ok(())
    }
}

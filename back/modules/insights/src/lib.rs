mod aggregate;
mod birthdays;
mod deals_by_source;
mod settings;
mod team_activity_ranking;

use std::sync::Arc;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{AppBuilder, Manifest, Module, SettingField, SettingKind};

pub(crate) use birthdays::{BIRTHDAY_FIELD_SETTING, BirthdaysWidget};
pub(crate) use deals_by_source::DealsBySourceWidget;
pub(crate) use team_activity_ranking::TeamActivityRankingWidget;

pub(crate) const KEY: &str = "insights";
pub(crate) const CATEGORY: &str = "insights";
pub(crate) const VERSION: &str = "0.1.0";

pub struct InsightsModule;

impl Module for InsightsModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Extra widgets", VERSION)
            .description(
                "Additional dashboard widgets: deals by source, team activity ranking and birthdays of the month",
            )
            .requires_aphel("0.1.0")
            .optional(aphel_sdk::deals::KEY)
            .optional("tasks")
            .setting(
                SettingField::new(
                    BIRTHDAY_FIELD_SETTING,
                    "Campo de aniversário",
                    SettingKind::Field {
                        record_type: "people.person",
                        field_types: vec!["date"],
                    },
                )
                .help("Campo de data dos contatos usado pelo widget de aniversariantes do mês"),
            )
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute::<dyn aphel_sdk::dashboard::WidgetProvider<aphel_sdk::dashboard::WidgetPorts>>(Arc::new(DealsBySourceWidget))
            .contribute::<dyn aphel_sdk::dashboard::WidgetProvider<aphel_sdk::dashboard::WidgetPorts>>(Arc::new(TeamActivityRankingWidget))
            .contribute::<dyn aphel_sdk::dashboard::WidgetProvider<aphel_sdk::dashboard::WidgetPorts>>(Arc::new(BirthdaysWidget));
        Ok(())
    }
}

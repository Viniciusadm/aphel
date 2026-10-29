use sqlx::{MySqlConnection, QueryBuilder};
use uuid::Uuid;

use aphel_kernel::errors::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DefaultActivityType {
    pub(crate) key: &'static str,
    pub(crate) name: &'static str,
    pub(crate) icon: &'static str,
    pub(crate) color: &'static str,
}

pub(crate) const DEFAULT_ACTIVITY_TYPES: [DefaultActivityType; 6] = [
    DefaultActivityType {
        key: "call",
        name: "Ligação",
        icon: "phone",
        color: "#1f6feb",
    },
    DefaultActivityType {
        key: "meeting",
        name: "Reunião",
        icon: "users",
        color: "#8250df",
    },
    DefaultActivityType {
        key: "email",
        name: "E-mail",
        icon: "mail",
        color: "#0969da",
    },
    DefaultActivityType {
        key: "message",
        name: "Mensagem",
        icon: "message-circle",
        color: "#1a7f37",
    },
    DefaultActivityType {
        key: "visit",
        name: "Visita",
        icon: "map-pin",
        color: "#bf8700",
    },
    DefaultActivityType {
        key: "task",
        name: "Tarefa",
        icon: "check-square",
        color: "#6e7781",
    },
];

pub(crate) const SEEDED_TABLE: &str = "activity_types";

pub(crate) async fn seed_activity_types(connection: &mut MySqlConnection) -> Result<(), AppError> {
    let mut builder = QueryBuilder::new(
        "INSERT IGNORE INTO activity_types (id, `key`, name, icon, color, is_system, is_active, position) ",
    );
    builder.push_values(
        DEFAULT_ACTIVITY_TYPES.iter().enumerate(),
        |mut values, (position, activity_type)| {
            values
                .push_bind(Uuid::now_v7().to_string())
                .push_bind(activity_type.key)
                .push_bind(activity_type.name)
                .push_bind(activity_type.icon)
                .push_bind(activity_type.color)
                .push_bind(true)
                .push_bind(true)
                .push_bind(position as i32);
        },
    );
    builder.build().execute(&mut *connection).await?;
    Ok(())
}

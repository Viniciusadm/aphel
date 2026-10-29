use sqlx::{MySqlConnection, QueryBuilder};
use uuid::Uuid;

use aphel_kernel::errors::AppError;

pub const STAGE_KIND_OPEN: &str = "open";
pub const STAGE_KIND_WON: &str = "won";
pub const STAGE_KIND_LOST: &str = "lost";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DefaultStage {
    pub(crate) name: &'static str,
    pub(crate) probability: u8,
    pub(crate) kind: &'static str,
}

pub(crate) const DEFAULT_PIPELINE_NAME: &str = "Funil de vendas";

pub(crate) const DEFAULT_STAGES: [DefaultStage; 6] = [
    DefaultStage {
        name: "Prospecção",
        probability: 10,
        kind: STAGE_KIND_OPEN,
    },
    DefaultStage {
        name: "Qualificação",
        probability: 25,
        kind: STAGE_KIND_OPEN,
    },
    DefaultStage {
        name: "Proposta",
        probability: 50,
        kind: STAGE_KIND_OPEN,
    },
    DefaultStage {
        name: "Negociação",
        probability: 75,
        kind: STAGE_KIND_OPEN,
    },
    DefaultStage {
        name: "Ganho",
        probability: 100,
        kind: STAGE_KIND_WON,
    },
    DefaultStage {
        name: "Perdido",
        probability: 0,
        kind: STAGE_KIND_LOST,
    },
];

pub(crate) const DEFAULT_LOST_REASONS: [&str; 6] = [
    "Preço",
    "Escolheu um concorrente",
    "Sem orçamento",
    "Sem resposta",
    "Momento inadequado",
    "Outro",
];

pub(crate) const SEEDED_TABLES: [&str; 2] = ["pipelines", "lost_reasons"];

pub(crate) async fn seed_pipeline_defaults(
    connection: &mut MySqlConnection,
) -> Result<(), AppError> {
    let has_pipeline: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pipelines)")
        .fetch_one(&mut *connection)
        .await?;
    if !has_pipeline {
        let pipeline_id = Uuid::now_v7().to_string();
        sqlx::query(
            "INSERT INTO pipelines (id, name, is_default, position, is_active) \
             VALUES (?, ?, TRUE, 0, TRUE)",
        )
        .bind(&pipeline_id)
        .bind(DEFAULT_PIPELINE_NAME)
        .execute(&mut *connection)
        .await?;
        let mut builder = QueryBuilder::new(
            "INSERT INTO pipeline_stages (id, pipeline_id, name, position, probability, kind, is_active) ",
        );
        builder.push_values(
            DEFAULT_STAGES.iter().enumerate(),
            |mut values, (position, stage)| {
                values
                    .push_bind(Uuid::now_v7().to_string())
                    .push_bind(pipeline_id.clone())
                    .push_bind(stage.name)
                    .push_bind(position as i32)
                    .push_bind(stage.probability)
                    .push_bind(stage.kind)
                    .push_bind(true);
            },
        );
        builder.build().execute(&mut *connection).await?;
    }
    let has_lost_reason: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM lost_reasons)")
        .fetch_one(&mut *connection)
        .await?;
    if !has_lost_reason {
        let mut builder =
            QueryBuilder::new("INSERT INTO lost_reasons (id, name, position, is_active) ");
        builder.push_values(
            DEFAULT_LOST_REASONS.iter().enumerate(),
            |mut values, (position, name)| {
                values
                    .push_bind(Uuid::now_v7().to_string())
                    .push_bind(*name)
                    .push_bind(position as i32)
                    .push_bind(true);
            },
        );
        builder.build().execute(&mut *connection).await?;
    }
    Ok(())
}

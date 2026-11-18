use aphel_sdk::favorites::{FavoriteViewSource, FavoriteViewTarget};
use aphel_sdk::{AppError, Ctx, shared::RecordType};
use async_trait::async_trait;
use sqlx::{MySql, QueryBuilder};
use std::collections::HashMap;
use uuid::Uuid;
fn push_ids(builder: &mut QueryBuilder<MySql>, ids: &[Uuid]) {
    builder.push(" IN (");
    let mut values = builder.separated(", ");
    for id in ids {
        values.push_bind(id.to_string());
    }
    builder.push(")");
}
pub(crate) struct SavedViewFavorites;
#[async_trait]
impl FavoriteViewSource for SavedViewFavorites {
    async fn visible(
        &self,
        ctx: &Ctx<'_>,
        ids: &[Uuid],
    ) -> Result<HashMap<Uuid, FavoriteViewTarget>, AppError> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut builder =
            QueryBuilder::<MySql>::new("SELECT id, record_type, name FROM saved_views WHERE ");
        builder.push("id");
        push_ids(&mut builder, ids);
        builder
            .push(" AND (visibility = 'organization' OR owner_id = ")
            .push_bind(ctx.scope.user_id().as_str().to_owned());
        if !ctx.scope.team_ids().is_empty() {
            builder.push(" OR (visibility = 'team' AND team_id IN (");
            let mut teams = builder.separated(", ");
            for team in ctx.scope.team_ids() {
                teams.push_bind(team.as_str().to_owned());
            }
            builder.push("))");
        }
        builder.push(")");
        let rows: Vec<(String, String, String)> =
            builder.build_query_as().fetch_all(ctx.pool()).await?;
        let mut views = HashMap::new();
        for (id, record_type, name) in rows {
            let record_type = RecordType::parse(record_type.clone())
                .map_err(|_| AppError::Internal("invalid stored view entity".into()))?;
            let Some(definition) = ctx.registry().get(&record_type) else {
                continue;
            };
            if !aphel_sdk::records::enabled(ctx, &record_type)
                || ctx.scope.filter_for(&definition.read_permission).is_none()
            {
                continue;
            }
            views.insert(
                Uuid::parse_str(&id)
                    .map_err(|_| AppError::Internal("invalid stored view identity".into()))?,
                FavoriteViewTarget { record_type, name },
            );
        }
        Ok(views)
    }
}

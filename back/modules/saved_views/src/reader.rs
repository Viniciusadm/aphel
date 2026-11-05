use crate::application::{SavedViewRepository, is_visible};
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::views::{SavedViewReader, SavedViewSnapshot};
use aphel_sdk::{AppError, Ctx};
use uuid::Uuid;

pub(crate) struct Reader;
#[async_trait::async_trait]
impl SavedViewReader for Reader {
    async fn visible(
        &self,
        ctx: &Ctx<'_>,
        id: Uuid,
    ) -> Result<Option<SavedViewSnapshot>, AppError> {
        let view = crate::infrastructure::MySqlSavedViewRepository::new(ctx.pool().clone())
            .find(id)
            .await?;
        let Some(view) = view.filter(|view| is_visible(ctx.scope, view)) else {
            return Ok(None);
        };
        let Some(definition) = ctx.registry().get(&view.record_type) else {
            return Ok(None);
        };
        if !aphel_sdk::records::enabled(ctx, &view.record_type)
            || ctx.scope.require(&definition.read_permission).is_err()
        {
            return Ok(None);
        }
        Ok(Some(SavedViewSnapshot {
            id: view.id,
            record_type: view.record_type,
            name: view.name,
            filter: view.filter,
            sort: view.sort,
            settings: view.settings,
        }))
    }
}

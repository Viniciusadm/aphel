use aphel_contract_site::{SiteEmbed, SiteEmbedContext, SiteEmbedOption, SiteEmbedSource};
use aphel_kernel::errors::AppError;
use aphel_kernel::state::AppState;
use async_trait::async_trait;
use uuid::Uuid;

use crate::{appearance, render, store};
use aphel_kernel::Ctx;
use aphel_sdk::platform::domain::access::AccessScope;

pub(crate) const EMBED_KIND: &str = "form";

pub(crate) struct FormEmbeds;

#[async_trait]
impl SiteEmbedSource for FormEmbeds {
    fn kind(&self) -> &'static str {
        EMBED_KIND
    }

    async fn options(&self, state: &AppState) -> Result<Vec<SiteEmbedOption>, AppError> {
        Ok(store::list_forms(&state.db()?)
            .await?
            .into_iter()
            .filter(|(form, _)| form.is_active)
            .map(|(form, _)| SiteEmbedOption {
                key: form.id.to_string(),
                label: form.name,
            })
            .collect())
    }

    async fn render(
        &self,
        state: &AppState,
        key: &str,
        _context: &SiteEmbedContext,
    ) -> Result<Option<SiteEmbed>, AppError> {
        let Ok(id) = Uuid::parse_str(key) else {
            return Ok(None);
        };
        let Some(form) = store::list_forms(&state.db()?)
            .await?
            .into_iter()
            .map(|(form, _)| form)
            .find(|form| form.id == id)
        else {
            return Ok(None);
        };
        if !form.is_active {
            return Ok(None);
        }
        let scope = AccessScope::system();
        let ctx = Ctx::from_state(state, &scope).await?;
        let branding = appearance::branding(&ctx).await.unwrap_or_default();
        Ok(Some(SiteEmbed {
            html: render::site_embed(&form, &branding),
            tags: Vec::new(),
        }))
    }
}

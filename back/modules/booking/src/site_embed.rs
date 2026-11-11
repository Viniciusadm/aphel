use aphel_contract_site::{SiteEmbed, SiteEmbedContext, SiteEmbedOption, SiteEmbedSource};
use aphel_kernel::errors::AppError;
use aphel_kernel::state::AppState;
use async_trait::async_trait;
use sqlx::Row;

use crate::store::BookingStore;

const EMBED_KIND: &str = "booking";
const MAX_OPTIONS: i64 = 200;

pub(crate) struct BookingEmbeds;

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[async_trait]
impl SiteEmbedSource for BookingEmbeds {
    fn kind(&self) -> &'static str {
        EMBED_KIND
    }

    async fn options(&self, state: &AppState) -> Result<Vec<SiteEmbedOption>, AppError> {
        let rows = sqlx::query(
            "SELECT slug, name FROM meeting_types WHERE is_active = TRUE ORDER BY name, id LIMIT ?",
        )
        .bind(MAX_OPTIONS)
        .fetch_all(&state.db()?)
        .await?;
        rows.iter()
            .map(|row| {
                Ok(SiteEmbedOption {
                    key: row.try_get("slug")?,
                    label: row.try_get("name")?,
                })
            })
            .collect()
    }

    async fn render(
        &self,
        state: &AppState,
        key: &str,
        context: &SiteEmbedContext,
    ) -> Result<Option<SiteEmbed>, AppError> {
        let Some(meeting) = BookingStore::new(state.db()?)
            .meeting_type_by_slug(key)
            .await?
            .filter(|meeting| meeting.is_active)
        else {
            return Ok(None);
        };
        let portuguese = context.locale.to_ascii_lowercase().starts_with("pt");
        let (minutes, action) = match portuguese {
            true => ("min", "Agendar horário"),
            false => ("min", "Book a time"),
        };
        let description = meeting
            .description
            .as_deref()
            .map(str::trim)
            .filter(|description| !description.is_empty())
            .map(|description| {
                format!(
                    "<p class=\"site-booking-description\">{}</p>",
                    escape(description)
                )
            })
            .unwrap_or_default();
        let html = format!(
            "<div class=\"site-booking\"><p class=\"site-booking-name\">{name}</p><p class=\"site-booking-duration\">{duration} {minutes}</p>{description}<p><a class=\"site-booking-button\" href=\"{mount}/book/{slug}\">{action}</a></p></div>",
            name = escape(&meeting.name),
            duration = meeting.duration_minutes,
            mount = escape(&context.mount),
            slug = escape(&meeting.slug),
        );
        Ok(Some(SiteEmbed {
            html,
            tags: Vec::new(),
        }))
    }
}

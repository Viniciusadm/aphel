use async_trait::async_trait;
use serde_json::{Value, json};

use crate::settings::{SiteLink, site_settings};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::organization_profile::{
    OrganizationConfigUseCases, SecretChange, UpdateOrganizationConfig,
};
use aphel_sdk::platform::infrastructure::organization_profile::MySqlOrganizationConfigPorts;
use aphel_sdk::provisioning::{ProvisionAction, ProvisionOutcome, ProvisionRequest, Provisioner};

pub(crate) const LINK_KIND: &str = "site.link";
const SETTINGS_KEY: &str = "site";

pub(crate) struct LinkProvisioner;

fn link_of(request: &ProvisionRequest) -> Result<SiteLink, AppError> {
    serde_json::from_value::<SiteLink>(request.data.clone()).map_err(|_| {
        AppError::validation_code(
            "plugin_pack_invalid",
            format!("{}: a site link needs id, label and url", request.key),
        )
    })
}

fn applied(link: &SiteLink) -> Value {
    json!({ "id": link.id, "label": link.label, "url": link.url })
}

async fn save(ctx: &Ctx<'_>, links: Vec<SiteLink>) -> Result<(), AppError> {
    let state = aphel_sdk::runtime()?;
    let state = state.as_ref();
    let mut settings = site_settings(&state.organization_config.snapshot());
    settings.links = links;
    let value = serde_json::to_value(&settings)
        .map_err(|error| AppError::Internal(format!("site settings: {error}")))?;
    OrganizationConfigUseCases::new(MySqlOrganizationConfigPorts::new(
        state.db()?,
        state.secrets.clone(),
        state.events.clone(),
        state.audit.clone(),
        state.organization_config.clone(),
    ))
    .update(
        ctx.scope,
        UpdateOrganizationConfig {
            public_base_url: None,
            app_base_path: None,
            site_base_path: None,
            site_host: None,
            max_upload_mb: None,
            storage: None,
            storage_secret: SecretChange::Keep,
            mail: None,
            mail_secret: SecretChange::Keep,
            api_tokens: None,
            extensions: [(SETTINGS_KEY.to_owned(), value)].into_iter().collect(),
            ip: None,
        },
    )
    .await?;
    Ok(())
}

#[async_trait]
impl Provisioner for LinkProvisioner {
    fn kind(&self) -> &'static str {
        LINK_KIND
    }

    async fn apply(
        &self,
        ctx: &Ctx<'_>,
        request: &ProvisionRequest,
    ) -> Result<ProvisionOutcome, AppError> {
        let wanted = link_of(request)?;
        let state = aphel_sdk::runtime()?;
        let mut links = site_settings(&state.as_ref().organization_config.snapshot()).links;
        let position = links.iter().position(|link| link.id == wanted.id);
        if let Some(existing) = &request.existing {
            let Some(position) = position else {
                return Ok(ProvisionOutcome {
                    action: ProvisionAction::Removed,
                    id: existing.id.clone(),
                    applied: existing.applied.clone(),
                });
            };
            if applied(&links[position]) != existing.applied {
                return Ok(ProvisionOutcome {
                    action: ProvisionAction::UserChanged,
                    id: existing.id.clone(),
                    applied: existing.applied.clone(),
                });
            }
            links[position] = wanted.clone();
            save(ctx, links).await?;
            return Ok(ProvisionOutcome {
                action: ProvisionAction::Updated,
                id: wanted.id.clone(),
                applied: applied(&wanted),
            });
        }
        if let Some(position) = position {
            return Ok(ProvisionOutcome {
                action: ProvisionAction::Kept,
                id: wanted.id.clone(),
                applied: applied(&links[position]),
            });
        }
        links.push(wanted.clone());
        save(ctx, links).await?;
        Ok(ProvisionOutcome {
            action: ProvisionAction::Created,
            id: wanted.id.clone(),
            applied: applied(&wanted),
        })
    }
}

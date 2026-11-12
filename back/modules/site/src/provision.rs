use async_trait::async_trait;
use serde_json::{Value, json};

use crate::settings::{NavigationItem, site_settings};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::organization_profile::{
    OrganizationConfigUseCases, SecretChange, UpdateOrganizationConfig,
};
use aphel_sdk::platform::infrastructure::organization_profile::MySqlOrganizationConfigPorts;
use aphel_sdk::provisioning::{ProvisionAction, ProvisionOutcome, ProvisionRequest, Provisioner};

pub(crate) const LINK_KIND: &str = "site.link";
pub(crate) const HOME_KIND: &str = "site.home";
const SETTINGS_KEY: &str = "site";

pub(crate) struct LinkProvisioner;

fn link_of(request: &ProvisionRequest) -> Result<NavigationItem, AppError> {
    serde_json::from_value::<NavigationItem>(request.data.clone())
        .ok()
        .filter(|item| item.target.is_some() != item.url.is_some() && item.children.is_empty())
        .ok_or_else(|| {
            AppError::validation_code(
                "plugin_pack_invalid",
                format!(
                    "{}: a site link needs an id, a label and either a target or a url",
                    request.key
                ),
            )
        })
}

fn applied(link: &NavigationItem) -> Value {
    json!({ "id": link.id, "label": link.label, "url": link.url, "target": link.target })
}

async fn save(ctx: &Ctx<'_>, links: Vec<NavigationItem>) -> Result<(), AppError> {
    let state = aphel_sdk::runtime()?;
    let mut settings = site_settings(&state.as_ref().organization_config.snapshot());
    settings.navigation = links;
    save_settings(ctx.scope, settings).await
}

pub(crate) async fn save_settings(
    scope: &aphel_sdk::platform::domain::access::AccessScope,
    settings: crate::settings::SiteSettings,
) -> Result<(), AppError> {
    let state = aphel_sdk::runtime()?;
    let state = state.as_ref();
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
        scope,
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
    crate::cache::invalidate(&state.db()?, &[crate::cache::ALL_TAG.to_owned()]).await?;
    crate::cache::forget_all();
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
        let mut links = site_settings(&state.as_ref().organization_config.snapshot()).navigation;
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

pub(crate) struct HomeProvisioner;

fn home_of(request: &ProvisionRequest) -> Result<String, AppError> {
    request
        .data
        .get("target")
        .and_then(Value::as_str)
        .filter(|target| crate::targets::Target::parse(target).is_some())
        .map(str::to_owned)
        .ok_or_else(|| {
            AppError::validation_code(
                "plugin_pack_invalid",
                format!("{}: a site home needs a target", request.key),
            )
        })
}

#[async_trait]
impl Provisioner for HomeProvisioner {
    fn kind(&self) -> &'static str {
        HOME_KIND
    }

    async fn apply(
        &self,
        ctx: &Ctx<'_>,
        request: &ProvisionRequest,
    ) -> Result<ProvisionOutcome, AppError> {
        let wanted = home_of(request)?;
        let state = aphel_sdk::runtime()?;
        let mut settings = site_settings(&state.as_ref().organization_config.snapshot());
        let current = settings.home.clone();
        if let Some(existing) = &request.existing {
            let action = match current.as_deref() == existing.applied["target"].as_str() {
                true => ProvisionAction::Kept,
                false => ProvisionAction::UserChanged,
            };
            return Ok(ProvisionOutcome {
                action,
                id: existing.id.clone(),
                applied: existing.applied.clone(),
            });
        }
        if let Some(current) = current {
            return Ok(ProvisionOutcome {
                action: ProvisionAction::Kept,
                id: request.key.clone(),
                applied: json!({ "target": current }),
            });
        }
        settings.home = Some(wanted.clone());
        save_settings(ctx.scope, settings).await?;
        Ok(ProvisionOutcome {
            action: ProvisionAction::Created,
            id: request.key.clone(),
            applied: json!({ "target": wanted }),
        })
    }
}

use crate::domain::{CompanyData, CompanyView, company_not_found, company_snapshot};
use crate::hierarchy::{HierarchyStore, Reparent, blocked_parents, reparent_children};
use crate::store::CompanyStore;
use aphel_contract_crm::CrmPermissions;
use aphel_kernel::records::{Address, permission_denied};
use aphel_sdk::merges::{MergeOwner, PreparedMerge, use_duplicate};
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::access::{Permission, RecordScopeFilter};
use aphel_sdk::{AppError, Ctx};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::BTreeMap;
use uuid::Uuid;
pub(crate) const COMPANY_MERGE_KEYS: &[&str] = &[
    "name",
    "domain",
    "website",
    "industry",
    "size_range",
    "phone",
    "email",
    "owner_id",
    "address_street",
    "address_number",
    "address_complement",
    "address_district",
    "address_city",
    "address_state",
    "address_postal_code",
    "address_country",
    "description",
    "source",
];
fn address_slot<'a>(address: &'a mut Address, key: &str) -> Option<&'a mut Option<String>> {
    Some(match key {
        "address_street" => &mut address.street,
        "address_number" => &mut address.number,
        "address_complement" => &mut address.complement,
        "address_district" => &mut address.district,
        "address_city" => &mut address.city,
        "address_state" => &mut address.state,
        "address_postal_code" => &mut address.postal_code,
        "address_country" => &mut address.country,
        _ => return None,
    })
}

fn address_value<'a>(address: &'a Address, key: &str) -> Option<&'a Option<String>> {
    Some(match key {
        "address_street" => &address.street,
        "address_number" => &address.number,
        "address_complement" => &address.complement,
        "address_district" => &address.district,
        "address_city" => &address.city,
        "address_state" => &address.state,
        "address_postal_code" => &address.postal_code,
        "address_country" => &address.country,
        _ => return None,
    })
}

fn blank(value: &Option<String>) -> bool {
    value.as_deref().is_none_or(|text| text.trim().is_empty())
}

pub(crate) fn company_field_is_empty(data: &CompanyData, key: &str) -> bool {
    match key {
        "name" => data.name.trim().is_empty(),
        "domain" => blank(&data.domain),
        "website" => blank(&data.website),
        "industry" => blank(&data.industry),
        "size_range" => blank(&data.size_range),
        "phone" => blank(&data.phone),
        "email" => blank(&data.email),
        "owner_id" => data.owner_id.is_none(),
        "description" => blank(&data.description),
        "source" => blank(&data.source),
        other => address_value(&data.address, other).is_none_or(blank),
    }
}

pub(crate) fn copy_company_field(target: &mut CompanyData, source: &CompanyData, key: &str) {
    match key {
        "name" => target.name = source.name.clone(),
        "domain" => target.domain = source.domain.clone(),
        "website" => target.website = source.website.clone(),
        "industry" => target.industry = source.industry.clone(),
        "size_range" => target.size_range = source.size_range.clone(),
        "phone" => target.phone = source.phone.clone(),
        "email" => target.email = source.email.clone(),
        "owner_id" => target.owner_id = source.owner_id.clone(),
        "description" => target.description = source.description.clone(),
        "source" => target.source = source.source.clone(),
        other => {
            if let (Some(slot), Some(value)) = (
                address_slot(&mut target.address, other),
                address_value(&source.address, other),
            ) {
                *slot = value.clone();
            }
        }
    }
}
async fn load(
    ctx: &Ctx<'_>,
    organizations: &CompanyStore,
    id: Uuid,
    permission: &Permission,
) -> Result<CompanyView, AppError> {
    let scope = ctx.scope;
    let read = scope.record_filter(&Permission::COMPANIES_READ)?;
    let view = organizations
        .find(scope, &read, id)
        .await?
        .ok_or_else(company_not_found)?;
    let allowed = scope.record_filter(permission)?;
    if allowed != RecordScopeFilter::Organization
        && organizations.find(scope, &allowed, id).await?.is_none()
    {
        return Err(permission_denied());
    }
    Ok(view)
}

pub(crate) struct Owner;
struct Prepared {
    primary_id: Uuid,
    duplicate_id: Uuid,
    primary: CompanyView,
    duplicate: CompanyView,
    merged: CompanyData,
}
#[async_trait::async_trait]
impl MergeOwner for Owner {
    fn record_type(&self) -> &'static str {
        aphel_sdk::organizations::RECORD_TYPE
    }
    fn not_found(&self) -> AppError {
        company_not_found()
    }
    fn fields(&self) -> &'static [&'static str] {
        COMPANY_MERGE_KEYS
    }
    async fn prepare(
        &self,
        ctx: &Ctx<'_>,
        primary_id: Uuid,
        duplicate_id: Uuid,
        choices: &BTreeMap<String, bool>,
    ) -> Result<Box<dyn PreparedMerge>, AppError> {
        let store = CompanyStore::of(ctx);
        let primary = load(ctx, &store, primary_id, &Permission::COMPANIES_UPDATE).await?;
        let duplicate = load(ctx, &store, duplicate_id, &Permission::COMPANIES_DELETE).await?;
        let old = &primary.company.data;
        let source = &duplicate.company.data;
        let mut merged = old.clone();
        for key in COMPANY_MERGE_KEYS {
            if use_duplicate(
                choices,
                key,
                company_field_is_empty(old, key),
                company_field_is_empty(source, key),
            ) {
                copy_company_field(&mut merged, source, key);
            }
        }
        if merged.name.trim().is_empty() {
            merged.name = old.name.clone();
        }
        Ok(Box::new(Prepared {
            primary_id,
            duplicate_id,
            primary,
            duplicate,
            merged,
        }))
    }
    async fn retire(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        duplicate: Uuid,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        CompanyStore::of(ctx).soft_delete(tx, duplicate, now).await
    }
}
#[async_trait::async_trait]
impl PreparedMerge for Prepared {
    fn before(&self) -> Value {
        company_snapshot(&self.primary.company.data)
    }
    fn duplicate_name(&self) -> String {
        self.duplicate.company.data.name.clone()
    }
    async fn apply(
        &mut self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        now: DateTime<Utc>,
        ip: Option<String>,
    ) -> Result<Value, AppError> {
        let hierarchy = HierarchyStore::of(ctx);
        let blocked =
            blocked_parents(&hierarchy, tx, &[self.primary_id, self.duplicate_id]).await?;
        self.merged.parent_company_id = [
            self.merged.parent_company_id,
            self.primary.company.data.parent_company_id,
            self.duplicate.company.data.parent_company_id,
        ]
        .into_iter()
        .find(|candidate| candidate.is_none_or(|id| !blocked.contains(&id)))
        .flatten();

        reparent_children(
            &hierarchy,
            ctx.events(),
            ctx.audit(),
            tx,
            ctx.scope,
            Reparent {
                from_parent: self.duplicate_id,
                to_parent: Some(self.primary_id),
                except: Some(self.primary_id),
                now,
                ip,
            },
        )
        .await?;
        Ok(company_snapshot(&self.merged))
    }
    async fn persist(
        &mut self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        let store = CompanyStore::of(ctx);
        if self.merged != self.primary.company.data {
            store.update(tx, self.primary_id, &self.merged, now).await?;
        }
        Ok(())
    }
}

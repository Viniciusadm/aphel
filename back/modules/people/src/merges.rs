use crate::domain::NewContactCompanyLink;
use crate::domain::{ContactData, ContactView, contact_not_found, contact_snapshot};
use crate::store::ContactStore;
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
pub(crate) const CONTACT_MERGE_KEYS: &[&str] = &[
    "first_name",
    "last_name",
    "email",
    "phone",
    "mobile",
    "job_title",
    "company_id",
    "owner_id",
    "address_street",
    "address_number",
    "address_complement",
    "address_district",
    "address_city",
    "address_state",
    "address_postal_code",
    "address_country",
    "source",
    "notes",
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

pub(crate) fn contact_field_is_empty(data: &ContactData, key: &str) -> bool {
    match key {
        "first_name" => data.first_name.trim().is_empty(),
        "last_name" => blank(&data.last_name),
        "email" => blank(&data.email),
        "phone" => blank(&data.phone),
        "mobile" => blank(&data.mobile),
        "job_title" => blank(&data.job_title),
        "company_id" => data.company_id.is_none(),
        "owner_id" => data.owner_id.is_none(),
        "source" => blank(&data.source),
        "notes" => blank(&data.notes),
        other => address_value(&data.address, other).is_none_or(blank),
    }
}

pub(crate) fn copy_contact_field(target: &mut ContactData, source: &ContactData, key: &str) {
    match key {
        "first_name" => target.first_name = source.first_name.clone(),
        "last_name" => target.last_name = source.last_name.clone(),
        "email" => target.email = source.email.clone(),
        "phone" => target.phone = source.phone.clone(),
        "mobile" => target.mobile = source.mobile.clone(),
        "job_title" => target.job_title = source.job_title.clone(),
        "company_id" => target.company_id = source.company_id,
        "owner_id" => target.owner_id = source.owner_id.clone(),
        "source" => target.source = source.source.clone(),
        "notes" => target.notes = source.notes.clone(),
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
    people: &ContactStore<'_>,
    id: Uuid,
    permission: &Permission,
) -> Result<ContactView, AppError> {
    let scope = ctx.scope;
    let read = scope.record_filter(&Permission::CONTACTS_READ)?;
    let view = people
        .find(scope, &read, id)
        .await?
        .ok_or_else(contact_not_found)?;
    let allowed = scope.record_filter(permission)?;
    if allowed != RecordScopeFilter::Organization
        && people.find(scope, &allowed, id).await?.is_none()
    {
        return Err(permission_denied());
    }
    Ok(view)
}

pub(crate) struct Owner;
struct Prepared {
    primary_id: Uuid,
    primary: ContactView,
    duplicate: ContactView,
    merged: ContactData,
    extra_companies: Vec<Uuid>,
}
#[async_trait::async_trait]
impl MergeOwner for Owner {
    fn record_type(&self) -> &'static str {
        aphel_sdk::people::RECORD_TYPE
    }
    fn not_found(&self) -> AppError {
        contact_not_found()
    }
    fn fields(&self) -> &'static [&'static str] {
        CONTACT_MERGE_KEYS
    }
    async fn prepare(
        &self,
        ctx: &Ctx<'_>,
        primary_id: Uuid,
        duplicate_id: Uuid,
        choices: &BTreeMap<String, bool>,
    ) -> Result<Box<dyn PreparedMerge>, AppError> {
        let store = ContactStore::of(ctx);
        let primary = load(ctx, &store, primary_id, &Permission::CONTACTS_UPDATE).await?;
        let duplicate = load(ctx, &store, duplicate_id, &Permission::CONTACTS_DELETE).await?;
        let old = &primary.contact.data;
        let source = &duplicate.contact.data;
        let mut merged = old.clone();
        for key in CONTACT_MERGE_KEYS {
            if use_duplicate(
                choices,
                key,
                contact_field_is_empty(old, key),
                contact_field_is_empty(source, key),
            ) {
                copy_contact_field(&mut merged, source, key);
            }
        }
        if merged.first_name.trim().is_empty() {
            merged.first_name = old.first_name.clone();
        }
        let mut extra_companies = Vec::new();
        for company_id in [old.company_id, source.company_id].into_iter().flatten() {
            if merged.company_id != Some(company_id)
                && !extra_companies.contains(&company_id)
                && store.find_link(primary_id, company_id).await?.is_none()
            {
                extra_companies.push(company_id);
            }
        }

        Ok(Box::new(Prepared {
            primary_id,
            primary,
            duplicate,
            merged,
            extra_companies,
        }))
    }
    async fn retire(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        duplicate: Uuid,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        ContactStore::of(ctx).soft_delete(tx, duplicate, now).await
    }
}
#[async_trait::async_trait]
impl PreparedMerge for Prepared {
    fn before(&self) -> Value {
        contact_snapshot(&self.primary.contact.data)
    }
    fn duplicate_name(&self) -> String {
        self.duplicate.contact.full_name()
    }
    async fn apply(
        &mut self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        now: DateTime<Utc>,
        ip: Option<String>,
    ) -> Result<Value, AppError> {
        let _ = (ctx, tx, now, ip);
        Ok(contact_snapshot(&self.merged))
    }
    async fn persist(
        &mut self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        let store = ContactStore::of(ctx);
        if self.merged != self.primary.contact.data {
            store.update(tx, self.primary_id, &self.merged, now).await?;
        }
        for company_id in &self.extra_companies {
            store
                .insert_link(
                    tx,
                    &NewContactCompanyLink {
                        id: Uuid::now_v7(),
                        contact_id: self.primary_id,
                        company_id: *company_id,
                        role: None,
                        created_at: now,
                    },
                )
                .await?;
        }
        Ok(())
    }
}

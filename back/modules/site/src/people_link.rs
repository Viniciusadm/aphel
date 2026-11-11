use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::people::{KEY as PEOPLE_KEY, PersonCreation, RECORD_TYPE as PERSON_RECORD_TYPE};
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::application::access::{AccessRepository, build_access_scope};
use aphel_sdk::platform::application::organization_admin::ListWindow;
use aphel_sdk::platform::application::query::{Filter, FilterOperator, RecordQuery};
use aphel_sdk::platform::domain::access::{AccessScope, PermissionSet, RoleKey};
use aphel_sdk::platform::infrastructure::repositories::mysql_access::MySqlAccessRepository;
use serde_json::{Map, Value, json};

use crate::customers::customer_scope;
use crate::settings::site_settings;

fn split_name(name: &str) -> (String, Option<String>) {
    let mut words = name.split_whitespace();
    let first = words.next().unwrap_or_default().to_owned();
    let rest = words.collect::<Vec<_>>().join(" ");
    (first, (!rest.is_empty()).then_some(rest))
}

async fn find_person(ctx: &Ctx<'_>, email: &str) -> Result<Option<String>, AppError> {
    let query = RecordQuery {
        filter: Some(Filter::condition(
            "email",
            FilterOperator::Eq,
            json!(email.to_lowercase()),
        )),
        window: ListWindow {
            offset: 0,
            limit: 1,
        },
        ..RecordQuery::default()
    };
    Ok(
        aphel_sdk::record_context::list(ctx, PERSON_RECORD_TYPE, query)
            .await?
            .items
            .first()
            .and_then(|record| record.get("id"))
            .and_then(Value::as_str)
            .map(str::to_owned),
    )
}

async fn owner_scope(pool: &sqlx::MySqlPool, owner: &str) -> Result<Option<AccessScope>, AppError> {
    let Ok(user_id) = UserId::new(owner.to_owned()) else {
        return Ok(None);
    };
    Ok(MySqlAccessRepository::new(pool.clone())
        .load_access(&user_id)
        .await?
        .map(|access| {
            let scope = build_access_scope(user_id.clone(), access);
            AccessScope::new(
                user_id,
                RoleKey::system(),
                scope.team_ids().to_vec(),
                PermissionSet::everything(),
            )
            .with_enabled_modules(scope.enabled_modules().to_vec())
        }))
}

async fn link_customer(state: &AppState, customer_id: &str) -> Result<(), AppError> {
    let settings = site_settings(&state.organization_config.snapshot());
    if !settings.link_people || !state.app.is_enabled(PEOPLE_KEY) {
        return Ok(());
    }
    let pool = state.db()?;
    let row: Option<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT email, name, person_id FROM site_customers WHERE id = ? AND deleted_at IS NULL \
         AND email_verified_at IS NOT NULL",
    )
    .bind(customer_id)
    .fetch_optional(&pool)
    .await?;
    let Some((email, name, None)) = row else {
        return Ok(());
    };
    let scope = customer_scope(customer_id);
    let ctx = Ctx::from_state(state, &scope).await?;
    if let Some(person_id) = find_person(&ctx, &email).await? {
        sqlx::query("UPDATE site_customers SET person_id = ? WHERE id = ? AND person_id IS NULL")
            .bind(person_id)
            .bind(customer_id)
            .execute(&pool)
            .await?;
        return Ok(());
    }
    let Some(owner) = settings.people_owner.as_deref() else {
        return Ok(());
    };
    let Some(owner_scope) = owner_scope(&pool, owner).await? else {
        tracing::warn!(owner, "the owner of site contacts is not an active user");
        return Ok(());
    };
    let ctx = ctx.with_scope(&owner_scope);
    let (first_name, last_name) = split_name(&name);
    let mut values = Map::from_iter([
        ("first_name".to_owned(), Value::String(first_name)),
        ("email".to_owned(), Value::String(email)),
        ("owner_id".to_owned(), Value::String(owner.to_owned())),
    ]);
    if let Some(last_name) = last_name {
        values.insert("last_name".to_owned(), Value::String(last_name));
    }
    let mut tx = ctx.begin().await?;
    let created = aphel_sdk::people::writer()?
        .create_in_tx(
            &ctx,
            &mut tx,
            PersonCreation {
                values,
                custom_fields: Map::new(),
                ip: None,
            },
        )
        .await?;
    sqlx::query("UPDATE site_customers SET person_id = ? WHERE id = ? AND person_id IS NULL")
        .bind(created.record.record_id.to_string())
        .bind(customer_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    created.committed(&ctx).await;
    Ok(())
}

pub(crate) async fn link(state: &AppState, customer_id: &str) {
    if let Err(error) = link_customer(state, customer_id).await {
        tracing::warn!(%error, customer_id, "could not link the site customer to a person");
    }
}

const CUSTOMERS_TABLE: &str = "site_customers";
pub(crate) const PERSON_PURGED_EVENT: &str = "people.person.purged";

pub(crate) struct PersonMerges;

#[async_trait::async_trait]
impl aphel_sdk::merges::MergeReferences for PersonMerges {
    async fn move_references(
        &self,
        _ctx: &Ctx<'_>,
        tx: &mut aphel_sdk::platform::database::transaction::MySqlTx,
        record_type: &str,
        from: uuid::Uuid,
        to: uuid::Uuid,
    ) -> Result<std::collections::BTreeMap<String, u64>, AppError> {
        let mut moved = std::collections::BTreeMap::new();
        if record_type != PERSON_RECORD_TYPE {
            return Ok(moved);
        }
        let count = sqlx::query("UPDATE site_customers SET person_id = ? WHERE person_id = ?")
            .bind(to.to_string())
            .bind(from.to_string())
            .execute(&mut **tx)
            .await?
            .rows_affected();
        if count > 0 {
            moved.insert(CUSTOMERS_TABLE.to_owned(), count);
        }
        Ok(moved)
    }
}

pub(crate) async fn forget_person(pool: &sqlx::MySqlPool, person_id: &str) -> Result<(), AppError> {
    sqlx::query("UPDATE site_customers SET person_id = NULL WHERE person_id = ?")
        .bind(person_id)
        .execute(pool)
        .await?;
    Ok(())
}

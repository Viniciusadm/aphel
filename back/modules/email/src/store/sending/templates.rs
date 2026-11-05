use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::templates::{
    EmailTemplateView, NewEmailTemplate, TemplateFilter, TemplateListScope, TemplateScope,
};
use aphel_kernel::Ctx;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::fields::mysql_definitions::uuid_column;
use aphel_kernel::infrastructure::repositories::mapper;

const TEMPLATE_SELECT: &str = "SELECT id, `key`, name, scope, owner_id, record_type, subject, body_html, \
     created_at, updated_at FROM email_templates ";

fn owner_key(owner: Option<&UserId>) -> String {
    owner
        .map(|owner| owner.as_str().to_owned())
        .unwrap_or_default()
}

fn template_from_row(row: &MySqlRow) -> Result<EmailTemplateView, AppError> {
    let scope: String = row.try_get("scope")?;
    let owner_id: Option<String> = row.try_get("owner_id")?;
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    let updated_at: NaiveDateTime = row.try_get("updated_at")?;
    Ok(EmailTemplateView {
        id: uuid_column(row, "id")?,
        key: row.try_get("key")?,
        name: row.try_get("name")?,
        scope: TemplateScope::parse(&scope)
            .ok_or_else(|| mapper::invalid_column("scope", &scope))?,
        owner_id: owner_id
            .map(|value| mapper::typed_id(value, UserId::new))
            .transpose()?,
        record_type: row.try_get("record_type")?,
        subject: row.try_get("subject")?,
        body_html: row.try_get("body_html")?,
        created_at: mapper::utc(created_at),
        updated_at: mapper::utc(updated_at),
    })
}

#[derive(Clone)]
pub(crate) struct TemplateStore {
    pool: MySqlPool,
}

impl TemplateStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }
}

impl TemplateStore {
    pub(crate) async fn insert(
        &self,
        tx: &mut MySqlTx,
        template: &NewEmailTemplate,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO email_templates (id, `key`, name, scope, owner_id, owner_key, \
             record_type, subject, body_html, created_by, updated_by, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(template.id.to_string())
        .bind(&template.key)
        .bind(&template.name)
        .bind(template.scope.as_str())
        .bind(
            template
                .owner_id
                .as_ref()
                .map(|owner| owner.as_str().to_owned()),
        )
        .bind(owner_key(template.owner_id.as_ref()))
        .bind(&template.record_type)
        .bind(&template.subject)
        .bind(&template.body_html)
        .bind(
            template
                .created_by
                .as_ref()
                .map(|user| user.as_str().to_owned()),
        )
        .bind(
            template
                .created_by
                .as_ref()
                .map(|user| user.as_str().to_owned()),
        )
        .bind(mapper::naive(now))
        .bind(mapper::naive(now))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn update(
        &self,
        tx: &mut MySqlTx,
        template: &EmailTemplateView,
        updated_by: &UserId,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE email_templates SET `key` = ?, name = ?, record_type = ?, subject = ?, body_html = ?, \
             updated_by = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&template.key)
        .bind(&template.name)
        .bind(&template.record_type)
        .bind(&template.subject)
        .bind(&template.body_html)
        .bind(updated_by.as_str())
        .bind(mapper::naive(now))
        .bind(template.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn delete(&self, tx: &mut MySqlTx, id: Uuid) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM email_templates WHERE id = ?")
            .bind(id.to_string())
            .execute(&mut **tx)
            .await?;
        aphel_sdk::reference_cleanup::removed(tx, "email.template", id).await?;
        Ok(result.rows_affected() == 1)
    }

    pub(crate) async fn find(&self, id: Uuid) -> Result<Option<EmailTemplateView>, AppError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            "{TEMPLATE_SELECT} WHERE id = ?"
        )))
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(template_from_row).transpose()
    }

    pub(crate) async fn list(
        &self,
        filter: &TemplateFilter,
    ) -> Result<Vec<EmailTemplateView>, AppError> {
        let mut builder = QueryBuilder::<MySql>::new(TEMPLATE_SELECT);
        match filter.scope {
            TemplateListScope::Organization => {
                builder.push(" WHERE scope = 'organization'");
            }
            TemplateListScope::Personal => {
                builder
                    .push(" WHERE scope = 'personal' AND owner_id = ")
                    .push_bind(filter.viewer.as_str().to_owned());
            }
            TemplateListScope::All => {
                builder
                    .push(" WHERE (scope = 'organization' OR owner_id = ")
                    .push_bind(filter.viewer.as_str().to_owned())
                    .push(")");
            }
        }
        if let Some(record_type) = &filter.record_type {
            builder
                .push(" AND (record_type IS NULL OR record_type = ")
                .push_bind(record_type.clone())
                .push(")");
        }
        builder.push(" ORDER BY scope, name, id");
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter().map(template_from_row).collect()
    }

    pub(crate) async fn key_taken(
        &self,
        owner: Option<&UserId>,
        key: &str,
        except: Option<Uuid>,
    ) -> Result<bool, AppError> {
        let found: Option<String> = sqlx::query_scalar(
            "SELECT id FROM email_templates WHERE owner_key = ? AND `key` = ? \
             AND id <> ? LIMIT 1",
        )
        .bind(owner_key(owner))
        .bind(key)
        .bind(except.map(|id| id.to_string()).unwrap_or_default())
        .fetch_optional(&self.pool)
        .await?;
        Ok(found.is_some())
    }
}

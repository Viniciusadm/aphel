use aphel_contract_crm::CrmRecordTypes;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::domain::{Company, CompanyData, CompanyView, EmailEngagement};
use aphel_kernel::Ctx;
use aphel_kernel::application::fields::{RecordTypeDefinition, RecordTypeRegistry};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::{AccessScope, RecordScopeFilter};
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::fields::mysql_definitions::uuid_column;
use aphel_kernel::infrastructure::query::compiler::push_record_scope;
use aphel_kernel::infrastructure::repositories::mapper::{
    self, ADDRESS_COLUMNS, address_from_row, instant, optional_instant, optional_user,
    optional_uuid, owner_reference, push_ids,
};
use aphel_kernel::records::load_contributed;

const COMPANY_COLUMNS: &str = "r.id, r.owner_id, r.name, r.domain, r.website, r.industry, r.size_range, \
     r.phone, r.email, r.description, r.source, r.parent_company_id, r.created_by, r.archived_at, r.created_at, \
     r.updated_at, u.display_name AS owner_display_name, r.last_contacted_at, \
     r.last_email_received_at, r.last_email_sent_at, r.interaction_count, r.last_contacted_by";

pub fn engagement_from_row(row: &MySqlRow) -> Result<EmailEngagement, AppError> {
    Ok(EmailEngagement {
        last_contacted_at: optional_instant(row, "last_contacted_at")?,
        last_email_received_at: optional_instant(row, "last_email_received_at")?,
        last_email_sent_at: optional_instant(row, "last_email_sent_at")?,
        interaction_count: i64::from(row.try_get::<i32, _>("interaction_count")?),
        last_contacted_by: optional_user(row, "last_contacted_by")?,
    })
}

fn company_from_row(row: &MySqlRow) -> Result<CompanyView, AppError> {
    Ok(CompanyView {
        company: Company {
            id: uuid_column(row, "id")?,
            data: CompanyData {
                name: row.try_get("name")?,
                domain: row.try_get("domain")?,
                website: row.try_get("website")?,
                industry: row.try_get("industry")?,
                size_range: row.try_get("size_range")?,
                phone: row.try_get("phone")?,
                email: row.try_get("email")?,
                owner_id: optional_user(row, "owner_id")?,
                address: address_from_row(row)?,
                description: row.try_get("description")?,
                source: row.try_get("source")?,
                parent_company_id: optional_uuid(row, "parent_company_id")?,
            },
            engagement: engagement_from_row(row)?,
            created_by: optional_user(row, "created_by")?,
            archived_at: optional_instant(row, "archived_at")?,
            created_at: instant(row, "created_at")?,
            updated_at: instant(row, "updated_at")?,
        },
        owner: owner_reference(row)?,
        contributed: Default::default(),
    })
}

#[derive(Clone)]
pub struct CompanyStore {
    pool: MySqlPool,
    registry: Arc<RecordTypeRegistry>,
}

impl CompanyStore {
    pub(crate) fn new(pool: MySqlPool, registry: Arc<RecordTypeRegistry>) -> Self {
        Self { pool, registry }
    }

    pub fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone(), ctx.registry().clone())
    }

    fn definition(&self) -> Result<&RecordTypeDefinition, AppError> {
        self.registry.require(RecordType::COMPANY.as_str())
    }

    fn select() -> QueryBuilder<MySql> {
        QueryBuilder::new(format!(
            "SELECT {COMPANY_COLUMNS}, {ADDRESS_COLUMNS} FROM companies r \
             LEFT JOIN users u ON u.id = r.owner_id WHERE r.deleted_at IS NULL"
        ))
    }

    async fn with_contributed(
        &self,
        mut views: Vec<CompanyView>,
    ) -> Result<Vec<CompanyView>, AppError> {
        let ids: Vec<Uuid> = views.iter().map(|view| view.company.id).collect();
        let mut values = load_contributed(&self.pool, self.definition()?, &ids).await?;
        for view in &mut views {
            view.contributed = values.remove(&view.company.id).unwrap_or_default();
        }
        Ok(views)
    }

    pub async fn find(
        &self,
        scope: &AccessScope,
        visibility: &RecordScopeFilter,
        company_id: Uuid,
    ) -> Result<Option<CompanyView>, AppError> {
        let mut builder = Self::select();
        builder
            .push(" AND r.id = ")
            .push_bind(company_id.to_string());
        push_record_scope(&mut builder, scope, self.definition()?, visibility);
        let row = builder.build().fetch_optional(&self.pool).await?;
        let Some(view) = row.as_ref().map(company_from_row).transpose()? else {
            return Ok(None);
        };
        Ok(self.with_contributed(vec![view]).await?.pop())
    }

    pub(crate) async fn find_many(
        &self,
        company_ids: &[Uuid],
    ) -> Result<Vec<CompanyView>, AppError> {
        if company_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = Self::select();
        builder.push(" AND r.id");
        push_ids(&mut builder, company_ids);
        let rows = builder.build().fetch_all(&self.pool).await?;
        let views = rows
            .iter()
            .map(company_from_row)
            .collect::<Result<_, _>>()?;
        self.with_contributed(views).await
    }

    pub(crate) async fn insert(&self, tx: &mut MySqlTx, company: &Company) -> Result<(), AppError> {
        let data = &company.data;
        let address = &data.address;
        sqlx::query(
            "INSERT INTO companies (id, owner_id, name, domain, website, industry, \
             size_range, phone, email, address_street, address_number, address_complement, \
             address_district, address_city, address_state, address_postal_code, address_country, \
             description, source, parent_company_id, created_by, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(company.id.to_string())
        .bind(data.owner_id.as_ref().map(UserId::as_str))
        .bind(&data.name)
        .bind(&data.domain)
        .bind(&data.website)
        .bind(&data.industry)
        .bind(&data.size_range)
        .bind(&data.phone)
        .bind(&data.email)
        .bind(&address.street)
        .bind(&address.number)
        .bind(&address.complement)
        .bind(&address.district)
        .bind(&address.city)
        .bind(&address.state)
        .bind(&address.postal_code)
        .bind(&address.country)
        .bind(&data.description)
        .bind(&data.source)
        .bind(data.parent_company_id.map(|id| id.to_string()))
        .bind(company.created_by.as_ref().map(UserId::as_str))
        .bind(mapper::naive(company.created_at))
        .bind(mapper::naive(company.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub async fn update(
        &self,
        tx: &mut MySqlTx,
        company_id: Uuid,
        data: &CompanyData,
        updated_at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        let address = &data.address;
        sqlx::query(
            "UPDATE companies SET owner_id = ?, name = ?, domain = ?, website = ?, industry = ?, \
             size_range = ?, phone = ?, email = ?, address_street = ?, address_number = ?, \
             address_complement = ?, address_district = ?, address_city = ?, address_state = ?, \
             address_postal_code = ?, address_country = ?, description = ?, source = ?, \
             parent_company_id = ?, updated_at = ? \
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(data.owner_id.as_ref().map(UserId::as_str))
        .bind(&data.name)
        .bind(&data.domain)
        .bind(&data.website)
        .bind(&data.industry)
        .bind(&data.size_range)
        .bind(&data.phone)
        .bind(&data.email)
        .bind(&address.street)
        .bind(&address.number)
        .bind(&address.complement)
        .bind(&address.district)
        .bind(&address.city)
        .bind(&address.state)
        .bind(&address.postal_code)
        .bind(&address.country)
        .bind(&data.description)
        .bind(&data.source)
        .bind(data.parent_company_id.map(|id| id.to_string()))
        .bind(mapper::naive(updated_at))
        .bind(company_id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn set_archived(
        &self,
        tx: &mut MySqlTx,
        company_id: Uuid,
        archived_at: Option<DateTime<Utc>>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE companies SET archived_at = ? \
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(archived_at.map(mapper::naive))
        .bind(company_id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub async fn soft_delete(
        &self,
        tx: &mut MySqlTx,
        company_id: Uuid,
        deleted_at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE companies SET deleted_at = ? \
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(mapper::naive(deleted_at))
        .bind(company_id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}

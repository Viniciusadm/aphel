use aphel_contract_crm::CrmRecordTypes;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::domain::{
    CompanyContactLink, CompanyReference, Contact, ContactCompanyLink, ContactCompanyRole,
    ContactData, ContactSummary, ContactView, NewContactCompanyLink,
};
use aphel_kernel::Ctx;
use aphel_kernel::application::fields::{RecordTypeDefinition, RecordTypeRegistry};
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
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
use companies::store::engagement_from_row;

const CONTACT_COLUMNS: &str = "r.id, r.owner_id, r.company_id, r.first_name, r.last_name, r.email, \
     r.phone, r.mobile, r.job_title, r.source, r.notes, r.created_by, r.archived_at, r.created_at, \
     r.updated_at, u.display_name AS owner_display_name, co.name AS company_name, \
     co.domain AS company_domain, r.last_contacted_at, r.last_email_received_at, \
     r.last_email_sent_at, r.interaction_count, r.last_contacted_by";

fn role_column(row: &MySqlRow) -> Result<Option<ContactCompanyRole>, AppError> {
    let role: Option<String> = row.try_get("role")?;
    Ok(role.as_deref().and_then(ContactCompanyRole::parse))
}

fn contact_from_row(row: &MySqlRow) -> Result<ContactView, AppError> {
    let company_id = optional_uuid(row, "company_id")?;
    let company_name: Option<String> = row.try_get("company_name")?;
    Ok(ContactView {
        contact: Contact {
            id: uuid_column(row, "id")?,
            data: ContactData {
                first_name: row.try_get("first_name")?,
                last_name: row.try_get("last_name")?,
                email: row.try_get("email")?,
                phone: row.try_get("phone")?,
                mobile: row.try_get("mobile")?,
                job_title: row.try_get("job_title")?,
                company_id,
                owner_id: optional_user(row, "owner_id")?,
                address: address_from_row(row)?,
                source: row.try_get("source")?,
                notes: row.try_get("notes")?,
            },
            engagement: engagement_from_row(row)?,
            created_by: optional_user(row, "created_by")?,
            archived_at: optional_instant(row, "archived_at")?,
            created_at: instant(row, "created_at")?,
            updated_at: instant(row, "updated_at")?,
        },
        owner: owner_reference(row)?,
        contributed: Default::default(),
        company: company_id
            .zip(company_name)
            .map(|(id, name)| -> Result<CompanyReference, AppError> {
                Ok(CompanyReference {
                    id,
                    name,
                    domain: row.try_get("company_domain")?,
                })
            })
            .transpose()?,
    })
}

fn company_link_from_row(row: &MySqlRow) -> Result<ContactCompanyLink, AppError> {
    let is_primary: i64 = row.try_get("is_primary")?;
    Ok(ContactCompanyLink {
        link_id: optional_uuid(row, "link_id")?,
        company: CompanyReference {
            id: uuid_column(row, "company_id")?,
            name: row.try_get("company_name")?,
            domain: row.try_get("company_domain")?,
        },
        role: role_column(row)?,
        is_primary: is_primary != 0,
        created_at: optional_instant(row, "linked_at")?,
    })
}

fn company_contact_from_row(row: &MySqlRow) -> Result<CompanyContactLink, AppError> {
    let is_primary: i64 = row.try_get("is_primary")?;
    Ok(CompanyContactLink {
        link_id: optional_uuid(row, "link_id")?,
        contact: ContactSummary {
            id: uuid_column(row, "id")?,
            name: row.try_get("full_name")?,
            email: row.try_get("email")?,
            phone: row.try_get("phone")?,
            mobile: row.try_get("mobile")?,
            job_title: row.try_get("job_title")?,
        },
        role: role_column(row)?,
        is_primary: is_primary != 0,
    })
}

#[derive(Clone)]
pub struct ContactStore {
    pool: MySqlPool,
    registry: Arc<RecordTypeRegistry>,
}

impl ContactStore {
    pub(crate) fn new(pool: MySqlPool, registry: Arc<RecordTypeRegistry>) -> Self {
        Self { pool, registry }
    }

    pub fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone(), ctx.registry().clone())
    }

    pub(crate) async fn with_contributed(
        &self,
        mut views: Vec<ContactView>,
    ) -> Result<Vec<ContactView>, AppError> {
        let ids: Vec<Uuid> = views.iter().map(|view| view.contact.id).collect();
        let mut values =
            load_contributed(&self.pool, self.definition(&RecordType::CONTACT)?, &ids).await?;
        for view in &mut views {
            view.contributed = values.remove(&view.contact.id).unwrap_or_default();
        }
        Ok(views)
    }

    fn definition(&self, record_type: &RecordType) -> Result<&RecordTypeDefinition, AppError> {
        self.registry.require(record_type.as_str())
    }

    fn select() -> QueryBuilder<MySql> {
        QueryBuilder::new(format!(
            "SELECT {CONTACT_COLUMNS}, {ADDRESS_COLUMNS} FROM contacts r \
             LEFT JOIN users u ON u.id = r.owner_id \
             LEFT JOIN companies co ON co.id = r.company_id \
             AND co.deleted_at IS NULL \
             WHERE r.deleted_at IS NULL"
        ))
    }

    fn company_links_query(
        &self,
        scope: &AccessScope,
        company_visibility: &RecordScopeFilter,
        contact_id: Uuid,
        only_company: Option<Uuid>,
    ) -> Result<QueryBuilder<MySql>, AppError> {
        let definition = self.definition(&RecordType::COMPANY)?;
        let mut builder = QueryBuilder::new(
            "SELECT * FROM (SELECT NULL AS link_id, r.id AS company_id, r.name AS company_name, \
             r.domain AS company_domain, NULL AS role, CAST(1 AS SIGNED) AS is_primary, NULL AS linked_at \
             FROM contacts c JOIN companies r ON r.id = c.company_id \
             WHERE r.deleted_at IS NULL AND c.deleted_at IS NULL",
        );
        builder
            .push(" AND c.id = ")
            .push_bind(contact_id.to_string());
        if let Some(company_id) = only_company {
            builder
                .push(" AND 1 = 0 AND r.id = ")
                .push_bind(company_id.to_string());
        }
        push_record_scope(&mut builder, scope, definition, company_visibility);
        builder.push(
            " UNION ALL SELECT l.id AS link_id, r.id AS company_id, r.name AS company_name, \
             r.domain AS company_domain, l.role AS role, CAST(0 AS SIGNED) AS is_primary, l.created_at AS linked_at \
             FROM contact_companies l JOIN companies r ON r.id = l.company_id \
             WHERE r.deleted_at IS NULL",
        );
        builder
            .push(" AND l.contact_id = ")
            .push_bind(contact_id.to_string());
        if let Some(company_id) = only_company {
            builder
                .push(" AND r.id = ")
                .push_bind(company_id.to_string());
        }
        push_record_scope(&mut builder, scope, definition, company_visibility);
        builder.push(") links ORDER BY is_primary DESC, company_name, company_id");
        Ok(builder)
    }

    pub async fn find(
        &self,
        scope: &AccessScope,
        visibility: &RecordScopeFilter,
        contact_id: Uuid,
    ) -> Result<Option<ContactView>, AppError> {
        let mut builder = Self::select();
        builder
            .push(" AND r.id = ")
            .push_bind(contact_id.to_string());
        push_record_scope(
            &mut builder,
            scope,
            self.definition(&RecordType::CONTACT)?,
            visibility,
        );
        let row = builder.build().fetch_optional(&self.pool).await?;
        let Some(view) = row.as_ref().map(contact_from_row).transpose()? else {
            return Ok(None);
        };
        Ok(self.with_contributed(vec![view]).await?.pop())
    }

    pub(crate) async fn find_many(
        &self,
        contact_ids: &[Uuid],
    ) -> Result<Vec<ContactView>, AppError> {
        if contact_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = Self::select();
        builder.push(" AND r.id");
        push_ids(&mut builder, contact_ids);
        let rows = builder.build().fetch_all(&self.pool).await?;
        let views = rows
            .iter()
            .map(contact_from_row)
            .collect::<Result<_, _>>()?;
        self.with_contributed(views).await
    }

    pub(crate) async fn insert(&self, tx: &mut MySqlTx, contact: &Contact) -> Result<(), AppError> {
        let data = &contact.data;
        let address = &data.address;
        sqlx::query(
            "INSERT INTO contacts (id, owner_id, company_id, first_name, last_name, \
             email, phone, mobile, job_title, address_street, address_number, address_complement, \
             address_district, address_city, address_state, address_postal_code, address_country, \
             source, notes, created_by, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(contact.id.to_string())
        .bind(data.owner_id.as_ref().map(UserId::as_str))
        .bind(data.company_id.map(|id| id.to_string()))
        .bind(&data.first_name)
        .bind(&data.last_name)
        .bind(&data.email)
        .bind(&data.phone)
        .bind(&data.mobile)
        .bind(&data.job_title)
        .bind(&address.street)
        .bind(&address.number)
        .bind(&address.complement)
        .bind(&address.district)
        .bind(&address.city)
        .bind(&address.state)
        .bind(&address.postal_code)
        .bind(&address.country)
        .bind(&data.source)
        .bind(&data.notes)
        .bind(contact.created_by.as_ref().map(UserId::as_str))
        .bind(mapper::naive(contact.created_at))
        .bind(mapper::naive(contact.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub async fn update(
        &self,
        tx: &mut MySqlTx,
        contact_id: Uuid,
        data: &ContactData,
        updated_at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        let address = &data.address;
        sqlx::query(
            "UPDATE contacts SET owner_id = ?, company_id = ?, first_name = ?, last_name = ?, \
             email = ?, phone = ?, mobile = ?, job_title = ?, address_street = ?, address_number = ?, \
             address_complement = ?, address_district = ?, address_city = ?, address_state = ?, \
             address_postal_code = ?, address_country = ?, source = ?, notes = ?, updated_at = ? \
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(data.owner_id.as_ref().map(UserId::as_str))
        .bind(data.company_id.map(|id| id.to_string()))
        .bind(&data.first_name)
        .bind(&data.last_name)
        .bind(&data.email)
        .bind(&data.phone)
        .bind(&data.mobile)
        .bind(&data.job_title)
        .bind(&address.street)
        .bind(&address.number)
        .bind(&address.complement)
        .bind(&address.district)
        .bind(&address.city)
        .bind(&address.state)
        .bind(&address.postal_code)
        .bind(&address.country)
        .bind(&data.source)
        .bind(&data.notes)
        .bind(mapper::naive(updated_at))
        .bind(contact_id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn set_archived(
        &self,
        tx: &mut MySqlTx,
        contact_id: Uuid,
        archived_at: Option<DateTime<Utc>>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE contacts SET archived_at = ? \
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(archived_at.map(mapper::naive))
        .bind(contact_id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub async fn soft_delete(
        &self,
        tx: &mut MySqlTx,
        contact_id: Uuid,
        deleted_at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE contacts SET deleted_at = ? \
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(mapper::naive(deleted_at))
        .bind(contact_id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn list_companies(
        &self,
        scope: &AccessScope,
        company_visibility: &RecordScopeFilter,
        contact_id: Uuid,
        window: ListWindow,
    ) -> Result<Listing<ContactCompanyLink>, AppError> {
        let rows = self
            .company_links_query(scope, company_visibility, contact_id, None)?
            .build()
            .fetch_all(&self.pool)
            .await?;
        let links: Vec<ContactCompanyLink> = rows
            .iter()
            .map(company_link_from_row)
            .collect::<Result<_, _>>()?;
        let total = links.len() as u64;
        Ok(Listing {
            items: links
                .into_iter()
                .skip(window.offset as usize)
                .take(window.limit as usize)
                .collect(),
            total,
        })
    }

    pub async fn find_link(
        &self,
        scope: &AccessScope,
        contact_id: Uuid,
        company_id: Uuid,
    ) -> Result<Option<ContactCompanyLink>, AppError> {
        let row = self
            .company_links_query(
                scope,
                &RecordScopeFilter::Organization,
                contact_id,
                Some(company_id),
            )?
            .build()
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(company_link_from_row).transpose()
    }

    pub async fn insert_link(
        &self,
        tx: &mut MySqlTx,
        link: &NewContactCompanyLink,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO contact_companies (id, contact_id, company_id, role, created_at) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(link.id.to_string())
        .bind(link.contact_id.to_string())
        .bind(link.company_id.to_string())
        .bind(link.role.map(|role| role.as_str()))
        .bind(mapper::naive(link.created_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn delete_link(
        &self,
        tx: &mut MySqlTx,
        contact_id: Uuid,
        company_id: Uuid,
    ) -> Result<bool, AppError> {
        let result =
            sqlx::query("DELETE FROM contact_companies WHERE contact_id = ? AND company_id = ?")
                .bind(contact_id.to_string())
                .bind(company_id.to_string())
                .execute(&mut **tx)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    pub(crate) async fn list_company_contacts(
        &self,
        scope: &AccessScope,
        contact_visibility: &RecordScopeFilter,
        company_id: Uuid,
        window: ListWindow,
    ) -> Result<Listing<CompanyContactLink>, AppError> {
        let definition = self.definition(&RecordType::CONTACT)?;
        let company = company_id.to_string();
        let push_from = |builder: &mut QueryBuilder<MySql>| {
            builder
                .push(
                    " FROM contacts r LEFT JOIN contact_companies l ON l.contact_id = r.id \
                     AND l.company_id = ",
                )
                .push_bind(company.clone())
                .push(" WHERE r.deleted_at IS NULL")
                .push(" AND (r.company_id = ")
                .push_bind(company.clone())
                .push(" OR l.id IS NOT NULL)");
            push_record_scope(builder, scope, definition, contact_visibility);
        };

        let mut count = QueryBuilder::new("SELECT COUNT(*)");
        push_from(&mut count);
        let total: i64 = count.build_query_scalar().fetch_one(&self.pool).await?;

        let mut builder = QueryBuilder::new(
            "SELECT r.id, r.full_name, r.email, r.phone, r.mobile, r.job_title, l.id AS link_id, l.role AS role, CAST(r.company_id <=> ",
        );
        builder
            .push_bind(company.clone())
            .push(" AS SIGNED) AS is_primary");
        push_from(&mut builder);
        builder
            .push(" ORDER BY is_primary DESC, r.full_name, r.id LIMIT ")
            .push_bind(window.limit as i64)
            .push(" OFFSET ")
            .push_bind(window.offset as i64);
        let rows = builder.build().fetch_all(&self.pool).await?;
        Ok(Listing {
            items: rows
                .iter()
                .map(company_contact_from_row)
                .collect::<Result<_, _>>()?,
            total: total.max(0) as u64,
        })
    }
}

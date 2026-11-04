use aphel_contract_crm::CrmRecordTypes;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::domain::EmailEngagement;
use crate::domain::{
    CompanyContactLink, CompanyReference, Contact, ContactCompanyLink, ContactCompanyRole,
    ContactData, ContactSummary, ContactView, NewContactCompanyLink,
};
use aphel_kernel::Ctx;
use aphel_kernel::application::fields::{EntityDefinition, RecordTypeRegistry};
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

const CONTACT_COLUMNS: &str = "r.id, r.owner_id, r.company_id, r.first_name, r.last_name, r.email, \
     r.phone, r.mobile, r.job_title, r.source, r.notes, r.created_by, r.archived_at, r.created_at, \
     r.updated_at, u.display_name AS owner_display_name, r.last_contacted_at, r.last_email_received_at, \
     r.last_email_sent_at, r.interaction_count, r.last_contacted_by";

fn role_column(row: &MySqlRow) -> Result<Option<ContactCompanyRole>, AppError> {
    let role: Option<String> = row.try_get("role")?;
    Ok(role.as_deref().and_then(ContactCompanyRole::parse))
}

fn contact_from_row(row: &MySqlRow) -> Result<ContactView, AppError> {
    let company_id = optional_uuid(row, "company_id")?;
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
        company: None,
    })
}

fn company_link_from_row(row: &MySqlRow) -> Result<ContactCompanyLink, AppError> {
    let is_primary: i64 = row.try_get("is_primary")?;
    Ok(ContactCompanyLink {
        link_id: optional_uuid(row, "link_id")?,
        company: CompanyReference {
            id: uuid_column(row, "company_id")?,
            name: row
                .try_get::<Option<String>, _>("company_name")?
                .unwrap_or_default(),
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
pub struct ContactStore<'a> {
    pool: MySqlPool,
    registry: Arc<RecordTypeRegistry>,
    ctx: Ctx<'a>,
}

impl<'a> ContactStore<'a> {
    pub fn of(ctx: &Ctx<'a>) -> Self {
        Self {
            pool: ctx.pool().clone(),
            registry: ctx.registry().clone(),
            ctx: ctx.clone(),
        }
    }

    pub(crate) async fn with_contributed(
        &self,
        mut views: Vec<ContactView>,
    ) -> Result<Vec<ContactView>, AppError> {
        let ids: Vec<Uuid> = views.iter().map(|view| view.contact.id).collect();
        let mut values =
            load_contributed(&self.pool, self.definition(&RecordType::CONTACT)?, &ids).await?;
        let companies = self
            .company_references(
                &views
                    .iter()
                    .filter_map(|view| view.contact.data.company_id)
                    .collect::<Vec<_>>(),
            )
            .await?;
        for view in &mut views {
            view.contributed = values.remove(&view.contact.id).unwrap_or_default();
            view.company = view
                .contact
                .data
                .company_id
                .and_then(|id| companies.get(&id).cloned());
        }
        Ok(views)
    }

    fn definition(&self, record_type: &RecordType) -> Result<&EntityDefinition, AppError> {
        self.registry.require(record_type.as_str())
    }

    fn select() -> QueryBuilder<MySql> {
        QueryBuilder::new(format!(
            "SELECT {CONTACT_COLUMNS}, {ADDRESS_COLUMNS} FROM people r \
             LEFT JOIN users u ON u.id = r.owner_id \
             WHERE r.deleted_at IS NULL"
        ))
    }

    fn company_links_query(contact_id: Uuid, only_company: Option<Uuid>) -> QueryBuilder<MySql> {
        let mut builder = QueryBuilder::new(
            "SELECT * FROM (SELECT NULL AS link_id, c.company_id, NULL AS company_name, NULL AS company_domain, NULL AS role, CAST(1 AS SIGNED) AS is_primary, NULL AS linked_at FROM people c WHERE c.deleted_at IS NULL AND c.company_id IS NOT NULL AND c.id = ",
        );
        builder.push_bind(contact_id.to_string());
        if only_company.is_some() {
            builder.push(" AND FALSE");
        }
        builder.push(" UNION ALL SELECT l.id AS link_id, l.company_id, NULL AS company_name, NULL AS company_domain, l.role, CAST(0 AS SIGNED) AS is_primary, l.created_at AS linked_at FROM people_organizations l WHERE l.contact_id = ").push_bind(contact_id.to_string());
        if let Some(id) = only_company {
            builder
                .push(" AND l.company_id = ")
                .push_bind(id.to_string());
        }
        builder.push(") links ORDER BY is_primary DESC, company_id");
        builder
    }

    async fn company_references(
        &self,
        ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, CompanyReference>, AppError> {
        let mut references = std::collections::HashMap::new();
        let Some(definition) = self.registry.get(&RecordType::COMPANY) else {
            return Ok(references);
        };
        if self
            .ctx
            .scope
            .scope_for(&definition.read_permission)
            .is_none()
        {
            return Ok(references);
        }
        let ids: Vec<_> = ids
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        for batch in ids.chunks(1000) {
            let listing = aphel_sdk::record_context::list(
                &self.ctx,
                RecordType::COMPANY.as_str(),
                aphel_kernel::application::query::RecordQuery {
                    filter: Some(aphel_kernel::application::query::Filter::condition(
                        "id",
                        aphel_kernel::application::query::FilterOperator::In,
                        serde_json::json!(batch),
                    )),
                    archived: aphel_kernel::application::query::ArchivedFilter::Include,
                    window: ListWindow {
                        offset: 0,
                        limit: batch.len() as u64,
                    },
                    ..Default::default()
                },
            )
            .await?;
            for value in listing.items {
                let id = value
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .and_then(|id| Uuid::parse_str(id).ok())
                    .filter(|id| batch.contains(id))
                    .ok_or_else(|| {
                        AppError::Internal("invalid organization reference snapshot".into())
                    })?;
                references.insert(
                    id,
                    CompanyReference {
                        id,
                        name: value
                            .get("name")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                        domain: value
                            .get("domain")
                            .and_then(serde_json::Value::as_str)
                            .map(str::to_owned),
                    },
                );
            }
        }
        Ok(references)
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
            "INSERT INTO people (id, owner_id, company_id, first_name, last_name, \
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
            "UPDATE people SET owner_id = ?, company_id = ?, first_name = ?, last_name = ?, \
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
            "UPDATE people SET archived_at = ? \
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
            "UPDATE people SET deleted_at = ? \
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
        contact_id: Uuid,
        window: ListWindow,
    ) -> Result<Listing<ContactCompanyLink>, AppError> {
        let rows = Self::company_links_query(contact_id, None)
            .build()
            .fetch_all(&self.pool)
            .await?;
        let references = self
            .company_references(
                &rows
                    .iter()
                    .map(|row| uuid_column(row, "company_id"))
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .await?;
        let mut links = Vec::new();
        for row in &rows {
            let mut link = company_link_from_row(row)?;
            if let Some(company) = references.get(&link.company.id) {
                link.company = company.clone();
                links.push(link);
            }
        }
        links.sort_by(|left, right| {
            right
                .is_primary
                .cmp(&left.is_primary)
                .then_with(|| {
                    left.company
                        .name
                        .to_lowercase()
                        .cmp(&right.company.name.to_lowercase())
                })
                .then(left.company.id.cmp(&right.company.id))
        });
        Ok(Listing {
            total: links.len() as u64,
            items: links
                .into_iter()
                .skip(window.offset as usize)
                .take(window.limit as usize)
                .collect(),
        })
    }

    pub async fn find_link(
        &self,
        contact_id: Uuid,
        company_id: Uuid,
    ) -> Result<Option<ContactCompanyLink>, AppError> {
        let row = Self::company_links_query(contact_id, Some(company_id))
            .build()
            .fetch_optional(&self.pool)
            .await?;
        let Some(mut link) = row.as_ref().map(company_link_from_row).transpose()? else {
            return Ok(None);
        };
        if let Some(company) = self
            .company_references(&[company_id])
            .await?
            .remove(&company_id)
        {
            link.company = company;
        }
        Ok(Some(link))
    }

    pub async fn insert_link(
        &self,
        tx: &mut MySqlTx,
        link: &NewContactCompanyLink,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO people_organizations (id, contact_id, company_id, role, created_at) \
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
            sqlx::query("DELETE FROM people_organizations WHERE contact_id = ? AND company_id = ?")
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
                    " FROM people r LEFT JOIN people_organizations l ON l.contact_id = r.id \
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

fn engagement_from_row(row: &MySqlRow) -> Result<EmailEngagement, AppError> {
    Ok(EmailEngagement {
        last_contacted_at: optional_instant(row, "last_contacted_at")?,
        last_email_received_at: optional_instant(row, "last_email_received_at")?,
        last_email_sent_at: optional_instant(row, "last_email_sent_at")?,
        interaction_count: i64::from(row.try_get::<i32, _>("interaction_count")?),
        last_contacted_by: optional_user(row, "last_contacted_by")?,
    })
}

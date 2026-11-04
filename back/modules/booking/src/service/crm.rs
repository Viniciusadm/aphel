use aphel_contract_crm::CrmRecordTypes;
use serde_json::json;
use uuid::Uuid;

use crate::{ACTIVITY_TYPE_KEY, ContactFieldOption, MAPPABLE_CONTACT_FIELDS};
use aphel_kernel::Ctx;
use aphel_kernel::application::access::{AccessRepository, build_access_scope};
use aphel_kernel::application::fields::FieldDefinitionRepository;
use aphel_kernel::application::organization_admin::ListWindow;
use aphel_kernel::application::query::{Filter, FilterOperator, RecordQuery};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::domain::fields::FieldType;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::fields::MySqlFieldDefinitionRepository;
use aphel_kernel::infrastructure::repositories::mysql_access::MySqlAccessRepository;
use aphel_kernel::records::PendingCommit;
use aphel_sdk::tasks::{MeetingActivity, MeetingActivityMove};

fn mappable_custom(field_type: FieldType) -> bool {
    matches!(
        field_type,
        FieldType::Text
            | FieldType::LongText
            | FieldType::Url
            | FieldType::Email
            | FieldType::Phone
    )
}

pub(crate) async fn host_scope(
    ctx: &Ctx<'_>,
    user_id: &UserId,
) -> Result<Option<AccessScope>, AppError> {
    Ok(MySqlAccessRepository::new(ctx.pool().clone())
        .load_access(user_id)
        .await?
        .map(|access| build_access_scope(user_id.clone(), access)))
}

pub(crate) async fn find_contact_by_email(
    ctx: &Ctx<'_>,
    email: &str,
) -> Result<Option<Uuid>, AppError> {
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
    Ok(people::service::list(ctx, query)
        .await?
        .items
        .first()
        .map(|details| details.view.contact.id))
}

pub(crate) async fn meeting_activity_type(
    ctx: &Ctx<'_>,
    scope: &AccessScope,
) -> Result<Option<Uuid>, AppError> {
    Ok(aphel_sdk::tasks::types()?
        .list(
            &ctx.with_scope(scope),
            false,
            ListWindow {
                offset: 0,
                limit: 500,
            },
        )
        .await?
        .items
        .into_iter()
        .find(|activity_type| activity_type.key == ACTIVITY_TYPE_KEY && activity_type.is_active)
        .map(|activity_type| activity_type.id))
}

pub(crate) async fn schedule_meeting(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    meeting: MeetingActivity,
) -> Result<Uuid, AppError> {
    aphel_sdk::tasks::meetings()?
        .schedule(ctx, tx, meeting)
        .await
}

pub(crate) async fn move_meeting(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    scope: &AccessScope,
    activity_id: Uuid,
    change: MeetingActivityMove,
) -> Result<Option<PendingCommit>, AppError> {
    aphel_sdk::tasks::meetings()?
        .move_to(ctx, tx, scope, activity_id, change)
        .await
}

pub(crate) async fn cancel_meeting(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    scope: &AccessScope,
    activity_id: Uuid,
    ip: Option<String>,
) -> Result<Option<PendingCommit>, AppError> {
    aphel_sdk::tasks::meetings()?
        .cancel(ctx, tx, scope, activity_id, ip)
        .await
}

pub(crate) async fn contact_fields(ctx: &Ctx<'_>) -> Result<Vec<ContactFieldOption>, AppError> {
    let definitions = MySqlFieldDefinitionRepository::new(ctx.pool().clone())
        .list(&RecordType::CONTACT)
        .await?;
    let mut options = Vec::new();
    for key in MAPPABLE_CONTACT_FIELDS {
        if let Some(definition) = definitions
            .iter()
            .find(|definition| definition.key.as_str() == key && definition.is_active)
        {
            options.push(ContactFieldOption {
                key: key.to_owned(),
                label: definition.label.clone(),
                is_custom: false,
            });
        }
    }
    options.extend(
        definitions
            .iter()
            .filter(|definition| {
                !definition.is_system
                    && definition.is_active
                    && mappable_custom(definition.field_type)
            })
            .map(|definition| ContactFieldOption {
                key: definition.key.as_str().to_owned(),
                label: definition.label.clone(),
                is_custom: true,
            }),
    );
    Ok(options)
}

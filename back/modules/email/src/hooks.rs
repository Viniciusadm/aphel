use uuid::Uuid;

use crate::domain::{EmailDirection, EmailKind};
use crate::{EmailAddress, EmailBody, KEY};
use aphel_kernel::Ctx;
use aphel_kernel::application::shared::RecordRef;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::Hook;

#[derive(Debug, Clone, PartialEq)]
pub struct OutgoingMessage {
    pub(crate) mailbox_id: Uuid,
    pub(crate) sender_id: UserId,
    pub(crate) from: EmailAddress,
    pub(crate) to: Vec<EmailAddress>,
    pub(crate) cc: Vec<EmailAddress>,
    pub(crate) bcc: Vec<EmailAddress>,
    pub(crate) subject: String,
    pub(crate) body: EmailBody,
    pub(crate) record: Option<RecordRef>,
    pub(crate) thread_id: Option<Uuid>,
    pub(crate) scheduled: bool,
    pub(crate) tracking: bool,
}

pub struct OutgoingEmail;

impl Hook for OutgoingEmail {
    type Args = OutgoingMessage;
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinkingEmail {
    pub(crate) mailbox_id: Uuid,
    pub(crate) owner_id: UserId,
    pub(crate) thread_id: Uuid,
    pub(crate) message_id: String,
    pub(crate) direction: EmailDirection,
    pub(crate) kind: EmailKind,
    pub(crate) from: Option<String>,
    pub(crate) participants: Vec<String>,
    pub(crate) contact_ids: Vec<Uuid>,
    pub(crate) records: Vec<RecordRef>,
}

pub struct InboundLinking;

impl Hook for InboundLinking {
    type Args = LinkingEmail;
}

pub(crate) async fn apply_linking(
    ctx: &Ctx<'_>,
    email: LinkingEmail,
) -> Result<LinkingEmail, AppError> {
    apply::<InboundLinking>(ctx, email).await
}

async fn apply<H: Hook>(ctx: &Ctx<'_>, args: H::Args) -> Result<H::Args, AppError> {
    if !ctx.app().hooks().has_filters_except::<H>(None) {
        return Ok(args);
    }
    let mut tx = ctx.begin().await?;
    let result = ctx.apply_hook::<H>(&mut tx, KEY, args).await;
    tx.rollback().await?;
    result
}

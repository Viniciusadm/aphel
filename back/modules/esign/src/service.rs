use aphel_contract_crm::CrmRecordTypes;
use bytes::Bytes;
use chrono::Utc;
use futures::StreamExt;
use serde_json::{Value, json};
use uuid::Uuid;

use aphel_kernel::Ctx;
use aphel_kernel::application::files::{ListAttachmentsQuery, UploadFileCommand};
use aphel_kernel::application::organization_admin::ListWindow;
use aphel_kernel::application::shared::{
    Actor, AuditEntry, DomainEvent, FieldChange, Job, RecordRef, RecordType,
};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::{AppError, CodedKind, FieldViolation};
use aphel_kernel::records::RecordCtx;

use crate::clicksign::Clicksign;
use crate::provider::{NoticeKind, SignatureProvider, WebhookNotice};
use crate::store::{
    self, RequestRow, SIGNER_PENDING, SIGNER_REFUSED, SIGNER_SIGNED, STATUS_CANCELLED,
    STATUS_COMPLETED, STATUS_EXPIRED, STATUS_REFUSED, STATUS_SENT, SignerRow,
};
use crate::{
    ACCESS_TOKEN_SETTING, ATTACH_SIGNED_SETTING, CANCELLED, COMPLETED, DEAL, ENVIRONMENT_SETTING,
    EXPIRED, PRODUCTION, REFUSED, SANDBOX, SIGNED, SIGNED_FILE_ATTACHED, job_kind, job_unique_key,
    secret, settings, user_uuid,
};

pub(crate) const NOT_CONFIGURED: &str = "esign_not_configured";
pub(crate) const DEAL_NOT_FOUND: &str = "deal_not_found";
pub(crate) const REQUEST_NOT_FOUND: &str = "esign_request_not_found";
pub(crate) const FILE_NOT_ATTACHED: &str = "esign_file_not_attached";
pub(crate) const FILE_NOT_PDF: &str = "esign_file_not_pdf";
pub(crate) const FILE_TOO_LARGE: &str = "esign_file_too_large";
pub(crate) const INVALID_REQUEST: &str = "esign_invalid_request";
pub(crate) const REQUEST_CLOSED: &str = "esign_request_closed";
pub(crate) const ATTACH_JOB: &str = "attach_signed";
pub(crate) const SIGNED_NOT_READY: &str = "esign_signed_document_unavailable";

pub(crate) const MAX_SIGNERS: usize = 20;
pub(crate) const MAX_FILE_BYTES: usize = 20 * 1024 * 1024;
const MAX_MESSAGE_CHARS: usize = 1000;
const ATTACHMENT_PAGE_SIZE: u64 = 100;
const MAX_ATTACHMENT_PAGES: u64 = 20;
const MAX_FILE_READ_BYTES: usize = 64 * 1024 * 1024;

pub(crate) fn request_not_found() -> AppError {
    AppError::not_found_code(
        REQUEST_NOT_FOUND,
        "solicitação de assinatura não encontrada",
    )
}

pub(crate) fn deal_not_found() -> AppError {
    AppError::not_found_code(DEAL_NOT_FOUND, "oportunidade não encontrada")
}

fn not_configured() -> AppError {
    AppError::unavailable_code(
        NOT_CONFIGURED,
        "configure o token de acesso da Clicksign nas configurações da extensão",
    )
}

pub(crate) struct SignerInput {
    pub(crate) name: String,
    pub(crate) email: String,
    pub(crate) contact_id: Option<Uuid>,
}

pub(crate) struct SendInput {
    pub(crate) deal_id: Uuid,
    pub(crate) file_id: Uuid,
    pub(crate) signers: Vec<SignerInput>,
    pub(crate) message: Option<String>,
    pub(crate) quote_id: Option<Uuid>,
}

fn is_missing(error: &AppError) -> bool {
    matches!(
        error,
        AppError::NotFound(_)
            | AppError::Forbidden(_)
            | AppError::Coded {
                kind: CodedKind::NotFound | CodedKind::Forbidden,
                ..
            }
    )
}

pub(crate) async fn ensure_quote(
    ctx: &Ctx<'_>,
    scope: &AccessScope,
    quote_id: Uuid,
    deal_id: Uuid,
) -> Result<(), AppError> {
    let not_found = || AppError::not_found_code("quote_not_found", "quote not found for this deal");
    let quote = match quotes::service::get(&ctx.with_scope(scope), &quote_id.to_string()).await {
        Ok(quote) => quote,
        Err(error) if is_missing(&error) => return Err(not_found()),
        Err(error) => return Err(error),
    };
    if quote.quote.deal_id == deal_id {
        Ok(())
    } else {
        Err(not_found())
    }
}

pub(crate) async fn environment(ctx: &Ctx<'_>) -> Result<String, AppError> {
    let settings = settings(ctx).await?;
    Ok(
        match settings.get(ENVIRONMENT_SETTING).and_then(Value::as_str) {
            Some(PRODUCTION) => PRODUCTION,
            _ => SANDBOX,
        }
        .to_owned(),
    )
}

pub(crate) async fn is_configured(ctx: &Ctx<'_>) -> Result<bool, AppError> {
    Ok(secret(ctx, ACCESS_TOKEN_SETTING)
        .await?
        .is_some_and(|token| !token.trim().is_empty()))
}

pub(crate) async fn build_provider(
    ctx: &Ctx<'_>,
    environment: &str,
) -> Result<Clicksign, AppError> {
    let token = secret(ctx, ACCESS_TOKEN_SETTING)
        .await?
        .map(|token| token.trim().to_owned())
        .filter(|token| !token.is_empty())
        .ok_or_else(not_configured)?;
    Ok(Clicksign::new(environment == PRODUCTION, token))
}

pub(crate) async fn provider(
    ctx: &Ctx<'_>,
    environment: &str,
) -> Result<std::sync::Arc<dyn SignatureProvider>, AppError> {
    aphel_sdk::documents::signature_provider(
        ctx,
        crate::clicksign::PROVIDER_KEY,
        &serde_json::json!({"environment":environment}),
    )
    .await
}

async fn load_record(
    ctx: &Ctx<'_>,
    scope: &AccessScope,
    record_type: &str,
    record_id: Uuid,
) -> Result<Option<Value>, AppError> {
    let Ok(handler) = ctx.registry().handler(record_type) else {
        return Ok(None);
    };
    let state = ctx.state();
    match handler
        .load(&RecordCtx::new(scope, &state), record_id)
        .await
    {
        Ok(value) => Ok(value),
        Err(error) if is_missing(&error) => Ok(None),
        Err(error) => Err(error),
    }
}

pub(crate) async fn load_deal(
    ctx: &Ctx<'_>,
    scope: &AccessScope,
    deal_id: Uuid,
) -> Result<Value, AppError> {
    load_record(ctx, scope, DEAL, deal_id)
        .await?
        .ok_or_else(deal_not_found)
}

pub(crate) async fn ensure_attached(
    ctx: &Ctx<'_>,
    scope: &AccessScope,
    deal_id: Uuid,
    file_id: Uuid,
) -> Result<(), AppError> {
    let files = ctx.files().await?;
    for page in 1..=MAX_ATTACHMENT_PAGES {
        let listing = files
            .list_attachments(
                scope,
                ListAttachmentsQuery {
                    record_type: DEAL.to_owned(),
                    record_id: deal_id.to_string(),
                },
                ListWindow {
                    offset: (page - 1) * ATTACHMENT_PAGE_SIZE,
                    limit: ATTACHMENT_PAGE_SIZE,
                },
            )
            .await?;
        if listing
            .items
            .iter()
            .any(|attachment| attachment.file.id == file_id)
        {
            return Ok(());
        }
        if listing.items.is_empty() || page * ATTACHMENT_PAGE_SIZE >= listing.total {
            break;
        }
    }
    Err(AppError::validation_code(
        FILE_NOT_ATTACHED,
        "o arquivo escolhido não está anexado a esta oportunidade",
    ))
}

pub(crate) struct FileContent {
    pub(crate) file_name: String,
    pub(crate) mime_type: String,
    pub(crate) content: Vec<u8>,
}

pub(crate) async fn read_file(
    ctx: &Ctx<'_>,
    scope: &AccessScope,
    file_id: Uuid,
) -> Result<FileContent, AppError> {
    let download = ctx
        .files()
        .await?
        .download(scope, &file_id.to_string())
        .await?;
    let mut content = Vec::new();
    let mut stream = download.content;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if content.len() + chunk.len() > MAX_FILE_READ_BYTES {
            return Err(AppError::validation_code(
                "file_too_large",
                "the file is too large to be read by a module",
            ));
        }
        content.extend_from_slice(&chunk);
    }
    Ok(FileContent {
        file_name: download.file.original_name,
        mime_type: download.file.mime_type,
        content,
    })
}

pub(crate) async fn record_event(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    scope: &AccessScope,
    event_type: &str,
    deal_id: Uuid,
    payload: Value,
    changes: Vec<FieldChange>,
) -> Result<(), AppError> {
    let actor = Actor::from_scope(scope);
    let now = Utc::now();
    let record = RecordRef::new(RecordType::DEAL, deal_id);
    ctx.events()
        .publish(
            tx,
            DomainEvent::new(actor.clone(), event_type, now)
                .with_record(record.clone())
                .with_payload(payload),
        )
        .await?;
    ctx.audit()
        .record(
            tx,
            AuditEntry::new(actor, event_type, now)
                .with_record(record)
                .with_changes(changes),
        )
        .await
}

fn is_email(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    value.len() <= 254
        && !local.is_empty()
        && !domain.contains('@')
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !value.chars().any(char::is_whitespace)
}

fn validate(input: &SendInput) -> Result<(Vec<SignerInput>, Option<String>), AppError> {
    let mut violations = Vec::new();
    if input.signers.is_empty() {
        violations.push(FieldViolation::new(
            "signers",
            "informe pelo menos um signatário",
        ));
    }
    if input.signers.len() > MAX_SIGNERS {
        violations.push(FieldViolation::new(
            "signers",
            format!("use no máximo {MAX_SIGNERS} signatários"),
        ));
    }
    let mut signers = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for (index, signer) in input.signers.iter().enumerate() {
        let name = signer.name.trim().to_owned();
        let email = signer.email.trim().to_lowercase();
        if name.is_empty() || name.chars().count() > 255 {
            violations.push(FieldViolation::new(
                format!("signers.{index}.name"),
                "informe o nome (até 255 caracteres)",
            ));
        }
        if !is_email(&email) {
            violations.push(FieldViolation::new(
                format!("signers.{index}.email"),
                "informe um e-mail válido",
            ));
        } else if !seen.insert(email.clone()) {
            violations.push(FieldViolation::new(
                format!("signers.{index}.email"),
                "e-mail repetido",
            ));
        }
        signers.push(SignerInput {
            name,
            email,
            contact_id: signer.contact_id,
        });
    }
    let message = input
        .message
        .as_deref()
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .map(str::to_owned);
    if message
        .as_ref()
        .is_some_and(|message| message.chars().count() > MAX_MESSAGE_CHARS)
    {
        violations.push(FieldViolation::new(
            "message",
            format!("use no máximo {MAX_MESSAGE_CHARS} caracteres"),
        ));
    }
    if violations.is_empty() {
        Ok((signers, message))
    } else {
        Err(AppError::invalid_fields_code(
            INVALID_REQUEST,
            "revise os dados da solicitação",
            violations,
        ))
    }
}

fn signer_summary(signers: &[SignerRow]) -> Value {
    Value::Array(
        signers
            .iter()
            .map(|signer| json!({ "name": signer.name, "status": signer.status }))
            .collect(),
    )
}

pub(crate) fn event_payload(request: &RequestRow, signers: &[SignerRow], extra: Value) -> Value {
    let mut payload = json!({
        "request_id": request.id,
        "deal_id": request.deal_id,
        "file_id": request.file_id,
        "file_name": request.file_name,
        "provider": request.provider,
        "status": request.status,
        "signers": signer_summary(signers),
    });
    if let (Some(target), Value::Object(extra)) = (payload.as_object_mut(), extra) {
        target.extend(extra);
    }
    payload
}

pub(crate) async fn send(
    ctx: &Ctx<'_>,
    scope: &AccessScope,
    input: SendInput,
) -> Result<(RequestRow, Vec<SignerRow>), AppError> {
    let (signers, message) = validate(&input)?;
    load_deal(ctx, scope, input.deal_id).await?;
    if let Some(quote_id) = input.quote_id {
        ensure_quote(ctx, scope, quote_id, input.deal_id).await?;
    }
    for signer in &signers {
        if let Some(contact_id) = signer.contact_id {
            load_record(ctx, scope, "people.person", contact_id)
                .await?
                .ok_or_else(|| {
                    AppError::invalid_fields_code(
                        INVALID_REQUEST,
                        "contato não encontrado",
                        vec![FieldViolation::new("signers", "contato não encontrado")],
                    )
                })?;
        }
    }
    let environment = environment(ctx).await?;
    let provider = provider(ctx, &environment).await?;
    ensure_attached(ctx, scope, input.deal_id, input.file_id).await?;
    let file = read_file(ctx, scope, input.file_id).await?;
    let is_pdf = file.mime_type.eq_ignore_ascii_case("application/pdf")
        || file.content.starts_with(b"%PDF-");
    if !is_pdf {
        return Err(AppError::validation_code(
            FILE_NOT_PDF,
            "só é possível enviar arquivos PDF para assinatura",
        ));
    }
    if file.content.len() > MAX_FILE_BYTES {
        return Err(AppError::validation_code(
            FILE_TOO_LARGE,
            "o PDF passa do limite de 20 MB do envio para assinatura",
        ));
    }
    let now = Utc::now();
    let request = RequestRow {
        id: Uuid::now_v7(),
        deal_id: input.deal_id,
        file_id: input.file_id,
        file_name: file.file_name.chars().take(255).collect(),
        provider: provider.key().to_owned(),
        environment,
        provider_envelope_id: String::new(),
        provider_document_id: None,
        status: "queued".to_owned(),
        message,
        signed_file_id: None,
        created_by: user_uuid(scope)?,
        created_at: now,
        updated_at: now,
        completed_at: None,
    };
    let rows: Vec<SignerRow> = signers
        .into_iter()
        .enumerate()
        .map(|(index, signer)| SignerRow {
            id: Uuid::now_v7(),
            request_id: request.id,
            position: u32::try_from(index).unwrap_or(u32::MAX),
            contact_id: signer.contact_id,
            name: signer.name,
            email: signer.email,
            provider_signer_id: None,
            status: SIGNER_PENDING.to_owned(),
            signed_at: None,
            refused_at: None,
        })
        .collect();
    let mut tx = ctx.begin().await?;
    store::insert_request(&mut tx, &request, &rows).await?;
    record_event(
        ctx,
        &mut tx,
        scope,
        crate::QUEUED,
        request.deal_id,
        event_payload(
            &request,
            &rows,
            json!({ "environment": request.environment, "quote_id": input.quote_id }),
        ),
        vec![
            FieldChange::new("request_id", Value::Null, json!(request.id)),
            FieldChange::new("file_id", Value::Null, json!(request.file_id)),
            FieldChange::new("signers", Value::Null, json!(rows.len())),
            FieldChange::new("quote_id", Value::Null, json!(input.quote_id)),
            FieldChange::new("status", Value::Null, json!("queued")),
        ],
    )
    .await?;
    ctx.jobs().enqueue(&mut tx, Job::new("esign.deliver", json!({
        "request_id": request.id, "quote_id": input.quote_id,
        "authority": aphel_kernel::infrastructure::deferred_authority::DeferredAuthority::capture(scope),
    })).with_max_attempts(1).with_unique_key(format!("esign.deliver:{}", request.id))).await?;
    tx.commit().await?;
    Ok((request, rows))
}

pub(crate) async fn cancel(
    ctx: &Ctx<'_>,
    scope: &AccessScope,
    request_id: Uuid,
) -> Result<RequestRow, AppError> {
    let current = store::find_request(ctx.pool(), request_id)
        .await?
        .ok_or_else(request_not_found)?;
    load_deal(ctx, scope, current.deal_id).await?;
    let mut tx = ctx.begin().await?;
    let mut request = store::lock_request(&mut tx, request_id)
        .await?
        .ok_or_else(request_not_found)?;
    let previous = request.status.clone();
    match request.status.as_str() {
        "queued" => request.status = STATUS_CANCELLED.to_owned(),
        STATUS_SENT => {
            request.status = "cancel_pending".to_owned();
            ctx.jobs().enqueue(&mut tx, Job::new("esign.deliver", json!({
                "request_id": request.id, "cancel": true,
                "authority": aphel_kernel::infrastructure::deferred_authority::DeferredAuthority::capture(scope),
            })).with_max_attempts(1).with_unique_key(format!("esign.cancel:{}", request.id))).await?;
        }
        _ => {
            return Err(AppError::conflict_code(
                REQUEST_CLOSED,
                "this signature request cannot be cancelled",
            ));
        }
    }
    store::set_request_status(&mut tx, request.id, &request.status, Utc::now()).await?;
    let signers = store::signers_in_tx(&mut tx, request.id).await?;
    record_event(
        ctx,
        &mut tx,
        scope,
        if request.status == STATUS_CANCELLED {
            CANCELLED
        } else {
            crate::CANCEL_QUEUED
        },
        request.deal_id,
        event_payload(&request, &signers, json!({ "source": "user" })),
        vec![FieldChange::new(
            "status",
            json!(previous),
            json!(request.status),
        )],
    )
    .await?;
    tx.commit().await?;
    Ok(request)
}

pub(crate) async fn deliver_cancel(
    ctx: &Ctx<'_>,
    scope: &AccessScope,
    request_id: Uuid,
) -> Result<RequestRow, AppError> {
    let current = store::find_request(ctx.pool(), request_id)
        .await?
        .ok_or_else(request_not_found)?;
    load_deal(ctx, scope, current.deal_id).await?;
    if current.status != "delivery_unknown" {
        return Err(AppError::conflict_code(
            REQUEST_CLOSED,
            "esta solicitação de assinatura já foi encerrada",
        ));
    }
    let provider = provider(ctx, &current.environment).await?;
    provider.cancel(&current.provider_envelope_id).await?;
    let now = Utc::now();
    let mut tx = ctx.begin().await?;
    let mut request = store::lock_request(&mut tx, request_id)
        .await?
        .ok_or_else(request_not_found)?;
    if request.status != "delivery_unknown" {
        return Err(AppError::conflict_code(
            REQUEST_CLOSED,
            "esta solicitação de assinatura já foi encerrada",
        ));
    }
    store::set_request_status(&mut tx, request.id, STATUS_CANCELLED, now).await?;
    request.status = STATUS_CANCELLED.to_owned();
    let signers = store::signers_in_tx(&mut tx, request.id).await?;
    record_event(
        ctx,
        &mut tx,
        scope,
        CANCELLED,
        request.deal_id,
        event_payload(&request, &signers, json!({ "source": "user" })),
        vec![
            FieldChange::new("request_id", json!(request.id), json!(request.id)),
            FieldChange::new("status", json!(STATUS_SENT), json!(STATUS_CANCELLED)),
        ],
    )
    .await?;
    tx.commit().await?;
    Ok(request)
}

pub(crate) enum NoticeOutcome {
    Applied,
    Ignored,
    Duplicate,
}

pub(crate) async fn apply_notice(
    ctx: &Ctx<'_>,
    scope: &AccessScope,
    request_id: Uuid,
    notice: &WebhookNotice,
    body_hash: &str,
) -> Result<NoticeOutcome, AppError> {
    let mut tx = ctx.begin().await?;
    let fresh =
        store::record_receipt(&mut tx, body_hash, Some(request_id), &notice.event_name).await?;
    if !fresh {
        return Ok(NoticeOutcome::Duplicate);
    }
    let Some(mut request) = store::lock_request(&mut tx, request_id).await? else {
        tx.commit().await?;
        return Ok(NoticeOutcome::Ignored);
    };
    let mut signers = store::signers_in_tx(&mut tx, request.id).await?;
    let now = Utc::now();
    let previous_status = request.status.clone();
    let mut events: Vec<(&'static str, Value)> = Vec::new();
    match notice.kind {
        Some(kind @ (NoticeKind::SignerSigned | NoticeKind::SignerRefused)) => {
            let (status, event) = if kind == NoticeKind::SignerSigned {
                (SIGNER_SIGNED, SIGNED)
            } else {
                (SIGNER_REFUSED, REFUSED)
            };
            let target = notice.signer_email.as_deref().and_then(|email| {
                signers
                    .iter_mut()
                    .find(|signer| signer.email.eq_ignore_ascii_case(email))
            });
            if let Some(signer) = target
                && signer.status == SIGNER_PENDING
            {
                store::set_signer_status(&mut tx, signer.id, status, now).await?;
                signer.status = status.to_owned();
                let name = signer.name.clone();
                if kind == NoticeKind::SignerRefused && request.is_open() {
                    store::set_request_status(&mut tx, request.id, STATUS_REFUSED, now).await?;
                    request.status = STATUS_REFUSED.to_owned();
                }
                events.push((event, json!({ "signer_name": name })));
            }
        }
        Some(NoticeKind::Completed) if request.status != STATUS_COMPLETED => {
            store::set_request_status(&mut tx, request.id, STATUS_COMPLETED, now).await?;
            request.status = STATUS_COMPLETED.to_owned();
            events.push((COMPLETED, json!({})));
        }
        Some(NoticeKind::Cancelled) if request.is_open() => {
            store::set_request_status(&mut tx, request.id, STATUS_CANCELLED, now).await?;
            request.status = STATUS_CANCELLED.to_owned();
            events.push((CANCELLED, json!({ "source": "provider" })));
        }
        Some(NoticeKind::Expired) if request.is_open() => {
            store::set_request_status(&mut tx, request.id, STATUS_EXPIRED, now).await?;
            request.status = STATUS_EXPIRED.to_owned();
            events.push((EXPIRED, json!({})));
        }
        _ => {}
    }
    if events.is_empty() {
        tx.commit().await?;
        return Ok(NoticeOutcome::Ignored);
    }
    for (event, extra) in &events {
        let mut extra = extra.clone();
        if let Value::Object(map) = &mut extra {
            map.entry("source")
                .or_insert_with(|| json!("provider_webhook"));
            map.insert("provider_event".to_owned(), json!(notice.event_name));
        }
        let mut changes = vec![FieldChange::new(
            "request_id",
            json!(request.id),
            json!(request.id),
        )];
        if previous_status != request.status {
            changes.push(FieldChange::new(
                "status",
                json!(previous_status),
                json!(request.status),
            ));
        }
        if let Some(name) = extra.get("signer_name") {
            changes.push(FieldChange::new("signer", Value::Null, name.clone()));
        }
        record_event(
            ctx,
            &mut tx,
            scope,
            event,
            request.deal_id,
            event_payload(&request, &signers, extra.clone()),
            changes,
        )
        .await?;
    }
    if request.status == STATUS_COMPLETED && previous_status != STATUS_COMPLETED {
        let settings = settings(ctx).await?;
        let attach = settings
            .get(ATTACH_SIGNED_SETTING)
            .and_then(Value::as_bool)
            .unwrap_or(true);
        if attach {
            ctx.jobs()
                .enqueue(
                    &mut tx,
                    Job::new(job_kind(ATTACH_JOB), json!({ "request_id": request.id }))
                        .with_unique_key(job_unique_key(&format!("attach:{}", request.id)))
                        .with_max_attempts(5),
                )
                .await?;
        }
    }
    tx.commit().await?;
    Ok(NoticeOutcome::Applied)
}

pub(crate) async fn record_ignored(
    ctx: &Ctx<'_>,
    body_hash: &str,
    event_name: &str,
) -> Result<(), AppError> {
    let mut tx = ctx.begin().await?;
    store::record_receipt(&mut tx, body_hash, None, event_name).await?;
    Ok(tx.commit().await?)
}

pub(crate) async fn attach_signed(ctx: &Ctx<'_>, request_id: Uuid) -> Result<(), AppError> {
    let scope = ctx.scope;
    let Some(request) = store::find_request(ctx.pool(), request_id).await? else {
        return Ok(());
    };
    if request.status != STATUS_COMPLETED || request.signed_file_id.is_some() {
        return Ok(());
    }
    let Some(document_id) = request.provider_document_id.clone() else {
        return Ok(());
    };
    let provider = provider(ctx, &request.environment).await?;
    let Some(content) = provider
        .signed_document(&request.provider_envelope_id, &document_id)
        .await?
    else {
        return Err(AppError::unavailable_code(
            SIGNED_NOT_READY,
            "o PDF assinado ainda não está disponível no provedor",
        ));
    };
    let stem = request
        .file_name
        .strip_suffix(".pdf")
        .or_else(|| request.file_name.strip_suffix(".PDF"))
        .unwrap_or(&request.file_name);
    let file = ctx
        .files()
        .await?
        .upload(
            scope,
            UploadFileCommand {
                file_name: format!("{stem} (assinado).pdf"),
                content: Bytes::from(content),
                attach_to: Some(RecordRef::new(RecordType::DEAL, request.deal_id)),
                ip: None,
            },
        )
        .await?
        .file;
    let mut tx = ctx.begin().await?;
    store::set_signed_file(&mut tx, request.id, file.id).await?;
    let signers = store::signers_in_tx(&mut tx, request.id).await?;
    record_event(
        ctx,
        &mut tx,
        scope,
        SIGNED_FILE_ATTACHED,
        request.deal_id,
        event_payload(
            &request,
            &signers,
            json!({ "signed_file_id": file.id, "signed_file_name": file.original_name }),
        ),
        vec![FieldChange::new(
            "signed_file_id",
            Value::Null,
            json!(file.id),
        )],
    )
    .await?;
    Ok(tx.commit().await?)
}

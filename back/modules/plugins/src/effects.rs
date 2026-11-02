use aphel_contract_crm::CrmPermissions;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{MySqlPool, Row, types::Json};
use uuid::Uuid;

use crate::registry::{self, LoadedPlugin};
use crate::runtime::CallState;
use aphel_kernel::application::shared::{ExecutionContext, Job, JobContext, JobHandler, JobQueue};
use aphel_kernel::database::{
    receipt::{self, MutationReceipt},
    transaction::MySqlTx,
};
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
pub use aphel_kernel::infrastructure::deferred_authority::DeferredAuthority as Authority;
use aphel_kernel::infrastructure::jobs::MySqlJobQueue;
use aphel_kernel::records::RecordCtx;
use aphel_kernel::state::{runtime_pool, runtime_state};

pub const RUN_JOB: &str = "plugin.effect";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Create {
        record_type: String,
        values: Value,
    },
    Update {
        record_type: String,
        id: String,
        values: Value,
    },
    Delete {
        record_type: String,
        id: String,
    },
    Archive {
        record_type: String,
        id: String,
    },
    Restore {
        record_type: String,
        id: String,
    },
    KvSet {
        collection: String,
        id: String,
        value: Value,
    },
    KvDelete {
        collection: String,
        id: String,
    },
    Emit {
        name: String,
        record_type: Option<String>,
        record_id: Option<String>,
        payload: Value,
    },
    Invoke {
        operation: String,
        input: Value,
    },
    Notify {
        name: String,
        recipient_user_id: String,
        title: String,
        body: Option<String>,
        record_type: Option<String>,
        record_id: Option<String>,
        link: Option<String>,
        data: Value,
        #[serde(default)]
        dedupe_key: Option<String>,
    },
    Email {
        to: String,
        #[serde(default)]
        to_name: Option<String>,
        subject: String,
        text: String,
        #[serde(default)]
        html: Option<String>,
        #[serde(default = "transactional")]
        purpose: String,
    },
    Http {
        method: String,
        url: String,
        #[serde(default)]
        headers: Vec<(String, String)>,
        #[serde(default)]
        body: Option<String>,
    },
}

fn transactional() -> String {
    "transactional".to_owned()
}

const MAX_EMAIL_SUBJECT: usize = 200;
const MAX_EMAIL_BODY: usize = 100 * 1024;
const EMAIL_PER_MINUTE: u32 = 120;
const HTTP_EFFECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

static EMAIL_LIMIT: std::sync::LazyLock<aphel_kernel::api::rate_limit::SharedRateLimiter> =
    std::sync::LazyLock::new(|| {
        aphel_kernel::api::rate_limit::SharedRateLimiter::new(
            "plugins.email",
            EMAIL_PER_MINUTE,
            std::time::Duration::from_secs(60),
        )
    });

fn url_host(url: &str) -> Option<String> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let authority = rest.split(['/', '?', '#']).next()?;
    let host = authority.rsplit('@').next()?.split(':').next()?;
    (!host.is_empty()).then(|| host.to_ascii_lowercase())
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\n', "<br>")
}

pub struct PendingEffect {
    id: Uuid,
    plugin: String,
    key: String,
    command: Command,
    authority: Authority,
    execution: ExecutionContext,
}

pub async fn defer(
    state: &mut CallState,
    key: String,
    command_json: String,
) -> Result<String, AppError> {
    if state.read_only {
        return Err(AppError::forbidden_code(
            "plugin_read_only",
            "this plugin execution cannot defer side effects",
        ));
    }
    if key.is_empty() || key.len() > 128 || command_json.len() > 65536 || state.effects.len() >= 100
    {
        return Err(AppError::validation_code(
            "plugin_effect_invalid",
            "effect keys must have 1 to 128 bytes; commands are limited to 64 KiB and 100 per hook",
        ));
    }
    let command: Command = serde_json::from_str(&command_json)
        .map_err(|error| AppError::validation_code("plugin_effect_invalid", error.to_string()))?;
    validate(&state.plugin, &command).await?;
    let execution = ExecutionContext::current()
        .ok_or_else(|| AppError::Internal("plugin execution context is missing".to_owned()))?;
    if let Some(existing) = state.effects.iter().find(|effect| effect.key == key) {
        if serde_json::to_value(&existing.command).ok() != serde_json::to_value(&command).ok() {
            return Err(AppError::conflict_code(
                "plugin_effect_key_conflict",
                "the effect key was already used with another command",
            ));
        }
        return Ok(existing.id.to_string());
    }
    let effect = PendingEffect {
        id: Uuid::now_v7(),
        plugin: state.plugin.key.clone(),
        key,
        command,
        authority: Authority::capture(&state.scope),
        execution,
    };
    let id = effect.id.to_string();
    if state.in_transaction {
        state.effects.push(effect);
    } else {
        let pool = runtime_pool()?;
        let mut tx = aphel_kernel::database::transaction::MySqlTx::from(pool.begin().await?);
        let canonical = persist(&mut tx, &pool, vec![effect]).await?;
        tx.commit().await?;
        return canonical
            .into_iter()
            .next()
            .ok_or_else(|| AppError::Internal("effect identifier is missing".to_owned()));
    }
    Ok(id)
}

async fn validate(plugin: &LoadedPlugin, command: &Command) -> Result<(), AppError> {
    match command {
        Command::Create { record_type, .. }
        | Command::Update { record_type, .. }
        | Command::Delete { record_type, .. }
        | Command::Archive { record_type, .. }
        | Command::Restore { record_type, .. } => {
            let handler = runtime_state()?.registry().await?.handler(record_type)?;
            if !handler.durable_mutations() {
                return Err(AppError::validation_code(
                    "plugin_effect_not_supported",
                    format!("{record_type} does not declare single-transaction durable mutations"),
                ));
            }
        }
        Command::KvSet { collection, id, .. } | Command::KvDelete { collection, id } => {
            if collection.is_empty() || collection.len() > 64 || id.is_empty() || id.len() > 128 {
                return Err(AppError::validation_code(
                    "plugin_effect_invalid",
                    "invalid key-value collection or id",
                ));
            }
        }
        Command::Emit { name, .. } => {
            if !plugin
                .manifest
                .emits
                .iter()
                .any(|event| event.name == *name)
            {
                return Err(AppError::forbidden_code(
                    "plugin_event_not_declared",
                    "the emitted event must be declared",
                ));
            }
        }
        Command::Notify { name, .. } => {
            if !plugin
                .manifest
                .notifications
                .iter()
                .any(|notification| notification.name == *name)
            {
                return Err(AppError::forbidden_code(
                    "plugin_notification_not_declared",
                    "the notification must be declared",
                ));
            }
        }
        Command::Email {
            to,
            subject,
            text,
            html,
            purpose,
            ..
        } => {
            if !plugin
                .manifest
                .allows(&aphel_kernel::domain::access::Permission::EMAILS_SEND)
            {
                return Err(AppError::forbidden_code(
                    "plugin_email_not_allowed",
                    "declare emails.send in requires to send e-mail",
                ));
            }
            aphel_kernel::domain::Email::new(to.trim().to_lowercase())?;
            if !matches!(purpose.as_str(), "transactional" | "marketing") {
                return Err(AppError::validation_code(
                    "plugin_effect_invalid",
                    "purpose must be transactional or marketing",
                ));
            }
            if subject.trim().is_empty()
                || subject.contains(['\r', '\n'])
                || subject.chars().count() > MAX_EMAIL_SUBJECT
                || text.len() > MAX_EMAIL_BODY
                || html
                    .as_ref()
                    .is_some_and(|html| html.len() > MAX_EMAIL_BODY)
            {
                return Err(AppError::validation_code(
                    "plugin_effect_invalid",
                    "the subject must be one line of up to 200 characters and the bodies up to 100 KiB",
                ));
            }
        }
        Command::Http { method, url, .. } => {
            if !["GET", "POST", "PUT", "PATCH", "DELETE"].contains(&method.as_str()) {
                return Err(AppError::validation_code(
                    "plugin_effect_invalid",
                    "the HTTP method must be GET, POST, PUT, PATCH or DELETE",
                ));
            }
            let host = url_host(url).ok_or_else(|| {
                AppError::validation_code("plugin_effect_invalid", "the URL is invalid")
            })?;
            if !plugin.manifest.host_allowed(&host) {
                return Err(AppError::forbidden_code(
                    "plugin_host_not_allowed",
                    format!("{host} is not in [http] allowed_hosts"),
                ));
            }
        }
        Command::Invoke { operation, .. } => {
            if !plugin.manifest.uses_operation(operation)
                || !aphel_kernel::modules::app()
                    .contributions::<aphel_kernel::modules::PluginOperation>()
                    .iter()
                    .any(|candidate| candidate.key == operation && candidate.durable)
            {
                return Err(AppError::validation_code(
                    "plugin_effect_not_supported",
                    "the operation must declare durable execution support",
                ));
            }
        }
    }
    Ok(())
}

pub async fn persist(
    tx: &mut MySqlTx,
    pool: &MySqlPool,
    pending: Vec<PendingEffect>,
) -> Result<Vec<String>, AppError> {
    let queue = MySqlJobQueue::new(pool.clone());
    let mut identifiers = Vec::new();
    for effect in pending {
        sqlx::query(
            "INSERT IGNORE INTO plugins_effect_limits (plugin_key, execution_id) VALUES (?, ?)",
        )
        .bind(&effect.plugin)
        .bind(effect.execution.id.to_string())
        .execute(&mut **tx)
        .await?;
        let count: u32 = sqlx::query_scalar("SELECT effect_count FROM plugins_effect_limits WHERE plugin_key = ? AND execution_id = ? FOR UPDATE")
            .bind(&effect.plugin).bind(effect.execution.id.to_string()).fetch_one(&mut **tx).await?;
        let inserted = sqlx::query("INSERT IGNORE INTO plugins_effects (id, plugin_key, execution_id, effect_key, command, authority, execution) VALUES (?, ?, ?, ?, ?, ?, ?)")
            .bind(effect.id.to_string()).bind(&effect.plugin).bind(effect.execution.id.to_string()).bind(&effect.key)
            .bind(Json(&effect.command)).bind(Json(&effect.authority)).bind(Json(&effect.execution)).execute(&mut **tx).await?.rows_affected();
        if inserted != 0 {
            if count >= 100 {
                return Err(AppError::validation_code(
                    "plugin_effect_limit",
                    "an execution can defer at most 100 distinct effects",
                ));
            }
            sqlx::query("UPDATE plugins_effect_limits SET effect_count = effect_count + 1 WHERE plugin_key = ? AND execution_id = ?")
                .bind(&effect.plugin).bind(effect.execution.id.to_string()).execute(&mut **tx).await?;
        }
        let row = sqlx::query("SELECT id, command FROM plugins_effects WHERE plugin_key = ? AND execution_id = ? AND effect_key = ? FOR UPDATE")
            .bind(&effect.plugin).bind(effect.execution.id.to_string()).bind(&effect.key).fetch_one(&mut **tx).await?;
        let Json(command): Json<Value> = row.try_get("command")?;
        if command
            != serde_json::to_value(&effect.command)
                .map_err(|error| AppError::Internal(error.to_string()))?
        {
            return Err(AppError::conflict_code(
                "plugin_effect_key_conflict",
                "the effect key was already used with another command",
            ));
        }
        let id: String = row.try_get("id")?;
        queue
            .enqueue(
                tx,
                Job::new(RUN_JOB, json!({"id": id})).with_unique_key(format!("plugin.effect:{id}")),
            )
            .await?;
        identifiers.push(id);
    }
    Ok(identifiers)
}

pub struct Run {
    pool: MySqlPool,
}

impl Run {
    pub fn shared(pool: MySqlPool) -> Arc<dyn JobHandler> {
        Arc::new(Self { pool })
    }
}

#[async_trait]
impl JobHandler for Run {
    fn kind(&self) -> &str {
        RUN_JOB
    }

    async fn handle(&self, _context: &JobContext, payload: Value) -> Result<(), AppError> {
        let id = payload
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| Uuid::parse_str(id).ok())
            .ok_or_else(|| AppError::Validation("plugin.effect requires an id".to_owned()))?;
        let row = sqlx::query("SELECT plugin_key, command, authority, execution, status FROM plugins_effects WHERE id = ?").bind(id.to_string()).fetch_one(&self.pool).await?;
        if row.try_get::<String, _>("status")? == "completed" {
            return Ok(());
        }
        if receipt::applied(&self.pool, id).await? {
            return complete(&self.pool, id).await;
        }
        let key: String = row.try_get("plugin_key")?;
        let snapshot = registry::current(&self.pool).await?;
        let plugin = snapshot.plugin(&key).ok_or_else(|| {
            AppError::unavailable_code("plugin_disabled", "the plugin is disabled or uninstalled")
        })?;
        let Json(command): Json<Command> = row.try_get("command")?;
        validate(plugin, &command).await?;
        let Json(authority): Json<Authority> = row.try_get("authority")?;
        let scope = authority
            .resolve(&self.pool)
            .await?
            .restricted(|permission| plugin.manifest.allows(permission))
            .acting_as("plugin", &plugin.key);
        let Json(execution): Json<ExecutionContext> = row.try_get("execution")?;
        let result = execution
            .scope(MutationReceipt::scope(
                id,
                execute(&self.pool, plugin, &scope, id, command),
            ))
            .await;
        if receipt::applied(&self.pool, id).await? {
            return complete(&self.pool, id).await;
        }
        let error = result.err().unwrap_or_else(|| {
            AppError::Internal("durable command did not commit a receipt".to_owned())
        });
        sqlx::query("UPDATE plugins_effects SET last_error = ? WHERE id = ?")
            .bind(error.client_message())
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Err(error)
    }
}

async fn complete(pool: &MySqlPool, id: Uuid) -> Result<(), AppError> {
    sqlx::query("UPDATE plugins_effects SET status = 'completed', completed_at = UTC_TIMESTAMP(6), last_error = NULL WHERE id = ?")
        .bind(id.to_string()).execute(pool).await?;
    Ok(())
}

async fn execute(
    pool: &MySqlPool,
    plugin: &LoadedPlugin,
    scope: &AccessScope,
    effect_id: Uuid,
    command: Command,
) -> Result<(), AppError> {
    match command {
        Command::Email {
            to,
            to_name: _,
            subject,
            text,
            html,
            purpose,
        } => {
            use aphel_kernel::application::access::Authorizer;
            scope.require(&aphel_kernel::domain::access::Permission::EMAILS_SEND)?;
            if let Err(refusal) =
                aphel_kernel::application::mail::check_recipient(pool, &to, &purpose).await
            {
                sqlx::query("UPDATE plugins_effects SET last_error = ? WHERE id = ?")
                    .bind(refusal.client_message())
                    .bind(effect_id.to_string())
                    .execute(pool)
                    .await?;
            } else {
                EMAIL_LIMIT
                    .check(
                        &plugin.key,
                        "the plugin is sending too many e-mails; retrying later",
                    )
                    .await?;
                let html = html.unwrap_or_else(|| format!("<p>{}</p>", escape_html(&text)));
                runtime_state()?
                    .mailer()
                    .send(aphel_kernel::application::mail::EmailMessage {
                        to,
                        subject,
                        html,
                        text,
                        calendar: None,
                    })
                    .await?;
            }
        }
        Command::Http {
            method,
            url,
            mut headers,
            body,
        } => {
            headers.retain(|(name, _)| !name.eq_ignore_ascii_case("idempotency-key"));
            headers.push(("Idempotency-Key".to_owned(), effect_id.to_string()));
            let response = crate::net::fetch(
                plugin,
                std::time::Instant::now() + HTTP_EFFECT_TIMEOUT,
                crate::runtime::wit::HttpRequest {
                    method,
                    url,
                    headers,
                    body: body.map(String::into_bytes),
                },
            )
            .await?;
            if !(200..300).contains(&response.status) {
                return Err(AppError::unavailable_code(
                    "plugin_http_effect_failed",
                    format!(
                        "the remote service answered {}; the request will be retried with the same Idempotency-Key",
                        response.status
                    ),
                ));
            }
        }
        Command::Create {
            record_type,
            values,
        } => {
            crate::host::write(scope, &record_type, None, &values.to_string()).await?;
        }
        Command::Update {
            record_type,
            id,
            values,
        } => {
            crate::host::write(scope, &record_type, Some(&id), &values.to_string()).await?;
        }
        Command::Delete { record_type, id } => {
            crate::host::delete(scope, &record_type, &id).await?
        }
        Command::Archive { record_type, id } => {
            let state = runtime_state()?;
            let registry = state.registry().await?;
            let handler = registry.handler(&record_type)?;
            let id = Uuid::parse_str(&id)
                .map_err(|_| AppError::Validation("invalid record id".to_owned()))?;
            let ctx = RecordCtx::new(scope, &state);
            handler.archive(&ctx, id).await?;
        }
        Command::Restore { record_type, id } => {
            let state = runtime_state()?;
            let registry = state.registry().await?;
            let handler = registry.handler(&record_type)?;
            let id = Uuid::parse_str(&id)
                .map_err(|_| AppError::Validation("invalid record id".to_owned()))?;
            handler.restore(&RecordCtx::new(scope, &state), id).await?;
        }
        Command::KvSet {
            collection,
            id,
            value,
        } => {
            let mut tx = receipt::begin(pool).await?;
            sqlx::query("INSERT INTO plugins_data (plugin_key, collection, item_id, value) VALUES (?, ?, ?, ?) ON DUPLICATE KEY UPDATE value = VALUES(value)")
                .bind(&plugin.key).bind(collection).bind(id).bind(Json(value)).execute(&mut *tx).await?;
            tx.commit().await?;
        }
        Command::KvDelete { collection, id } => {
            let mut tx = receipt::begin(pool).await?;
            sqlx::query(
                "DELETE FROM plugins_data WHERE plugin_key = ? AND collection = ? AND item_id = ?",
            )
            .bind(&plugin.key)
            .bind(collection)
            .bind(id)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
        }
        Command::Emit {
            name,
            record_type,
            record_id,
            payload,
        } => {
            crate::host::emit(
                plugin,
                scope,
                &name,
                record_type,
                record_id,
                &payload.to_string(),
            )
            .await?
        }
        Command::Notify {
            name,
            recipient_user_id,
            title,
            body,
            record_type,
            record_id,
            link,
            data,
            dedupe_key,
        } => {
            crate::notify::notify(
                plugin,
                scope,
                crate::runtime::wit::Notification {
                    kind: name,
                    recipient_user_id,
                    title,
                    body,
                    record_type,
                    record_id,
                    link,
                    data_json: data.to_string(),
                    dedupe_key,
                },
            )
            .await?;
        }
        Command::Invoke { operation, input } => {
            crate::bridge::invoke(
                plugin,
                scope,
                false,
                std::time::Instant::now() + std::time::Duration::from_secs(30),
                &operation,
                &input.to_string(),
            )
            .await?;
        }
    }
    receipt::finish_noop(pool).await?;
    Ok(())
}

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::domain::AssignmentSource;
use crate::service::{
    self, AssignOutcome, AssignRequest, OwnerWriter, RuleListQuery, flatten_values,
};
use aphel_kernel::Ctx;
use aphel_kernel::application::organization_admin::ListWindow;
use aphel_kernel::application::shared::{RecordRef, RecordType};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::modules::{OperationFuture, PluginOperation, operation_input};
use aphel_kernel::records::{PendingCommit, RecordCtx, RecordTypeHandler};
use aphel_kernel::state::{AppState, runtime_state};

const OWNER_FIELD: &str = "owner_id";
const PLUGIN_IDEMPOTENCY_PREFIX: &str = "plugin:";
const MAX_IDEMPOTENCY_KEY_LENGTH: usize = 120;

#[derive(Deserialize)]
struct AssignInput {
    record_type: String,
    record_id: String,
    #[serde(default)]
    rule: Option<String>,
    #[serde(default)]
    idempotency_key: Option<String>,
}

fn invalid(field: &str, message: &str) -> AppError {
    AppError::invalid_fields(
        "invalid lead distribution request",
        vec![FieldViolation::new(field, message.to_owned())],
    )
}

async fn rule_for(ctx: &Ctx<'_>, record_type: &str) -> Result<String, AppError> {
    let listing = service::list(
        ctx,
        RuleListQuery {
            record_type: Some(record_type.to_owned()),
            active: Some(true),
        },
        ListWindow {
            offset: 0,
            limit: 2,
        },
    )
    .await?;
    match listing.items.as_slice() {
        [only] => Ok(only.rule.id.to_string()),
        [] => Err(invalid(
            "rule",
            "no active rule distributes this record type",
        )),
        _ => Err(invalid(
            "rule",
            "is required when more than one active rule distributes this record type",
        )),
    }
}

struct OwnerUpdate<'a> {
    handler: &'a dyn RecordTypeHandler,
    state: &'a AppState,
    scope: &'a AccessScope,
    record_id: Uuid,
}

#[async_trait]
impl OwnerWriter for OwnerUpdate<'_> {
    async fn write(
        &self,
        tx: &mut MySqlTx,
        user_id: UserId,
    ) -> Result<Option<PendingCommit>, AppError> {
        self.handler
            .update_in_tx(
                &RecordCtx::new(self.scope, self.state),
                tx,
                self.record_id,
                json!({ OWNER_FIELD: user_id.into_inner() }),
            )
            .await
    }
}

fn assign<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: AssignInput = operation_input(input)?;
        let record_type = input.record_type.trim().to_owned();
        let record_id = Uuid::parse_str(input.record_id.trim())
            .map_err(|_| invalid("record_id", "must be a valid id"))?;
        let idempotency_key = match input.idempotency_key.as_deref().map(str::trim) {
            Some(key) if key.is_empty() || key.chars().count() > MAX_IDEMPOTENCY_KEY_LENGTH => {
                return Err(invalid(
                    "idempotency_key",
                    "must have between 1 and 120 characters",
                ));
            }
            Some(key) => Some(format!("{PLUGIN_IDEMPOTENCY_PREFIX}{key}")),
            None => None,
        };
        let handler = ctx.registry().handler(&record_type)?;
        let state = runtime_state()?;
        let scope = ctx.scope;
        let record = handler
            .load(&RecordCtx::new(scope, &state), record_id)
            .await?
            .ok_or_else(|| AppError::not_found_code("record_not_found", "record not found"))?;
        let values: Map<String, Value> = flatten_values(&record);
        let rule_id = match input.rule {
            Some(rule) => rule,
            None => rule_for(ctx, &record_type).await?,
        };
        let reference = RecordRef::new(RecordType::parse(record_type.clone())?, record_id);
        let outcome = service::assign(
            ctx,
            AssignRequest {
                rule_id: &rule_id,
                record_type: &record_type,
                record: Some(reference),
                values: &values,
                source: AssignmentSource::Plugin,
                idempotency_key,
            },
            &OwnerUpdate {
                handler: handler.as_ref(),
                state: &state,
                scope,
                record_id,
            },
        )
        .await?;
        Ok(match outcome {
            AssignOutcome::Assigned {
                recipient,
                selection,
                reused,
            } => json!({
                "rule_id": rule_id,
                "user_id": recipient.user_id.into_inner(),
                "user_name": recipient.name,
                "selection": selection.as_str(),
                "route_index": selection.route_index(),
                "reused": reused,
                "skipped": Value::Null,
            }),
            AssignOutcome::Skipped(cause) => json!({
                "rule_id": rule_id,
                "user_id": Value::Null,
                "skipped": cause.as_str(),
            }),
        })
    })
}

pub(crate) fn operations() -> Vec<Arc<PluginOperation>> {
    vec![Arc::new(PluginOperation::write(
        "lead_distribution.assign",
        "Pick the next owner of a record with a lead distribution rule and set its owner_id (needs update access to the record): { record_type, record_id, rule? (rule id; optional when exactly one active rule distributes the type), idempotency_key? } -> { rule_id, user_id, user_name, selection, route_index, reused, skipped }",
        assign,
    )
    .with_durable_mutations())]
}

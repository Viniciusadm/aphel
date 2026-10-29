use std::sync::Arc;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::http::{RecordScoreResponse, ScoringSignalSchema};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::modules::{OperationFuture, PluginOperation, operation_input, operation_output};

#[derive(Deserialize)]
struct RecordInput {
    record_type: String,
    record_id: String,
}

#[derive(Deserialize)]
struct SignalInput {
    record_type: String,
    record_id: String,
    signal: ScoringSignalSchema,
    source: String,
}

fn scores<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: RecordInput = operation_input(input)?;
        let scores = service::record_scores(ctx, &input.record_type, &input.record_id).await?;
        let scores: Vec<RecordScoreResponse> = scores.into_iter().map(Into::into).collect();
        operation_output(scores)
    })
}

fn signal<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: SignalInput = operation_input(input)?;
        let recorded = service::record_signal(
            ctx,
            &input.record_type,
            &input.record_id,
            input.signal.into(),
            &input.source,
        )
        .await?;
        Ok(json!({ "recorded": recorded }))
    })
}

pub(crate) fn operations() -> Vec<Arc<PluginOperation>> {
    vec![
        Arc::new(PluginOperation::read(
            "scoring.scores",
            "Scores of a record in every active scoring model of its type, primary first: { record_type, record_id } -> [{ model_id, model_key, model_name, is_primary, score, band, warm_from, hot_from, explanation, calculated_at }]",
            scores,
        )),
        Arc::new(PluginOperation::write(
            "scoring.signal",
            "Record a behavior signal on a record the actor can update and recalculate its scores; idempotent per record, signal and source: { record_type, record_id, signal: email_opened|email_clicked|email_replied|meeting_booked|form_submitted|call_connected, source } -> { recorded }",
            signal,
        )),
    ]
}

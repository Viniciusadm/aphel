use std::sync::Arc;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::domain::SegmentMembersQuery;
use crate::dto::{SegmentListParams, SegmentMemberResponse, SegmentResponse};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::application::query::{ArchivedFilter, Filter, parse_sort};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::modules::{
    OperationFuture, OperationPage, PluginOperation, operation_input, operation_output,
};

#[derive(Deserialize)]
struct ListInput {
    #[serde(flatten)]
    filter: SegmentListParams,
    #[serde(flatten)]
    page: OperationPage,
}

#[derive(Deserialize)]
struct SegmentId {
    id: String,
}

#[derive(Deserialize)]
struct MembersInput {
    id: String,
    #[serde(default)]
    filter: Option<Value>,
    #[serde(default)]
    sort: Option<String>,
    #[serde(default)]
    search: Option<String>,
    #[serde(default)]
    archived: Option<String>,
    #[serde(flatten)]
    page: OperationPage,
}

impl MembersInput {
    fn query(&self) -> Result<SegmentMembersQuery, AppError> {
        let mut violations = Vec::new();
        let filter = match &self.filter {
            None | Some(Value::Null) => None,
            Some(value) => match serde_json::from_value::<Filter>(value.clone()) {
                Ok(filter) => Some(filter),
                Err(_) => {
                    violations.push(FieldViolation::new(
                        "filter",
                        "must be a filter group or condition",
                    ));
                    None
                }
            },
        };
        let archived = match self.archived.as_deref().map(str::trim) {
            None | Some("") => ArchivedFilter::Exclude,
            Some(value) => ArchivedFilter::parse(value).unwrap_or_else(|| {
                violations.push(FieldViolation::new(
                    "archived",
                    "must be exclude, include or only",
                ));
                ArchivedFilter::Exclude
            }),
        };
        if !violations.is_empty() {
            return Err(AppError::invalid_fields("invalid query", violations));
        }
        Ok(SegmentMembersQuery {
            filter,
            sort: self.sort.as_deref().map(parse_sort).unwrap_or_default(),
            search: self.search.clone(),
            archived,
            window: self.page.window(),
        })
    }
}

fn list<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: ListInput = operation_input(input)?;
        let segments = service::list(
            ctx,
            input.filter.record_type.as_deref(),
            input.filter.counts.unwrap_or(false),
        )
        .await?;
        let total = segments.len() as u64;
        let window = input.page.window();
        let segments: Vec<SegmentResponse> = segments
            .into_iter()
            .skip(usize::try_from(window.offset).unwrap_or(usize::MAX))
            .take(usize::try_from(window.limit).unwrap_or(usize::MAX))
            .map(Into::into)
            .collect();
        Ok(json!({ "data": operation_output(segments)?, "total": total }))
    })
}

fn get<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: SegmentId = operation_input(input)?;
        operation_output(SegmentResponse::from(service::get(ctx, &input.id).await?))
    })
}

fn members<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: MembersInput = operation_input(input)?;
        let listing = service::members(ctx, &input.id, input.query()?).await?;
        let members: Vec<SegmentMemberResponse> =
            listing.items.into_iter().map(Into::into).collect();
        Ok(json!({ "data": operation_output(members)?, "total": listing.total }))
    })
}

pub(crate) fn operations() -> Vec<Arc<PluginOperation>> {
    vec![
        Arc::new(PluginOperation::read(
            "segments.list",
            "Segments of the record types the actor can read: { record_type?, counts?, page?, per_page? } -> { data, total }",
            list,
        )),
        Arc::new(PluginOperation::read(
            "segments.get",
            "One segment with its member count: { id }",
            get,
        )),
        Arc::new(PluginOperation::read(
            "segments.members",
            "Records of a segment visible to the actor: { id, filter? (filter AST), sort? (e.g. -created_at,name), search?, archived?: exclude|include|only, page?, per_page? } -> { data, total }",
            members,
        )),
    ]
}

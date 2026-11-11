use aphel_contract_site::{PageSource, PathPattern, RESERVED_PAGE_PREFIXES, SiteBundle};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::modules::HookCtx;
use aphel_kernel::records::{ChangedRecord, query_ports};
use aphel_sdk::platform::application::organization_admin::ListWindow;
use aphel_sdk::platform::application::query::{
    ArchivedFilter, Filter, FilterOperator, RecordQuery, RecordQueryEngine,
};
use aphel_sdk::platform::domain::access::AccessScope;
use serde_json::Value;

pub(crate) const PATH_TAKEN: &str = "site_path_taken";
const FIXED_SEGMENTS: &[&str] = &["sitemap.xml", "robots.txt", "_aphel", "api"];

struct Claim<'b> {
    entity: &'b str,
    field: &'b str,
}

fn first_literal(path: &str) -> Option<String> {
    let pattern = PathPattern::parse(path).ok()?;
    let shape = pattern.shape();
    let first = shape.trim_start_matches('/').split('/').next()?.to_owned();
    (!first.is_empty() && first != "{}").then_some(first)
}

fn segment_routes(bundles: &[SiteBundle]) -> Vec<Claim<'_>> {
    bundles
        .iter()
        .flat_map(|bundle| &bundle.pages)
        .filter(|page| page.prefix.is_none())
        .filter_map(|page| match &page.source {
            PageSource::Record {
                entity,
                match_field,
            } if PathPattern::parse(&page.path).is_ok_and(|pattern| pattern.shape() == "/{}") => {
                Some(Claim {
                    entity,
                    field: match_field,
                })
            }
            _ => None,
        })
        .collect()
}

fn prefix_claims(bundles: &[SiteBundle]) -> Vec<Claim<'_>> {
    bundles
        .iter()
        .flat_map(|bundle| &bundle.pages)
        .filter_map(|page| page.prefix.as_ref())
        .map(|prefix| Claim {
            entity: &prefix.entity,
            field: &prefix.field,
        })
        .collect()
}

async fn taken_by(claim: &Claim<'_>, value: &str, except: &str) -> Result<bool, AppError> {
    let state = aphel_sdk::runtime()?;
    let ports = query_ports(state.as_ref()).await?;
    let scope = AccessScope::system();
    let listing = RecordQueryEngine::new(&ports)
        .query(
            &scope,
            claim.entity,
            RecordQuery {
                filter: Some(Filter::and(vec![
                    Filter::condition(
                        claim.field,
                        FilterOperator::Eq,
                        Value::String(value.to_owned()),
                    ),
                    Filter::condition("id", FilterOperator::Neq, Value::String(except.to_owned())),
                ])),
                archived: ArchivedFilter::Include,
                window: ListWindow {
                    offset: 0,
                    limit: 1,
                },
                ..Default::default()
            },
        )
        .await?;
    Ok(!listing.items.is_empty())
}

fn taken(field: &str) -> AppError {
    AppError::invalid_fields_code(
        PATH_TAKEN,
        "this address is already used on the site",
        vec![FieldViolation::new(
            field,
            "is already used by another page of the site",
        )],
    )
}

async fn serialize_claims(ctx: &mut HookCtx<'_>) -> Result<(), AppError> {
    sqlx::query("SELECT id FROM site_path_lock WHERE id = 1 FOR UPDATE")
        .execute(&mut **ctx.tx)
        .await?;
    Ok(())
}

pub(crate) async fn guard(ctx: &mut HookCtx<'_>, args: &ChangedRecord) -> Result<(), AppError> {
    let record_type = args.record_type.as_str();
    let bundles = match crate::invalidation::site_bundles().await {
        Ok(bundles) => bundles,
        Err(_) => return Ok(()),
    };
    let prefixes = prefix_claims(&bundles);
    let segments = segment_routes(&bundles);
    let record_id = args.record.record_id.to_string();
    let value_of = |field: &str| {
        args.values
            .get(field)
            .and_then(Value::as_str)
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    };
    let claims_value = prefixes
        .iter()
        .chain(&segments)
        .any(|claim| claim.entity == record_type && value_of(claim.field).is_some());
    if !claims_value {
        return Ok(());
    }
    serialize_claims(ctx).await?;
    for claim in prefixes.iter().filter(|claim| claim.entity == record_type) {
        let Some(value) = value_of(claim.field) else {
            continue;
        };
        let literal = bundles
            .iter()
            .flat_map(|bundle| &bundle.pages)
            .filter(|page| page.prefix.is_none())
            .filter_map(|page| first_literal(&page.path))
            .any(|literal| literal == value);
        let reserved = FIXED_SEGMENTS.contains(&value.as_str())
            || RESERVED_PAGE_PREFIXES
                .iter()
                .any(|prefix| prefix.trim_start_matches('/') == value);
        if literal || reserved {
            return Err(taken(claim.field));
        }
        for other in &segments {
            if taken_by(other, &value, &record_id).await? {
                return Err(taken(claim.field));
            }
        }
    }
    for claim in segments.iter().filter(|claim| claim.entity == record_type) {
        let Some(value) = value_of(claim.field) else {
            continue;
        };
        for other in &prefixes {
            if taken_by(other, &value, &record_id).await? {
                return Err(taken(claim.field));
            }
        }
    }
    Ok(())
}

use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use std::collections::{HashMap, HashSet};

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde_json::json;
use uuid::Uuid;

use crate::domain::company_not_found;
use crate::hierarchy::{HierarchyEntry, HierarchyStore, MAX_HIERARCHY_DEPTH};
use crate::store::CompanyStore;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::application::fields::permissions::{FieldAccess, FieldAccessPolicy};
use aphel_sdk::platform::application::files::UserReference;
use aphel_sdk::platform::application::query::{
    AggregateEngine, AggregateFunction, AggregateQuery, AggregateResult, Filter, FilterOperator,
    GroupBySpec, MAX_AGGREGATE_LIMIT, MetricSpec, RecordQueryPorts,
};
use aphel_sdk::platform::application::shared::RecordType;
use aphel_sdk::platform::domain::access::Permission;
use aphel_sdk::platform::records::parse_record_id;

pub(crate) const MAX_GROUP_COMPANIES: usize = 1000;

const COUNT: &str = "count";
const AMOUNT_SUM: &str = "sum:amount_base";
const COMPANY_FIELD: &str = "company_id";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct HierarchyNode {
    pub(crate) id: Uuid,
    pub(crate) parent_id: Option<Uuid>,
    pub(crate) depth: u32,
    pub(crate) accessible: bool,
    pub(crate) name: Option<String>,
    pub(crate) owner: Option<UserReference>,
    pub(crate) archived: bool,
    pub(crate) subsidiaries: u32,
    pub(crate) contacts_count: Option<u64>,
    pub(crate) open_deals_count: Option<u64>,
    pub(crate) open_deals_amount: Option<Decimal>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompanyHierarchy {
    pub(crate) company_id: Uuid,
    pub(crate) root_id: Uuid,
    pub(crate) max_depth: usize,
    pub(crate) truncated: bool,
    pub(crate) currency: Option<String>,
    pub(crate) nodes: Vec<HierarchyNode>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GroupTotals {
    pub(crate) company_id: Uuid,
    pub(crate) companies_count: u64,
    pub(crate) currency: Option<String>,
    pub(crate) contacts_count: Option<u64>,
    pub(crate) open_deals_count: Option<u64>,
    pub(crate) open_deals_amount: Option<Decimal>,
    pub(crate) won_deals_count: Option<u64>,
    pub(crate) won_amount: Option<Decimal>,
}

#[derive(Debug, Default)]
struct DealTotals {
    count: Option<u64>,
    amount: Option<Decimal>,
}

fn whole(value: Option<Decimal>) -> u64 {
    value.and_then(|value| value.trunc().to_u64()).unwrap_or(0)
}

fn with_limit(mut query: AggregateQuery) -> AggregateQuery {
    query.limit = Some(MAX_AGGREGATE_LIMIT);
    query
}

fn group_filter(company_id: Uuid) -> Filter {
    Filter::condition(
        COMPANY_FIELD,
        FilterOperator::InGroup,
        json!(company_id.to_string()),
    )
}

fn grouped_counts(result: &AggregateResult, key: &str) -> HashMap<Uuid, Decimal> {
    let Some(index) = result.metric_index(key) else {
        return HashMap::new();
    };
    result
        .rows
        .iter()
        .filter_map(|row| {
            let id = row
                .groups
                .first()
                .and_then(|group| group.value.as_deref())
                .and_then(|value| Uuid::parse_str(value).ok())?;
            Some((id, row.values.get(index).copied().flatten()?))
        })
        .collect()
}

fn order_depth_first(entries: &[HierarchyEntry], names: &HashMap<Uuid, String>) -> Vec<Uuid> {
    let mut children: HashMap<Option<Uuid>, Vec<Uuid>> = HashMap::new();
    let ids: HashSet<Uuid> = entries.iter().map(|entry| entry.id).collect();
    let mut roots = Vec::new();
    for entry in entries {
        match entry.parent_id.filter(|parent| ids.contains(parent)) {
            Some(parent) => children.entry(Some(parent)).or_default().push(entry.id),
            None => roots.push(entry.id),
        }
    }
    let sort = |list: &mut Vec<Uuid>| {
        list.sort_by(|left, right| {
            let left_name = names.get(left).map(String::as_str).unwrap_or("\u{10FFFF}");
            let right_name = names.get(right).map(String::as_str).unwrap_or("\u{10FFFF}");
            left_name
                .to_lowercase()
                .cmp(&right_name.to_lowercase())
                .then(left.cmp(right))
        });
    };
    sort(&mut roots);
    for list in children.values_mut() {
        sort(list);
    }
    let mut ordered = Vec::with_capacity(entries.len());
    let mut stack: Vec<Uuid> = roots.into_iter().rev().collect();
    while let Some(id) = stack.pop() {
        ordered.push(id);
        if let Some(list) = children.get(&Some(id)) {
            stack.extend(list.iter().rev());
        }
    }
    ordered
}

async fn visible_company(ctx: &Ctx<'_>, company_id: &str) -> Result<Uuid, AppError> {
    let scope = ctx.scope;
    let visibility = scope.record_filter(&Permission::COMPANIES_READ)?;
    let id = parse_record_id(company_id, company_not_found)?;
    CompanyStore::of(ctx)
        .find(scope, &visibility, id)
        .await?
        .map(|view| view.company.id)
        .ok_or_else(company_not_found)
}

async fn amount_visible(ctx: &Ctx<'_>) -> Result<bool, AppError> {
    let scope = ctx.scope;
    let policy =
        FieldAccessPolicy::load(ctx.query().definitions(), scope, &RecordType::DEAL).await?;
    Ok(policy.key_access("amount_base") != FieldAccess::Hidden)
}

async fn aggregate(ctx: &Ctx<'_>, query: AggregateQuery) -> Result<AggregateResult, AppError> {
    let scope = ctx.scope;
    AggregateEngine::new(&ctx.query()).run(scope, query).await
}

async fn deal_totals(
    ctx: &Ctx<'_>,
    company_id: Uuid,
    status: &str,
    with_amount: bool,
) -> Result<DealTotals, AppError> {
    let mut metrics = vec![MetricSpec::count()];
    if with_amount {
        metrics.push(MetricSpec::of(AggregateFunction::Sum, "amount_base"));
    }
    let result = aggregate(
        ctx,
        AggregateQuery::new(RecordType::DEAL.as_str())
            .filter(Filter::and(vec![
                group_filter(company_id),
                Filter::condition("status", FilterOperator::Eq, json!(status)),
            ]))
            .metrics(metrics),
    )
    .await?;
    Ok(DealTotals {
        count: Some(whole(result.total(COUNT))),
        amount: with_amount.then(|| result.total(AMOUNT_SUM).unwrap_or_default()),
    })
}

pub(crate) async fn group_totals(ctx: &Ctx<'_>, company_id: &str) -> Result<GroupTotals, AppError> {
    let scope = ctx.scope;
    let id = visible_company(ctx, company_id).await?;
    let visibility = scope.record_filter(&Permission::COMPANIES_READ)?;
    let members: Vec<Uuid> = HierarchyStore::of(ctx)
        .subtree(id)
        .await?
        .into_iter()
        .map(|entry| entry.id)
        .collect();
    let companies_count = HierarchyStore::of(ctx)
        .visible_ids(scope, &visibility, &members)
        .await?
        .len() as u64;
    let contacts_count = if scope.filter_for(&Permission::CONTACTS_READ).is_some() {
        let result = aggregate(
            ctx,
            AggregateQuery::new(RecordType::CONTACT.as_str())
                .filter(group_filter(id))
                .metrics(vec![MetricSpec::count()]),
        )
        .await?;
        Some(whole(result.total(COUNT)))
    } else {
        None
    };
    let (open, won) = if scope.filter_for(&Permission::DEALS_READ).is_some() {
        let with_amount = amount_visible(ctx).await?;
        (
            deal_totals(ctx, id, "open", with_amount).await?,
            deal_totals(ctx, id, "won", with_amount).await?,
        )
    } else {
        (DealTotals::default(), DealTotals::default())
    };
    Ok(GroupTotals {
        company_id: id,
        companies_count,
        currency: HierarchyStore::of(ctx).base_currency().await?,
        contacts_count,
        open_deals_count: open.count,
        open_deals_amount: open.amount,
        won_deals_count: won.count,
        won_amount: won.amount,
    })
}

pub(crate) async fn hierarchy(
    ctx: &Ctx<'_>,
    company_id: &str,
) -> Result<CompanyHierarchy, AppError> {
    let scope = ctx.scope;
    let id = visible_company(ctx, company_id).await?;
    let visibility = scope.record_filter(&Permission::COMPANIES_READ)?;
    let reader = HierarchyStore::of(ctx);
    let root_id = reader.lineage(id).await?.last().copied().unwrap_or(id);
    let mut entries = reader.subtree(root_id).await?;
    let truncated = entries.len() > MAX_GROUP_COMPANIES;
    entries.truncate(MAX_GROUP_COMPANIES);
    let all_ids: Vec<Uuid> = entries.iter().map(|entry| entry.id).collect();
    let readable: HashSet<Uuid> = reader
        .visible_ids(scope, &visibility, &all_ids)
        .await?
        .into_iter()
        .collect();
    let parents: HashMap<Uuid, Option<Uuid>> = entries
        .iter()
        .map(|entry| (entry.id, entry.parent_id))
        .collect();
    let mut kept: HashSet<Uuid> = HashSet::new();
    for readable_id in &readable {
        let mut cursor = Some(*readable_id);
        while let Some(current) = cursor {
            if !kept.insert(current) {
                break;
            }
            cursor = parents.get(&current).copied().flatten();
        }
    }
    entries.retain(|entry| kept.contains(&entry.id));

    let readable_ids: Vec<Uuid> = readable.iter().copied().collect();
    let views: HashMap<Uuid, _> = CompanyStore::of(ctx)
        .find_many(&readable_ids)
        .await?
        .into_iter()
        .map(|view| (view.company.id, view))
        .collect();

    let people = if scope.filter_for(&Permission::CONTACTS_READ).is_some() {
        let result = aggregate(
            ctx,
            with_limit(
                AggregateQuery::new(RecordType::CONTACT.as_str())
                    .filter(group_filter(root_id))
                    .group(GroupBySpec::field(COMPANY_FIELD))
                    .metrics(vec![MetricSpec::count()]),
            ),
        )
        .await?;
        Some(grouped_counts(&result, COUNT))
    } else {
        None
    };
    let (deals, amounts) = if scope.filter_for(&Permission::DEALS_READ).is_some() {
        let with_amount = amount_visible(ctx).await?;
        let mut metrics = vec![MetricSpec::count()];
        if with_amount {
            metrics.push(MetricSpec::of(AggregateFunction::Sum, "amount_base"));
        }
        let result = aggregate(
            ctx,
            with_limit(
                AggregateQuery::new(RecordType::DEAL.as_str())
                    .filter(Filter::and(vec![
                        group_filter(root_id),
                        Filter::condition("status", FilterOperator::Eq, json!("open")),
                    ]))
                    .group(GroupBySpec::field(COMPANY_FIELD))
                    .metrics(metrics),
            ),
        )
        .await?;
        (
            Some(grouped_counts(&result, COUNT)),
            with_amount.then(|| grouped_counts(&result, AMOUNT_SUM)),
        )
    } else {
        (None, None)
    };

    let names: HashMap<Uuid, String> = views
        .iter()
        .map(|(id, view)| (*id, view.company.data.name.clone()))
        .collect();
    let mut subsidiaries: HashMap<Uuid, u32> = HashMap::new();
    for entry in &entries {
        if let Some(parent) = entry.parent_id {
            *subsidiaries.entry(parent).or_default() += 1;
        }
    }
    let by_id: HashMap<Uuid, HierarchyEntry> =
        entries.iter().map(|entry| (entry.id, *entry)).collect();
    let nodes = order_depth_first(&entries, &names)
        .into_iter()
        .filter_map(|node_id| {
            let entry = by_id.get(&node_id)?;
            let view = views.get(&node_id);
            let accessible = view.is_some();
            let count = |map: &Option<HashMap<Uuid, Decimal>>| {
                map.as_ref()
                    .filter(|_| accessible)
                    .map(|map| whole(map.get(&node_id).copied()))
            };
            Some(HierarchyNode {
                id: node_id,
                parent_id: entry.parent_id,
                depth: entry.depth,
                accessible,
                name: view.map(|view| view.company.data.name.clone()),
                owner: view.and_then(|view| view.owner.clone()),
                archived: view.is_some_and(|view| view.company.is_archived()),
                subsidiaries: subsidiaries.get(&node_id).copied().unwrap_or(0),
                contacts_count: count(&people),
                open_deals_count: count(&deals),
                open_deals_amount: amounts
                    .as_ref()
                    .filter(|_| accessible)
                    .map(|map| map.get(&node_id).copied().unwrap_or_default()),
            })
        })
        .collect();
    Ok(CompanyHierarchy {
        company_id: id,
        root_id,
        max_depth: MAX_HIERARCHY_DEPTH,
        truncated,
        currency: reader.base_currency().await?,
        nodes,
    })
}

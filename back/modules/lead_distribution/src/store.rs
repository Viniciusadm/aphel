use std::collections::HashMap;

use chrono::NaiveDateTime;
use serde_json::{Value, json};
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::domain::{
    AssignmentEntry, AssignmentSource, Counter, DistributionRule, RuleStats, UserStatus,
};
use crate::domain::{DistributionStrategy, Route, Routing, RoutingFallback, RuleMember};
use aphel_kernel::Ctx;
use aphel_kernel::application::fields::{FieldDefinitionRepository, FieldSet};
use aphel_kernel::application::organization_admin::ListWindow;
use aphel_kernel::application::organization_admin::Listing;
use aphel_kernel::application::shared::RecordRef;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::{TeamId, UserId};
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::fields::MySqlFieldDefinitionRepository;
use aphel_kernel::infrastructure::repositories::mapper;
use chrono::{DateTime, Utc};

const RULE_SELECT: &str = "SELECT r.id, r.`key`, r.name, r.description, r.record_type, r.strategy, \
     r.is_active, r.routing_field, r.routes, r.routing_fallback, r.created_by, r.created_at, \
     r.updated_at, s.sequence, s.last_user_id, s.last_assigned_at \
     FROM lead_distribution_rules r \
     LEFT JOIN lead_distribution_states s ON s.rule_id = r.id \
     WHERE 1 = 1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickRecord {
    pub rule_id: Uuid,
    pub sequence: u64,
    pub user_id: UserId,
    pub credits: Vec<(UserId, i64)>,
    pub record: Option<RecordRef>,
    pub source: AssignmentSource,
    pub idempotency_key: Option<String>,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleListFilter {
    pub record_type: Option<String>,
    pub active: Option<bool>,
    pub window: ListWindow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AssignmentFilter {
    pub(crate) rule_id: Option<Uuid>,
    pub(crate) user_id: Option<UserId>,
    pub(crate) record_type: Option<String>,
    pub(crate) source: Option<AssignmentSource>,
    pub(crate) window: ListWindow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DirectoryUser {
    pub(crate) user_id: UserId,
    pub(crate) email: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DirectoryTeam {
    pub(crate) team_id: TeamId,
    pub(crate) name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetInfo {
    pub(crate) has_owner: bool,
    pub(crate) fields: Vec<String>,
}

pub(crate) async fn target(
    ctx: &Ctx<'_>,
    record_type: &str,
) -> Result<Option<TargetInfo>, AppError> {
    let Ok(definition) = ctx.registry().require(record_type) else {
        return Ok(None);
    };
    let definitions = MySqlFieldDefinitionRepository::new(ctx.pool().clone())
        .list(&definition.record_type)
        .await?;
    let fields = FieldSet::new(definition, definitions)
        .active()
        .map(|field| field.key().to_owned())
        .collect();
    Ok(Some(TargetInfo {
        has_owner: definition.owner_column.is_some(),
        fields,
    }))
}

#[derive(Clone)]
pub struct DistributionStore {
    pool: MySqlPool,
}

impl DistributionStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    async fn members(&self, rule_ids: &[Uuid]) -> Result<HashMap<Uuid, Vec<RuleMember>>, AppError> {
        let mut members: HashMap<Uuid, Vec<RuleMember>> = HashMap::new();
        if rule_ids.is_empty() {
            return Ok(members);
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT rule_id, user_id, weight, is_away, away_from, away_until \
             FROM lead_distribution_members WHERE rule_id",
        );
        push_list(&mut builder, rule_ids);
        builder.push(" ORDER BY rule_id, position");
        for row in builder.build().fetch_all(&self.pool).await? {
            let rule_id: String = row.try_get("rule_id")?;
            let rule_id = Uuid::parse_str(&rule_id)
                .map_err(|_| mapper::invalid_column("rule_id", &rule_id))?;
            let weight: u32 = row.try_get("weight")?;
            let away_from: Option<NaiveDateTime> = row.try_get("away_from")?;
            let away_until: Option<NaiveDateTime> = row.try_get("away_until")?;
            members.entry(rule_id).or_default().push(RuleMember {
                user_id: mapper::typed_id(row.try_get("user_id")?, UserId::new)?,
                weight,
                away: row.try_get("is_away")?,
                away_from: away_from.map(mapper::utc),
                away_until: away_until.map(mapper::utc),
            });
        }
        Ok(members)
    }

    async fn with_members(
        &self,
        mut rules: Vec<DistributionRule>,
    ) -> Result<Vec<DistributionRule>, AppError> {
        let ids: Vec<Uuid> = rules.iter().map(|rule| rule.id).collect();
        let mut members = self.members(&ids).await?;
        for rule in &mut rules {
            rule.members = members.remove(&rule.id).unwrap_or_default();
        }
        Ok(rules)
    }

    async fn write_members(
        &self,
        tx: &mut MySqlTx,
        rule: &DistributionRule,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM lead_distribution_members WHERE rule_id = ?")
            .bind(rule.id.to_string())
            .execute(&mut **tx)
            .await?;
        if rule.members.is_empty() {
            return Ok(());
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "INSERT INTO lead_distribution_members \
             (rule_id, user_id, position, weight, is_away, away_from, away_until) ",
        );
        builder.push_values(
            rule.members.iter().enumerate(),
            |mut row, (position, member)| {
                row.push_bind(rule.id.to_string())
                    .push_bind(member.user_id.as_str().to_owned())
                    .push_bind(u32::try_from(position).unwrap_or(u32::MAX))
                    .push_bind(member.weight)
                    .push_bind(member.away)
                    .push_bind(member.away_from.map(mapper::naive))
                    .push_bind(member.away_until.map(mapper::naive));
            },
        );
        builder.build().execute(&mut **tx).await?;
        Ok(())
    }
}

fn push_list<T: ToString>(builder: &mut QueryBuilder<MySql>, values: &[T]) {
    builder.push(" IN (");
    let mut separated = builder.separated(", ");
    for value in values {
        separated.push_bind(value.to_string());
    }
    builder.push(")");
}

fn routes_json(routing: Option<&Routing>) -> Option<Json<Value>> {
    routing.map(|routing| {
        Json(Value::Array(
            routing
                .routes
                .iter()
                .map(|route| {
                    json!({
                        "values": route.values,
                        "team_id": route.team_id.as_ref().map(TeamId::as_str),
                        "user_ids": route.user_ids.iter().map(UserId::as_str).collect::<Vec<_>>(),
                    })
                })
                .collect(),
        ))
    })
}

fn route_from_json(value: &Value) -> Result<Route, AppError> {
    let invalid = || mapper::invalid_column("routes", &value.to_string());
    let values = value
        .get("values")
        .and_then(Value::as_array)
        .ok_or_else(invalid)?
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    let team_id = value
        .get("team_id")
        .and_then(Value::as_str)
        .map(|id| mapper::typed_id(id.to_owned(), TeamId::new))
        .transpose()?;
    let user_ids = value
        .get("user_ids")
        .and_then(Value::as_array)
        .map(|ids| {
            ids.iter()
                .filter_map(Value::as_str)
                .map(|id| mapper::typed_id(id.to_owned(), UserId::new))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();
    Ok(Route {
        values,
        team_id,
        user_ids,
    })
}

fn rule_from_row(row: &MySqlRow) -> Result<DistributionRule, AppError> {
    let id: String = row.try_get("id")?;
    let strategy: String = row.try_get("strategy")?;
    let fallback: String = row.try_get("routing_fallback")?;
    let routing_field: Option<String> = row.try_get("routing_field")?;
    let routes: Option<Json<Value>> = row.try_get("routes")?;
    let routing = match routing_field {
        Some(field) => {
            let routes = match routes {
                Some(Json(Value::Array(items))) => items
                    .iter()
                    .map(route_from_json)
                    .collect::<Result<Vec<_>, _>>()?,
                _ => Vec::new(),
            };
            Some(Routing {
                field,
                routes,
                fallback: RoutingFallback::parse(&fallback)
                    .ok_or_else(|| mapper::invalid_column("routing_fallback", &fallback))?,
            })
        }
        None => None,
    };
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    let updated_at: NaiveDateTime = row.try_get("updated_at")?;
    let sequence: Option<u64> = row.try_get("sequence")?;
    let last_assigned_at: Option<NaiveDateTime> = row.try_get("last_assigned_at")?;
    Ok(DistributionRule {
        id: Uuid::parse_str(&id).map_err(|_| mapper::invalid_column("id", &id))?,
        key: row.try_get("key")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        record_type: row.try_get("record_type")?,
        strategy: DistributionStrategy::parse(&strategy)
            .ok_or_else(|| mapper::invalid_column("strategy", &strategy))?,
        is_active: row.try_get("is_active")?,
        members: Vec::new(),
        routing,
        created_by: row
            .try_get::<Option<String>, _>("created_by")?
            .map(|id| mapper::typed_id(id, UserId::new))
            .transpose()?,
        created_at: mapper::utc(created_at),
        updated_at: mapper::utc(updated_at),
        stats: RuleStats {
            sequence: sequence.unwrap_or(0),
            last_user_id: row
                .try_get::<Option<String>, _>("last_user_id")?
                .map(|id| mapper::typed_id(id, UserId::new))
                .transpose()?,
            last_assigned_at: last_assigned_at.map(mapper::utc),
        },
    })
}

fn counter_from_row(row: &MySqlRow) -> Result<Counter, AppError> {
    Ok(Counter {
        user_id: mapper::typed_id(row.try_get("user_id")?, UserId::new)?,
        last_sequence: row.try_get("last_sequence")?,
        credit: row.try_get("credit")?,
    })
}

fn counters_query(rule_id: Uuid, users: &[UserId], lock: bool) -> QueryBuilder<MySql> {
    let mut builder = QueryBuilder::<MySql>::new(
        "SELECT c.user_id, c.last_sequence, c.credit FROM lead_distribution_counters c \
         WHERE c.rule_id = ",
    );
    builder
        .push_bind(rule_id.to_string())
        .push(" AND c.user_id");
    push_list(&mut builder, users);
    if lock {
        builder.push(" FOR UPDATE");
    }
    builder
}

impl DistributionStore {
    pub async fn list(
        &self,
        filter: &RuleListFilter,
    ) -> Result<Listing<DistributionRule>, AppError> {
        let push_filter = |builder: &mut QueryBuilder<MySql>| {
            if let Some(record_type) = &filter.record_type {
                builder
                    .push(" AND r.record_type = ")
                    .push_bind(record_type.clone());
            }
            if let Some(active) = filter.active {
                builder.push(" AND r.is_active = ").push_bind(active);
            }
        };
        let mut count = QueryBuilder::<MySql>::new(
            "SELECT COUNT(*) FROM lead_distribution_rules r WHERE 1 = 1",
        );
        push_filter(&mut count);
        let total: i64 = count.build_query_scalar().fetch_one(&self.pool).await?;
        if total == 0 {
            return Ok(Listing {
                items: Vec::new(),
                total: 0,
            });
        }
        let mut builder = QueryBuilder::<MySql>::new(RULE_SELECT);
        push_filter(&mut builder);
        builder
            .push(" ORDER BY r.name, r.id LIMIT ")
            .push_bind(filter.window.limit)
            .push(" OFFSET ")
            .push_bind(filter.window.offset);
        let rows = builder.build().fetch_all(&self.pool).await?;
        let rules = rows
            .iter()
            .map(rule_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Listing {
            items: self.with_members(rules).await?,
            total: u64::try_from(total).unwrap_or_default(),
        })
    }

    pub async fn find(&self, rule_id: Uuid) -> Result<Option<DistributionRule>, AppError> {
        let mut builder = QueryBuilder::<MySql>::new(RULE_SELECT);
        builder.push(" AND r.id = ").push_bind(rule_id.to_string());
        let Some(row) = builder.build().fetch_optional(&self.pool).await? else {
            return Ok(None);
        };
        let rule = rule_from_row(&row)?;
        Ok(self.with_members(vec![rule]).await?.into_iter().next())
    }

    pub(crate) async fn key_taken(
        &self,
        key: &str,
        except: Option<Uuid>,
    ) -> Result<bool, AppError> {
        let found: Option<String> = sqlx::query_scalar(
            "SELECT id FROM lead_distribution_rules WHERE `key` = ? \
             AND id <> ? LIMIT 1",
        )
        .bind(key)
        .bind(except.map(|id| id.to_string()).unwrap_or_default())
        .fetch_optional(&self.pool)
        .await?;
        Ok(found.is_some())
    }

    pub(crate) async fn insert(
        &self,
        tx: &mut MySqlTx,
        rule: &DistributionRule,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO lead_distribution_rules (id, `key`, name, description, \
             record_type, strategy, is_active, routing_field, routes, routing_fallback, created_by, \
             created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(rule.id.to_string())
        .bind(&rule.key)
        .bind(&rule.name)
        .bind(&rule.description)
        .bind(&rule.record_type)
        .bind(rule.strategy.as_str())
        .bind(rule.is_active)
        .bind(rule.routing.as_ref().map(|routing| routing.field.clone()))
        .bind(routes_json(rule.routing.as_ref()))
        .bind(
            rule.routing
                .as_ref()
                .map_or(RoutingFallback::Members, |routing| routing.fallback)
                .as_str(),
        )
        .bind(rule.created_by.as_ref().map(UserId::as_str))
        .bind(mapper::naive(rule.created_at))
        .bind(mapper::naive(rule.updated_at))
        .execute(&mut **tx)
        .await?;
        sqlx::query(
            "INSERT INTO lead_distribution_states (rule_id, sequence) \
             VALUES (?, 0)",
        )
        .bind(rule.id.to_string())
        .execute(&mut **tx)
        .await?;
        self.write_members(tx, rule).await
    }

    pub(crate) async fn update(
        &self,
        tx: &mut MySqlTx,
        rule: &DistributionRule,
    ) -> Result<(), AppError> {
        let result = sqlx::query(
            "UPDATE lead_distribution_rules SET `key` = ?, name = ?, description = ?, \
             record_type = ?, strategy = ?, is_active = ?, routing_field = ?, routes = ?, \
             routing_fallback = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&rule.key)
        .bind(&rule.name)
        .bind(&rule.description)
        .bind(&rule.record_type)
        .bind(rule.strategy.as_str())
        .bind(rule.is_active)
        .bind(rule.routing.as_ref().map(|routing| routing.field.clone()))
        .bind(routes_json(rule.routing.as_ref()))
        .bind(
            rule.routing
                .as_ref()
                .map_or(RoutingFallback::Members, |routing| routing.fallback)
                .as_str(),
        )
        .bind(mapper::naive(rule.updated_at))
        .bind(rule.id.to_string())
        .execute(&mut **tx)
        .await?;
        if result.rows_affected() == 0 {
            return Ok(());
        }
        self.write_members(tx, rule).await
    }

    pub(crate) async fn delete(&self, tx: &mut MySqlTx, rule_id: Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM lead_distribution_rules WHERE id = ?")
            .bind(rule_id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    pub async fn lock_state(
        &self,
        tx: &mut MySqlTx,
        rule_id: Uuid,
    ) -> Result<Option<u64>, AppError> {
        let sequence: Option<u64> = sqlx::query_scalar(
            "SELECT sequence FROM lead_distribution_states \
             WHERE rule_id = ? FOR UPDATE",
        )
        .bind(rule_id.to_string())
        .fetch_optional(&mut **tx)
        .await?;
        Ok(sequence)
    }

    pub async fn locked_counters(
        &self,
        tx: &mut MySqlTx,
        rule_id: Uuid,
        users: &[UserId],
    ) -> Result<Vec<Counter>, AppError> {
        if users.is_empty() {
            return Ok(Vec::new());
        }
        let rows = counters_query(rule_id, users, true)
            .build()
            .fetch_all(&mut **tx)
            .await?;
        rows.iter().map(counter_from_row).collect()
    }

    pub(crate) async fn counters(
        &self,
        rule_id: Uuid,
        users: &[UserId],
    ) -> Result<Vec<Counter>, AppError> {
        if users.is_empty() {
            return Ok(Vec::new());
        }
        let rows = counters_query(rule_id, users, false)
            .build()
            .fetch_all(&self.pool)
            .await?;
        rows.iter().map(counter_from_row).collect()
    }

    pub(crate) async fn find_assignment(
        &self,
        tx: &mut MySqlTx,
        idempotency_key: &str,
    ) -> Result<Option<UserId>, AppError> {
        let user: Option<Option<String>> = sqlx::query_scalar(
            "SELECT user_id FROM lead_distribution_assignments \
             WHERE idempotency_key = ? FOR UPDATE",
        )
        .bind(idempotency_key)
        .fetch_optional(&mut **tx)
        .await?;
        user.flatten()
            .map(|id| mapper::typed_id(id, UserId::new))
            .transpose()
    }

    pub async fn record_pick(&self, tx: &mut MySqlTx, pick: &PickRecord) -> Result<(), AppError> {
        let rule_id = pick.rule_id.to_string();
        let at = mapper::naive(pick.at);
        sqlx::query(
            "UPDATE lead_distribution_states SET sequence = ?, last_user_id = ?, \
             last_assigned_at = ? WHERE rule_id = ?",
        )
        .bind(pick.sequence)
        .bind(pick.user_id.as_str())
        .bind(at)
        .bind(&rule_id)
        .execute(&mut **tx)
        .await?;
        sqlx::query(
            "INSERT INTO lead_distribution_counters \
             (rule_id, user_id, assigned, last_sequence, credit, last_assigned_at) \
             VALUES (?, ?, 1, ?, 0, ?) \
             ON DUPLICATE KEY UPDATE assigned = assigned + 1, \
             last_sequence = VALUES(last_sequence), last_assigned_at = VALUES(last_assigned_at)",
        )
        .bind(&rule_id)
        .bind(pick.user_id.as_str())
        .bind(pick.sequence)
        .bind(at)
        .execute(&mut **tx)
        .await?;
        if !pick.credits.is_empty() {
            let mut builder = QueryBuilder::<MySql>::new(
                "INSERT INTO lead_distribution_counters (rule_id, user_id, credit) ",
            );
            builder.push_values(&pick.credits, |mut row, (user_id, credit)| {
                row.push_bind(rule_id.clone())
                    .push_bind(user_id.as_str().to_owned())
                    .push_bind(*credit);
            });
            builder.push(" ON DUPLICATE KEY UPDATE credit = VALUES(credit)");
            builder.build().execute(&mut **tx).await?;
        }
        sqlx::query(
            "INSERT INTO lead_distribution_assignments (id, rule_id, user_id, \
             record_type, record_id, source, idempotency_key, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(Uuid::now_v7().to_string())
        .bind(&rule_id)
        .bind(pick.user_id.as_str())
        .bind(
            pick.record
                .as_ref()
                .map(|record| record.record_type.as_str().to_owned()),
        )
        .bind(
            pick.record
                .as_ref()
                .map(|record| record.record_id.to_string()),
        )
        .bind(pick.source.as_str())
        .bind(&pick.idempotency_key)
        .bind(at)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn user_statuses(
        &self,
        users: &[UserId],
    ) -> Result<Vec<UserStatus>, AppError> {
        if users.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT u.id, u.display_name, (u.status = 'active') AS active \
             FROM users u WHERE u.id",
        );
        push_list(&mut builder, users);
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter()
            .map(|row| {
                let active: i64 = row.try_get("active")?;
                Ok(UserStatus {
                    user_id: mapper::typed_id(row.try_get("id")?, UserId::new)?,
                    name: row.try_get("display_name")?,
                    active: active != 0,
                })
            })
            .collect()
    }

    pub(crate) async fn team_members(&self, team_id: &TeamId) -> Result<Vec<UserId>, AppError> {
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT tm.user_id FROM team_memberships tm JOIN teams t ON t.id = tm.team_id \
             JOIN users u ON u.id = tm.user_id \
             WHERE tm.team_id = ? ORDER BY u.display_name, u.id",
        )
        .bind(team_id.as_str())
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|id| mapper::typed_id(id, UserId::new))
            .collect()
    }

    pub(crate) async fn existing_teams(
        &self,
        team_ids: &[TeamId],
    ) -> Result<Vec<TeamId>, AppError> {
        if team_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = QueryBuilder::<MySql>::new("SELECT id FROM teams WHERE ");
        builder.push("id");
        push_list(&mut builder, team_ids);
        let rows: Vec<String> = builder.build_query_scalar().fetch_all(&self.pool).await?;
        rows.into_iter()
            .map(|id| mapper::typed_id(id, TeamId::new))
            .collect()
    }

    pub async fn open_deals(&self, users: &[UserId]) -> Result<HashMap<UserId, u64>, AppError> {
        if users.is_empty() {
            return Ok(HashMap::new());
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT owner_id, COUNT(*) AS total FROM deals \
             WHERE status = 'open' AND deleted_at IS NULL AND archived_at IS NULL AND owner_id",
        );
        push_list(&mut builder, users);
        builder.push(" GROUP BY owner_id");
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter()
            .map(|row| {
                let total: i64 = row.try_get("total")?;
                Ok((
                    mapper::typed_id(row.try_get("owner_id")?, UserId::new)?,
                    u64::try_from(total).unwrap_or_default(),
                ))
            })
            .collect()
    }

    pub(crate) async fn all(&self) -> Result<Vec<DistributionRule>, AppError> {
        let mut builder = QueryBuilder::<MySql>::new(RULE_SELECT);
        builder.push(" ORDER BY r.created_at, r.id");
        let rows = builder.build().fetch_all(&self.pool).await?;
        let rules = rows
            .iter()
            .map(rule_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        self.with_members(rules).await
    }

    pub(crate) async fn users_directory(&self) -> Result<Vec<DirectoryUser>, AppError> {
        let rows = sqlx::query("SELECT id, email FROM users ORDER BY email")
            .fetch_all(&self.pool)
            .await?;
        rows.iter()
            .map(|row| {
                Ok(DirectoryUser {
                    user_id: mapper::typed_id(row.try_get("id")?, UserId::new)?,
                    email: row.try_get("email")?,
                })
            })
            .collect()
    }

    pub(crate) async fn teams_directory(&self) -> Result<Vec<DirectoryTeam>, AppError> {
        let rows = sqlx::query("SELECT id, name FROM teams ORDER BY created_at, id")
            .fetch_all(&self.pool)
            .await?;
        rows.iter()
            .map(|row| {
                Ok(DirectoryTeam {
                    team_id: mapper::typed_id(row.try_get("id")?, TeamId::new)?,
                    name: row.try_get("name")?,
                })
            })
            .collect()
    }

    pub(crate) async fn assignments(
        &self,
        filter: &AssignmentFilter,
    ) -> Result<Listing<AssignmentEntry>, AppError> {
        let push_filter = |builder: &mut QueryBuilder<MySql>| {
            if let Some(rule_id) = filter.rule_id {
                builder
                    .push(" AND a.rule_id = ")
                    .push_bind(rule_id.to_string());
            }
            if let Some(user_id) = &filter.user_id {
                builder
                    .push(" AND a.user_id = ")
                    .push_bind(user_id.as_str().to_owned());
            }
            if let Some(record_type) = &filter.record_type {
                builder
                    .push(" AND a.record_type = ")
                    .push_bind(record_type.clone());
            }
            if let Some(source) = &filter.source {
                builder.push(" AND a.source = ").push_bind(source.as_str());
            }
        };
        let mut count = QueryBuilder::<MySql>::new(
            "SELECT COUNT(*) FROM lead_distribution_assignments a WHERE 1 = 1",
        );
        push_filter(&mut count);
        let total: i64 = count.build_query_scalar().fetch_one(&self.pool).await?;
        if total == 0 {
            return Ok(Listing {
                items: Vec::new(),
                total: 0,
            });
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT a.id, a.rule_id, r.`key` AS rule_key, r.name AS rule_name, a.user_id, \
             u.display_name AS user_name, a.record_type, a.record_id, a.source, a.created_at \
             FROM lead_distribution_assignments a \
             JOIN lead_distribution_rules r ON r.id = a.rule_id \
             LEFT JOIN users u ON u.id = a.user_id WHERE 1 = 1",
        );
        push_filter(&mut builder);
        builder
            .push(" ORDER BY a.created_at DESC, a.id DESC LIMIT ")
            .push_bind(filter.window.limit)
            .push(" OFFSET ")
            .push_bind(filter.window.offset);
        let rows = builder.build().fetch_all(&self.pool).await?;
        let items = rows
            .iter()
            .map(|row| {
                let id: String = row.try_get("id")?;
                let rule_id: String = row.try_get("rule_id")?;
                let created_at: NaiveDateTime = row.try_get("created_at")?;
                let source: String = row.try_get("source")?;
                Ok(AssignmentEntry {
                    id: Uuid::parse_str(&id).map_err(|_| mapper::invalid_column("id", &id))?,
                    rule_id: Uuid::parse_str(&rule_id)
                        .map_err(|_| mapper::invalid_column("rule_id", &rule_id))?,
                    rule_key: row.try_get("rule_key")?,
                    rule_name: row.try_get("rule_name")?,
                    user_id: row
                        .try_get::<Option<String>, _>("user_id")?
                        .map(|id| mapper::typed_id(id, UserId::new))
                        .transpose()?,
                    user_name: row.try_get("user_name")?,
                    record_type: row.try_get("record_type")?,
                    record_id: row.try_get("record_id")?,
                    source: AssignmentSource::parse(&source)
                        .ok_or_else(|| mapper::invalid_column("source", &source))?,
                    created_at: mapper::utc(created_at),
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        Ok(Listing {
            items,
            total: u64::try_from(total).unwrap_or_default(),
        })
    }
}

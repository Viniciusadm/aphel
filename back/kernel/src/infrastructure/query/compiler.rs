use chrono::{Datelike, NaiveDate};
use sqlx::{MySql, QueryBuilder};

use crate::application::fields::{EntityDefinition, RelationPath};
use crate::application::query::{
    ArchivedFilter, FieldTarget, LIKE_ESCAPE, PlanCondition, PlanNode, PlanSort, PlanValue,
    Predicate, QueryPlan, RecordGroup, RelatedNode, SearchPlan,
};
use crate::domain::UserId;
use crate::domain::access::{AccessScope, RecordScopeFilter, ShareReach};
use crate::domain::fields::{FieldType, ValueColumn};
use crate::errors::AppError;
use crate::infrastructure::query::computed::computed_expression;
use crate::infrastructure::repositories::mapper;

pub fn quoted(identifier: &str) -> String {
    format!("`{}`", identifier.replace('`', ""))
}

pub(crate) const RECORD_ALIAS: &str = "r";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReferenceSetStorage {
    pub links_table: &'static str,
    pub value_column: &'static str,
    pub labels_table: &'static str,
}

pub(crate) fn reference_set_storage() -> Option<ReferenceSetStorage> {
    crate::modules::try_app()?
        .contributions::<ReferenceSetStorage>()
        .first()
        .map(|storage| **storage)
}

pub(crate) fn reference_set_source() -> String {
    match reference_set_storage() {
        Some(storage) => format!(
            "(SELECT record_type, record_id, {} AS value_id FROM {})",
            quoted(storage.value_column),
            quoted(storage.links_table)
        ),
        None => "(SELECT CAST(NULL AS CHAR) AS record_type, CAST(NULL AS CHAR) AS record_id, \
                 CAST(NULL AS CHAR) AS value_id)"
            .to_owned(),
    }
}

pub fn aliased(alias: &str, column: &str) -> String {
    format!("{alias}.{}", quoted(column))
}

fn record_column(column: &str) -> String {
    aliased(RECORD_ALIAS, column)
}

pub fn push_record_scope(
    builder: &mut QueryBuilder<MySql>,
    scope: &AccessScope,
    definition: &EntityDefinition,
    visibility: &RecordScopeFilter,
) {
    push_record_scope_as(builder, scope, definition, visibility, RECORD_ALIAS);
}

pub fn push_record_scope_as(
    builder: &mut QueryBuilder<MySql>,
    scope: &AccessScope,
    definition: &EntityDefinition,
    visibility: &RecordScopeFilter,
    alias: &str,
) {
    push_record_scope_keyed(builder, scope, definition, visibility, alias, "id");
}

pub fn push_record_scope_keyed(
    builder: &mut QueryBuilder<MySql>,
    scope: &AccessScope,
    definition: &EntityDefinition,
    visibility: &RecordScopeFilter,
    alias: &str,
    record_key: &str,
) {
    push_partition(builder, definition, alias);
    if definition.capabilities.has_trait("records.global") {
        return;
    }
    let (user_id, team_ids, shares) = match visibility {
        RecordScopeFilter::Organization => return,
        RecordScopeFilter::Owner { user_id, shares } => (user_id, &[][..], *shares),
        RecordScopeFilter::Teams {
            user_id,
            team_ids,
            shares,
        } => (user_id, team_ids.as_slice(), *shares),
    };
    let owner = definition
        .owner_column
        .as_deref()
        .map(|column| aliased(alias, column));
    builder.push(" AND (");
    match owner {
        None => {
            builder.push("1 = 0");
        }
        Some(owner) => {
            builder
                .push(format!("{owner} = "))
                .push_bind(user_id.as_str().to_owned());
            if visibility.include_memberships() {
                push_member_access(builder, definition, alias, user_id);
            }
            if !team_ids.is_empty() {
                builder.push(format!(
                    " OR {owner} IN (SELECT tm.user_id FROM team_memberships tm \
                     WHERE tm.team_id IN ("
                ));
                let mut teams = builder.separated(", ");
                for team in team_ids {
                    teams.push_bind(team.as_str().to_owned());
                }
                builder.push("))");
            }
        }
    }
    push_shared_access(
        builder,
        scope,
        definition,
        &aliased(alias, record_key),
        user_id,
        shares,
    );
    builder.push(")");
}

fn push_shared_access(
    builder: &mut QueryBuilder<MySql>,
    scope: &AccessScope,
    definition: &EntityDefinition,
    record_key: &str,
    user_id: &UserId,
    shares: ShareReach,
) {
    if shares == ShareReach::None {
        return;
    }
    builder
        .push(format!(
            " OR {record_key} IN (SELECT rs.record_id FROM record_shares rs WHERE "
        ))
        .push("rs.record_type = ")
        .push_bind(definition.record_type.as_str().to_owned())
        .push(" AND ((rs.grantee_type = 'user' AND rs.grantee_id = ")
        .push_bind(user_id.as_str().to_owned())
        .push(")");
    if !scope.team_ids().is_empty() {
        builder.push(" OR (rs.grantee_type = 'team' AND rs.grantee_id IN (");
        let mut teams = builder.separated(", ");
        for team in scope.team_ids() {
            teams.push_bind(team.as_str().to_owned());
        }
        builder.push("))");
    }
    builder.push(")");
    if shares == ShareReach::Edit {
        builder.push(" AND rs.access = 'edit'");
    }
    builder.push(")");
}

fn push_member_access(
    builder: &mut QueryBuilder<MySql>,
    definition: &EntityDefinition,
    alias: &str,
    user_id: &UserId,
) {
    let Some(access) = &definition.member_access else {
        return;
    };
    builder
        .push(format!(
            " OR {} IN (SELECT ma.{} FROM {} ma WHERE ma.{} = ",
            aliased(alias, &access.record_key),
            quoted(&access.record_column),
            quoted(&access.table),
            quoted(&access.user_column),
        ))
        .push_bind(user_id.as_str().to_owned())
        .push(")");
}

pub fn push_partition(
    builder: &mut QueryBuilder<MySql>,
    definition: &EntityDefinition,
    alias: &str,
) {
    if let Some(partition) = &definition.partition {
        builder
            .push(format!(" AND {} = ", aliased(alias, &partition.column)))
            .push_bind(partition.value.to_string());
    }
}

fn push_value(builder: &mut QueryBuilder<MySql>, value: &PlanValue) {
    match value {
        PlanValue::Text(text) => builder.push_bind(text.clone()),
        PlanValue::Number(number) => builder.push_bind(*number),
        PlanValue::Date(date) => builder.push_bind(*date),
        PlanValue::DateTime(instant) => builder.push_bind(mapper::naive(*instant)),
        PlanValue::Bool(value) => builder.push_bind(*value),
        PlanValue::OptionId(id) => builder.push_bind(id.to_string()),
    };
}

fn push_anniversary(
    builder: &mut QueryBuilder<MySql>,
    column: &str,
    from: NaiveDate,
    until: NaiveDate,
    offset_seconds: i32,
) {
    let day = if offset_seconds == 0 {
        format!("DATE({column})")
    } else {
        format!("DATE(DATE_ADD({column}, INTERVAL {offset_seconds} SECOND))")
    };
    let this_year = |builder: &mut QueryBuilder<MySql>, extra: i32| {
        builder
            .push(format!("DATE_ADD({day}, INTERVAL ("))
            .push_bind(from.year() + extra)
            .push(format!(" - YEAR({day})) YEAR)"));
    };
    builder.push(format!("({column} IS NOT NULL AND (CASE WHEN "));
    this_year(builder, 0);
    builder.push(" >= ").push_bind(from).push(" THEN ");
    this_year(builder, 0);
    builder.push(" ELSE ");
    this_year(builder, 1);
    builder.push(" END) < ").push_bind(until).push(")");
}

fn push_predicate(
    builder: &mut QueryBuilder<MySql>,
    column: &str,
    predicate: &Predicate,
    field_type: FieldType,
) {
    if field_type.base_type() == FieldType::References {
        match predicate {
            Predicate::In(values) => {
                builder.push(format!("JSON_OVERLAPS({column}, JSON_ARRAY("));
                let mut list = builder.separated(", ");
                for value in values {
                    match value {
                        PlanValue::Text(text) => list.push_bind(text.clone()),
                        _ => list.push("NULL"),
                    };
                }
                builder.push("))");
            }
            Predicate::Present => {
                builder.push(format!(
                    "({column} IS NOT NULL AND JSON_LENGTH({column}) > 0)"
                ));
            }
            _ => {
                builder.push("FALSE");
            }
        }
        return;
    }
    if field_type.base_type() == FieldType::MultiSelect {
        match predicate {
            Predicate::In(values) => {
                builder.push("(FALSE");
                for value in values {
                    builder.push(format!(" OR JSON_CONTAINS({column}, JSON_ARRAY("));
                    push_value(builder, value);
                    builder.push("))");
                }
                builder.push(")");
            }
            Predicate::Present => {
                builder.push(format!(
                    "({column} IS NOT NULL AND JSON_LENGTH({column}) > 0)"
                ));
            }
            _ => {
                builder.push("FALSE");
            }
        }
        return;
    }
    match predicate {
        Predicate::Compare(op, value) => {
            builder.push(format!("{column} {} ", op.sql()));
            push_value(builder, value);
        }
        Predicate::Between(low, high) => {
            builder.push(format!("{column} BETWEEN "));
            push_value(builder, low);
            builder.push(" AND ");
            push_value(builder, high);
        }
        Predicate::Range { from, until } => {
            builder.push("(1 = 1");
            if let Some(from) = from {
                builder.push(format!(" AND {column} >= "));
                push_value(builder, from);
            }
            if let Some(until) = until {
                builder.push(format!(" AND {column} < "));
                push_value(builder, until);
            }
            builder.push(")");
        }
        Predicate::In(values) => {
            builder.push(format!("{column} IN ("));
            let mut list = builder.separated(", ");
            for value in values {
                match value {
                    PlanValue::Text(text) => list.push_bind(text.clone()),
                    PlanValue::Number(number) => list.push_bind(*number),
                    PlanValue::Date(date) => list.push_bind(*date),
                    PlanValue::DateTime(instant) => list.push_bind(mapper::naive(*instant)),
                    PlanValue::Bool(value) => list.push_bind(*value),
                    PlanValue::OptionId(id) => list.push_bind(id.to_string()),
                };
            }
            builder.push(")");
        }
        Predicate::Like(pattern) => {
            builder
                .push(format!("{column} LIKE "))
                .push_bind(pattern.clone())
                .push(format!(" ESCAPE '{LIKE_ESCAPE}'"));
        }
        Predicate::InGroup(_) => {
            builder.push("1 = 0");
        }
        Predicate::Anniversary {
            from,
            until,
            offset_seconds,
        } => push_anniversary(builder, column, *from, *until, *offset_seconds),
        Predicate::Present => {
            if field_type.is_textual()
                || matches!(
                    field_type,
                    FieldType::Select | FieldType::User | FieldType::Record
                )
            {
                builder.push(format!("({column} IS NOT NULL AND {column} <> '')"));
            } else {
                builder.push(format!("{column} IS NOT NULL"));
            }
        }
    }
}

pub const MAX_GROUP_RECURSION: u32 = 64;

pub(crate) fn push_record_group(builder: &mut QueryBuilder<MySql>, group: &RecordGroup) {
    let table = &group.table;
    let parent = &group.parent_column;
    let live = group
        .deleted_column
        .as_ref()
        .map(|column| format!(" AND gc.{column} IS NULL"))
        .unwrap_or_default();
    builder
        .push(format!(
            "WITH RECURSIVE record_group (id, depth) AS (SELECT gc.id, 1 FROM {table} gc \
             WHERE gc.id = "
        ))
        .push_bind(group.root.to_string())
        .push(format!(
            "{live} UNION ALL SELECT gc.id, g.depth + 1 FROM {table} gc \
             JOIN record_group g ON gc.{parent} = g.id \
             WHERE g.depth < {MAX_GROUP_RECURSION}{live}) SELECT id FROM record_group"
        ));
}

pub fn push_live_record(
    builder: &mut QueryBuilder<MySql>,
    scope: &AccessScope,
    definition: &EntityDefinition,
    visibility: &RecordScopeFilter,
    alias: &str,
) {
    builder.push("TRUE");
    if let Some(column) = &definition.deleted_column {
        builder.push(format!(" AND {} IS NULL", aliased(alias, column)));
    }
    push_record_scope_as(builder, scope, definition, visibility, alias);
}

pub(crate) struct Compiler<'a> {
    pub(crate) scope: &'a AccessScope,
    definition: &'a EntityDefinition,
    joined_extensions: bool,
    pub(crate) record_type: &'a str,
    pub(crate) owner: Option<&'a str>,
    pub(crate) alias: String,
    pub(crate) depth: usize,
}

impl<'a> Compiler<'a> {
    pub(crate) fn new(scope: &'a AccessScope, definition: &'a EntityDefinition) -> Self {
        Self::aliased(scope, definition, RECORD_ALIAS, 0)
    }

    pub fn aliased(
        scope: &'a AccessScope,
        definition: &'a EntityDefinition,
        alias: &str,
        depth: usize,
    ) -> Self {
        Self {
            scope,
            definition,
            joined_extensions: false,
            record_type: definition.record_type.as_str(),
            owner: definition.owner_column.as_deref(),
            alias: alias.to_owned(),
            depth,
        }
    }

    pub(crate) fn column(&self, column: &str) -> String {
        if let Some(field) = self.definition.system_fields.iter().find(|field| {
            field.column == column && field.extension.is_some() && field.is_available()
        }) {
            if self.joined_extensions {
                return aliased(
                    &field.extension.as_ref().expect("extension").alias(),
                    column,
                );
            }
            return crate::infrastructure::query::computed::system_expression(field, &self.alias);
        }
        aliased(&self.alias, column)
    }

    fn target_expression(&self, target: &FieldTarget) -> String {
        match target {
            FieldTarget::Computed { computed } => computed_expression(computed, &self.alias),
            FieldTarget::System { column } => self.column(column),
            _ => "NULL".to_owned(),
        }
    }

    pub(crate) fn push_value_owner(&self, builder: &mut QueryBuilder<MySql>, alias: &str) {
        self.push_value_owner_of(builder, alias, self.record_type, &self.alias);
    }

    pub(crate) fn push_value_owner_of(
        &self,
        builder: &mut QueryBuilder<MySql>,
        alias: &str,
        record_type: &str,
        record_alias: &str,
    ) {
        builder
            .push(format!("{alias}.record_type = "))
            .push_bind(record_type.to_owned())
            .push(format!(" AND {alias}.record_id = {record_alias}.id"));
    }

    pub(crate) fn push_owner_teams(
        &self,
        builder: &mut QueryBuilder<MySql>,
        owner: &str,
        alias: &str,
    ) {
        builder.push(format!(
            "SELECT 1 FROM team_memberships {alias} WHERE {alias}.user_id = {owner}"
        ));
    }

    fn push_scoped_predicate(
        &self,
        builder: &mut QueryBuilder<MySql>,
        column: &str,
        predicate: &Predicate,
        field_type: FieldType,
    ) {
        if let Predicate::InGroup(group) = predicate {
            builder.push(format!("{column} IN ("));
            push_record_group(builder, group);
            builder.push(")");
        } else {
            push_predicate(builder, column, predicate, field_type);
        }
    }

    fn push_node(
        &self,
        builder: &mut QueryBuilder<MySql>,
        node: &PlanNode,
    ) -> Result<(), AppError> {
        match node {
            PlanNode::And(nodes) | PlanNode::Or(nodes) => {
                let joiner = if matches!(node, PlanNode::And(_)) {
                    " AND "
                } else {
                    " OR "
                };
                builder.push("(");
                for (index, child) in nodes.iter().enumerate() {
                    if index > 0 {
                        builder.push(joiner);
                    }
                    self.push_node(builder, child)?;
                }
                builder.push(")");
                Ok(())
            }
            PlanNode::Condition(condition) => self.push_condition(builder, condition),
            PlanNode::Related(related) => self.push_related(builder, related),
        }
    }

    fn push_related(
        &self,
        builder: &mut QueryBuilder<MySql>,
        related: &RelatedNode,
    ) -> Result<(), AppError> {
        let target = &related.target;
        let alias = format!("rel{}", self.depth);
        let link = format!("lnk{}", self.depth);
        let table = quoted(&target.table);
        let source = &self.alias;
        let nested = Compiler::aliased(self.scope, target, &alias, self.depth + 1);
        builder.push(if related.negated { "NOT (" } else { "(" });
        for (index, path) in related.relation.paths.iter().enumerate() {
            if index > 0 {
                builder.push(" OR ");
            }
            builder.push("EXISTS (SELECT 1 FROM ");
            match path {
                RelationPath::BelongsTo { column } => {
                    builder.push(format!(
                        "{table} {alias} WHERE {alias}.id = {} AND ",
                        aliased(source, column)
                    ));
                }
                RelationPath::HasMany { column } => {
                    builder.push(format!(
                        "{table} {alias} WHERE {} = {source}.id AND ",
                        aliased(&alias, column)
                    ));
                }
                RelationPath::Polymorphic {
                    type_column,
                    id_column,
                } => {
                    builder
                        .push(format!(
                            "{table} {alias} WHERE {alias}.id = {} AND {} = ",
                            aliased(source, id_column),
                            aliased(source, type_column)
                        ))
                        .push_bind(target.record_type.as_str().to_owned())
                        .push(" AND ");
                }
                RelationPath::PolymorphicMany {
                    type_column,
                    id_column,
                } => {
                    builder
                        .push(format!(
                            "{table} {alias} WHERE {} = {source}.id AND {} = ",
                            aliased(&alias, id_column),
                            aliased(&alias, type_column)
                        ))
                        .push_bind(self.record_type.to_owned())
                        .push(" AND ");
                }
                RelationPath::Linked {
                    table: link_table,
                    source_type_column,
                    source_id_column,
                    target_type_column,
                    target_id_column,
                } => {
                    builder.push(format!(
                        "{} {link} JOIN {table} {alias} ON {alias}.id = {} WHERE {} = {source}.id",
                        quoted(link_table),
                        aliased(&link, target_id_column),
                        aliased(&link, source_id_column)
                    ));
                    if let Some(column) = source_type_column {
                        builder
                            .push(format!(" AND {} = ", aliased(&link, column)))
                            .push_bind(self.record_type.to_owned());
                    }
                    if let Some(column) = target_type_column {
                        builder
                            .push(format!(" AND {} = ", aliased(&link, column)))
                            .push_bind(target.record_type.as_str().to_owned());
                    }
                    builder.push(" AND ");
                }
                RelationPath::Relationship {
                    relationship_id,
                    inverse,
                    ..
                } => {
                    let (source_column, target_column) = RelationPath::link_columns(*inverse);
                    builder
                        .push(format!(
                            "record_relations {link} JOIN {table} {alias} ON {alias}.id = {link}.{target_column} \
                             WHERE {link}.relationship_id = "
                        ))
                        .push_bind(relationship_id.to_string())
                        .push(format!(" AND {link}.{source_column} = {source}.id AND "));
                }
            }
            push_live_record(builder, self.scope, target, &related.visibility, &alias);
            builder.push(" AND ");
            nested.push_node(builder, &related.node)?;
            builder.push(")");
        }
        builder.push(")");
        Ok(())
    }

    fn push_condition(
        &self,
        builder: &mut QueryBuilder<MySql>,
        condition: &PlanCondition,
    ) -> Result<(), AppError> {
        match &condition.target {
            FieldTarget::OwnerTeam => {
                if condition.negated {
                    builder.push("NOT ");
                }
                builder.push("EXISTS (");
                let owner = self.owner_column()?;
                self.push_owner_teams(builder, &owner, "otm");
                if let Predicate::In(_) = &condition.predicate {
                    builder.push(" AND ");
                    push_predicate(
                        builder,
                        "otm.team_id",
                        &condition.predicate,
                        condition.field_type,
                    );
                }
                builder.push(")");
            }
            FieldTarget::System { .. } | FieldTarget::Computed { .. } => {
                let column = self.target_expression(&condition.target);
                if condition.negated {
                    builder.push(format!("({column} IS NULL OR NOT ("));
                    self.push_scoped_predicate(
                        builder,
                        &column,
                        &condition.predicate,
                        condition.field_type,
                    );
                    builder.push("))");
                } else {
                    builder.push("(");
                    self.push_scoped_predicate(
                        builder,
                        &column,
                        &condition.predicate,
                        condition.field_type,
                    );
                    builder.push(")");
                }
            }
            FieldTarget::ReferenceSet => {
                if condition.negated {
                    builder.push("NOT ");
                }
                builder.push(format!(
                    "EXISTS (SELECT 1 FROM {} tg WHERE ",
                    reference_set_source()
                ));
                self.push_value_owner(builder, "tg");
                if let Predicate::In(_) = &condition.predicate {
                    builder.push(" AND ");
                    push_predicate(
                        builder,
                        "tg.value_id",
                        &condition.predicate,
                        condition.field_type,
                    );
                }
                builder.push(")");
            }
            FieldTarget::Custom { field_id, column } => {
                if condition.negated {
                    builder.push("NOT ");
                }
                if *column == ValueColumn::MultiOption {
                    builder.push("EXISTS (SELECT 1 FROM custom_field_multi_values m WHERE ");
                    self.push_value_owner(builder, "m");
                    builder
                        .push(" AND m.field_id = ")
                        .push_bind(field_id.to_string());
                    match &condition.predicate {
                        Predicate::Present => {}
                        Predicate::In(_) => {
                            builder.push(" AND ");
                            push_predicate(
                                builder,
                                "m.option_id",
                                &condition.predicate,
                                FieldType::Select,
                            );
                        }
                        other => {
                            return Err(AppError::Internal(format!(
                                "unsupported multi_select predicate {other:?}"
                            )));
                        }
                    }
                } else {
                    builder.push("EXISTS (SELECT 1 FROM custom_field_values v WHERE ");
                    self.push_value_owner(builder, "v");
                    builder
                        .push(" AND v.field_id = ")
                        .push_bind(field_id.to_string());
                    if condition.predicate != Predicate::Present {
                        builder.push(" AND ");
                        self.push_scoped_predicate(
                            builder,
                            &format!("v.{}", column.column_name()),
                            &condition.predicate,
                            condition.field_type,
                        );
                    }
                }
                builder.push(")");
            }
        }
        Ok(())
    }

    pub(crate) fn owner_column(&self) -> Result<String, AppError> {
        self.owner.map(|owner| self.column(owner)).ok_or_else(|| {
            AppError::Internal(format!(
                "record type {} has no owner column",
                self.record_type
            ))
        })
    }

    fn push_search(&self, builder: &mut QueryBuilder<MySql>, search: &SearchPlan) {
        builder.push(" AND (1 = 0");
        for column in &search.system_columns {
            builder
                .push(format!(" OR {} LIKE ", self.column(column)))
                .push_bind(search.pattern.clone())
                .push(format!(" ESCAPE '{LIKE_ESCAPE}'"));
        }
        if !search.custom_field_ids.is_empty() {
            builder
                .push(format!(
                    " OR {}.id IN (SELECT sv.record_id FROM custom_field_values sv WHERE sv.record_type = ",
                    self.alias
                ))
                .push_bind(self.record_type.to_owned())
                .push(" AND sv.field_id IN (");
            let mut ids = builder.separated(", ");
            for id in &search.custom_field_ids {
                ids.push_bind(id.to_string());
            }
            builder
                .push(") AND (sv.value_text LIKE ")
                .push_bind(search.pattern.clone())
                .push(format!(
                    " ESCAPE '{LIKE_ESCAPE}' OR sv.value_long_text LIKE "
                ))
                .push_bind(search.pattern.clone())
                .push(format!(" ESCAPE '{LIKE_ESCAPE}'))"));
        }
        builder.push(")");
    }

    fn push_sort_joins(&self, builder: &mut QueryBuilder<MySql>, sorts: &[PlanSort]) {
        for (index, sort) in sorts.iter().enumerate() {
            if let FieldTarget::Custom { field_id, .. } = &sort.target {
                builder.push(format!(" LEFT JOIN custom_field_values s{index} ON "));
                self.push_value_owner(builder, &format!("s{index}"));
                builder
                    .push(format!(" AND s{index}.field_id = "))
                    .push_bind(field_id.to_string());
                if sort.field_type.base_type() == FieldType::Select {
                    builder.push(format!(
                        " LEFT JOIN field_options so{index} ON so{index}.id = s{index}.value_option_id"
                    ));
                }
            }
        }
    }

    fn push_order(&self, builder: &mut QueryBuilder<MySql>, sorts: &[PlanSort]) {
        builder.push(" ORDER BY ");
        for (index, sort) in sorts.iter().enumerate() {
            let expression = match &sort.target {
                FieldTarget::ReferenceSet | FieldTarget::OwnerTeam => continue,
                FieldTarget::System { .. } | FieldTarget::Computed { .. } => {
                    self.target_expression(&sort.target)
                }
                FieldTarget::Custom { .. } if sort.field_type.base_type() == FieldType::Select => {
                    format!("so{index}.position")
                }
                FieldTarget::Custom { column, .. } => format!("s{index}.{}", column.column_name()),
            };
            let direction = if sort.descending { "DESC" } else { "ASC" };
            let ordered =
                matches!(sort.target, FieldTarget::System { .. }) && !sort.option_order.is_empty();
            if ordered {
                builder.push(format!("({expression} IS NULL) ASC, FIELD({expression}"));
                for value in &sort.option_order {
                    builder.push(", ");
                    builder.push_bind(value.clone());
                }
                builder.push(format!(") {direction}, "));
            } else {
                builder.push(format!(
                    "({expression} IS NULL) ASC, {expression} {direction}, "
                ));
            }
        }
        builder.push("r.id DESC");
    }
}

pub enum Projection {
    Count,
    Ids,
}

pub fn push_where(
    builder: &mut QueryBuilder<MySql>,
    scope: &AccessScope,
    plan: &QueryPlan,
) -> Result<(), AppError> {
    push_where_with(
        builder,
        scope,
        plan,
        &Compiler::new(scope, &plan.record_type),
    )
}

fn push_where_with(
    builder: &mut QueryBuilder<MySql>,
    scope: &AccessScope,
    plan: &QueryPlan,
    compiler: &Compiler<'_>,
) -> Result<(), AppError> {
    let definition = &plan.record_type;
    builder.push(" WHERE TRUE");
    if let Some(column) = &definition.deleted_column {
        builder.push(format!(" AND {} IS NULL", record_column(column)));
    }
    if let Some(column) = &definition.archived_column {
        match plan.archived {
            ArchivedFilter::Exclude => {
                builder.push(format!(" AND {} IS NULL", record_column(column)));
            }
            ArchivedFilter::Only => {
                builder.push(format!(" AND {} IS NOT NULL", record_column(column)));
            }
            ArchivedFilter::Include => {}
        }
    }
    push_record_scope(builder, scope, definition, &plan.visibility);
    if let Some(filter) = &plan.filter {
        builder.push(" AND ");
        compiler.push_node(builder, filter)?;
    }
    if let Some(search) = &plan.search {
        compiler.push_search(builder, search);
    }
    Ok(())
}

pub fn build_query(
    scope: &AccessScope,
    plan: &QueryPlan,
    projection: Projection,
) -> Result<QueryBuilder<MySql>, AppError> {
    build_query_inner(scope, plan, projection, true)
}
pub fn build_query_unwindowed(
    scope: &AccessScope,
    plan: &QueryPlan,
    projection: Projection,
) -> Result<QueryBuilder<MySql>, AppError> {
    build_query_inner(scope, plan, projection, false)
}
fn build_query_inner(
    scope: &AccessScope,
    plan: &QueryPlan,
    projection: Projection,
    windowed: bool,
) -> Result<QueryBuilder<MySql>, AppError> {
    let definition = &plan.record_type;
    let mut compiler = Compiler::new(scope, definition);
    compiler.joined_extensions = true;
    let table = quoted(&definition.table);
    let mut builder = match projection {
        Projection::Count => QueryBuilder::new(format!("SELECT COUNT(*) FROM {table} r")),
        Projection::Ids => {
            let mut builder = QueryBuilder::new(format!("SELECT r.id FROM {table} r"));
            compiler.push_sort_joins(&mut builder, &plan.sort);
            builder
        }
    };
    let mut joined = std::collections::BTreeSet::new();
    for field in definition
        .system_fields
        .iter()
        .filter(|field| field.is_available())
    {
        if let Some(extension) = &field.extension
            && joined.insert(&extension.table)
        {
            builder.push(format!(
                " LEFT JOIN `{}` {} ON {}.`{}` = r.id",
                extension.table,
                extension.alias(),
                extension.alias(),
                extension.record_column
            ));
        }
    }
    push_where_with(&mut builder, scope, plan, &compiler)?;
    if windowed && matches!(projection, Projection::Ids) {
        compiler.push_order(&mut builder, &plan.sort);
        builder
            .push(" LIMIT ")
            .push_bind(plan.window.limit)
            .push(" OFFSET ")
            .push_bind(plan.window.offset);
    }
    Ok(builder)
}

pub fn push_reference_subquery(
    builder: &mut QueryBuilder<MySql>,
    scope: &AccessScope,
    plan: &QueryPlan,
    reference_alias: &str,
    type_column: &str,
    id_column: &str,
) -> Result<(), AppError> {
    for identifier in [reference_alias, type_column, id_column] {
        if !crate::application::fields::is_sql_identifier(identifier) {
            return Err(AppError::Configuration(
                "invalid reference query identifier".to_owned(),
            ));
        }
    }
    builder
        .push(format!(" OR ({} = ", aliased(reference_alias, type_column)))
        .push_bind(plan.record_type.record_type.as_str().to_owned())
        .push(format!(
            " AND {} IN (SELECT r.id FROM {} r",
            aliased(reference_alias, id_column),
            quoted(&plan.record_type.table)
        ));
    push_where(builder, scope, plan)?;
    builder.push("))");
    Ok(())
}

pub fn push_record_ids_subquery(
    builder: &mut QueryBuilder<MySql>,
    scope: &AccessScope,
    plan: &QueryPlan,
) -> Result<(), AppError> {
    builder.push(format!(
        "SELECT r.id FROM {} r",
        quoted(&plan.record_type.table)
    ));
    push_where(builder, scope, plan)
}

use aphel_sdk::configuration::ConfigReferenceKind;
use std::collections::BTreeSet;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{Tag, normalize_tag_color, normalize_tag_name};
use crate::store::TagStore;
use crate::taggable_record_type;
use aphel_kernel::errors::AppError;
use aphel_sdk::configuration::sections::common::{check_key, diff};
use aphel_sdk::configuration::{
    ApplyContext, ConfigKeyRepository, ConfigPackage, ConfigSection, Decisions, KeyedItem,
    PackageIssue, PlanChange, PlanItem, assign_keys, parse_section,
};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::access::AccessScope;

pub const TAGS_SECTION: &str = "tags";
const TAG_KIND: &str = "tag";
pub(crate) const MAX_PACKAGE_TAGS: usize = 200;
pub(crate) const TAG_NAME_TAKEN: &str = "tag_name_taken";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageTag {
    pub key: String,
    pub(crate) name: String,
    pub(crate) color: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) record_types: Vec<String>,
}

struct TagPlan {
    item: PlanItem,
    tag: Tag,
    is_new: bool,
    key: String,
}

pub(crate) struct TagsSection<K> {
    tags: TagStore,
    keys: K,
}

impl<K> TagsSection<K>
where
    K: ConfigKeyRepository,
{
    pub(crate) fn new(tags: TagStore, keys: K) -> Self {
        Self { tags, keys }
    }

    async fn keyed(&self) -> Result<Vec<(Tag, KeyedItem)>, AppError> {
        let mut tags = self.tags.list().await?;
        tags.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then(left.id.cmp(&right.id))
        });
        let stored = self
            .keys
            .keys(ConfigReferenceKind::new("tagging.tag"))
            .await?;
        let keyed = assign_keys(tags.iter().map(|tag| (tag.id, tag.name.as_str())), &stored);
        Ok(tags.into_iter().zip(keyed).collect())
    }

    async fn reconcile(
        &self,
        scope: &AccessScope,
        package: &[PackageTag],
        at: DateTime<Utc>,
    ) -> Result<Vec<TagPlan>, AppError> {
        let existing = self.keyed().await?;
        let mut plans = Vec::new();
        for entry in package {
            let name = normalize_tag_name(&entry.name).unwrap_or_else(|_| entry.name.clone());
            let color = normalize_tag_color(&entry.color).unwrap_or_else(|_| entry.color.clone());
            let record_types = normalized_record_types(&entry.record_types);
            let item = PlanItem::new(TAGS_SECTION, TAG_KIND, entry.key.clone(), name.clone());
            let name_owner = existing.iter().find(|(tag, keyed)| {
                keyed.key != entry.key && tag.name.to_lowercase() == name.to_lowercase()
            });
            match existing.iter().find(|(_, keyed)| keyed.key == entry.key) {
                None => {
                    let created = Tag {
                        id: Uuid::now_v7(),
                        name: name.clone(),
                        color,
                        record_types,
                        created_by: Some(scope.user_id().clone()),
                        created_at: at,
                        updated_at: at,
                    };
                    let changes: Vec<PlanChange> = snapshot(&created)
                        .into_iter()
                        .map(|(property, value)| PlanChange::new(property, Value::Null, value))
                        .collect();
                    let item = match name_owner {
                        Some(_) => item.conflicting(
                            changes,
                            TAG_NAME_TAKEN,
                            format!("the organization already has another tag named {name}"),
                            false,
                        ),
                        None => item.created(changes),
                    };
                    plans.push(TagPlan {
                        item,
                        tag: created,
                        is_new: true,
                        key: entry.key.clone(),
                    });
                }
                Some((current, _)) => {
                    let updated = Tag {
                        name: name.clone(),
                        color,
                        record_types,
                        updated_at: at,
                        ..current.clone()
                    };
                    let changes = diff(&snapshot(current), &snapshot(&updated));
                    let item = match name_owner {
                        Some(_) if updated.name != current.name => item.conflicting(
                            changes,
                            TAG_NAME_TAKEN,
                            format!("the organization already has another tag named {name}"),
                            false,
                        ),
                        _ => item.compared(changes),
                    };
                    plans.push(TagPlan {
                        item,
                        tag: updated,
                        is_new: false,
                        key: entry.key.clone(),
                    });
                }
            }
        }
        Ok(plans)
    }
}

fn normalized_record_types(values: &[String]) -> Vec<String> {
    let mut record_types: Vec<String> = Vec::new();
    for value in values {
        if let Ok(record_type) = taggable_record_type(value) {
            let value = record_type.as_str().to_owned();
            if !record_types.contains(&value) {
                record_types.push(value);
            }
        }
    }
    record_types
}

fn snapshot(tag: &Tag) -> Vec<(String, Value)> {
    vec![
        ("name".to_owned(), json!(tag.name)),
        ("color".to_owned(), json!(tag.color)),
        ("record_types".to_owned(), json!(tag.record_types)),
    ]
}

fn parse(content: &Value) -> Result<Vec<PackageTag>, AppError> {
    parse_section(content).map_err(|issue| AppError::Validation(issue.message))
}

#[async_trait]
impl<K> ConfigSection<MySqlTx> for TagsSection<K>
where
    K: ConfigKeyRepository<Tx = MySqlTx>,
{
    fn key(&self) -> &'static str {
        TAGS_SECTION
    }

    async fn export(&self, _scope: &AccessScope) -> Result<Option<Value>, AppError> {
        let tags: Vec<PackageTag> = self
            .keyed()
            .await?
            .into_iter()
            .map(|(tag, keyed)| PackageTag {
                key: keyed.key,
                name: tag.name,
                color: tag.color,
                record_types: tag.record_types,
            })
            .collect();
        Ok(Some(json!(tags)))
    }

    async fn validate(
        &self,
        _scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PackageIssue>, AppError> {
        let tags: Vec<PackageTag> = match parse_section(content) {
            Ok(tags) => tags,
            Err(issue) => return Ok(vec![issue]),
        };
        let mut issues = Vec::new();
        if tags.len() > MAX_PACKAGE_TAGS {
            issues.push(PackageIssue::new(
                "",
                "too_many_tags",
                format!("a package can have at most {MAX_PACKAGE_TAGS} tags"),
            ));
        }
        let mut keys = BTreeSet::new();
        let mut names = BTreeSet::new();
        for (index, tag) in tags.iter().enumerate() {
            check_key(&tag.key, &format!("{index}.key"), &mut keys, &mut issues);
            match normalize_tag_name(&tag.name) {
                Err(message) => issues.push(PackageIssue::new(
                    format!("{index}.name"),
                    "invalid_name",
                    message,
                )),
                Ok(name) if !names.insert(name.to_lowercase()) => issues.push(PackageIssue::new(
                    format!("{index}.name"),
                    "duplicated_name",
                    "is declared more than once",
                )),
                Ok(_) => {}
            }
            if let Err(message) = normalize_tag_color(&tag.color) {
                issues.push(PackageIssue::new(
                    format!("{index}.color"),
                    "invalid_color",
                    message,
                ));
            }
            for (type_index, record_type) in tag.record_types.iter().enumerate() {
                if taggable_record_type(record_type).is_err() {
                    issues.push(PackageIssue::new(
                        format!("{index}.record_types.{type_index}"),
                        "invalid_record_type",
                        "must be a taggable record type",
                    ));
                }
            }
        }
        Ok(issues)
    }

    async fn plan(
        &self,
        scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PlanItem>, AppError> {
        Ok(self
            .reconcile(scope, &parse(content)?, DateTime::<Utc>::UNIX_EPOCH)
            .await?
            .into_iter()
            .map(|plan| plan.item)
            .collect())
    }

    async fn apply(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
        decisions: &Decisions,
        context: &mut ApplyContext,
    ) -> Result<(), AppError> {
        for plan in self.reconcile(scope, &parse(content)?, context.at).await? {
            if !decisions.executes(&plan.item)
                && plan.item.action != aphel_sdk::configuration::PlanAction::Unchanged
            {
                continue;
            }
            if plan.is_new {
                self.tags.insert(tx, &plan.tag).await?;
            } else if decisions.executes(&plan.item) {
                self.tags.update(tx, &plan.tag).await?;
            }
            self.keys
                .assign(
                    tx,
                    ConfigReferenceKind::new("tagging.tag"),
                    plan.tag.id,
                    &plan.key,
                )
                .await?;
            context.resolve(
                ConfigReferenceKind::new("tagging.tag"),
                &plan.key,
                plan.tag.id,
            );
        }
        Ok(())
    }
}

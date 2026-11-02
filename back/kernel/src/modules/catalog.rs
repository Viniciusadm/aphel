use crate::application::public_events::PublicEventDefinition;
use crate::domain::access::{Permission, PermissionDefinition};
use crate::modules::builder::Contribution;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenApiTag {
    pub name: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionGrant {
    pub permission: Permission,
    pub roles: Vec<&'static str>,
    pub organization_wide: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogConflict {
    pub catalog: &'static str,
    pub key: String,
    pub modules: Vec<&'static str>,
}

#[derive(Default)]
pub struct Catalog {
    pub permissions: Vec<Contribution<PermissionDefinition>>,
    pub grants: Vec<Contribution<PermissionGrant>>,
    pub public_events: Vec<Contribution<PublicEventDefinition>>,
    pub tags: Vec<Contribution<OpenApiTag>>,
    pub conflicts: Vec<CatalogConflict>,
}

fn keep_last<T>(
    catalog: &'static str,
    items: &mut Vec<Contribution<T>>,
    key: impl Fn(&T) -> String,
    conflicts: &mut Vec<CatalogConflict>,
) {
    let mut kept: Vec<Contribution<T>> = Vec::new();
    for contribution in items.drain(..) {
        let current = key(&contribution.item);
        if let Some(position) = kept
            .iter()
            .position(|existing| key(&existing.item) == current)
        {
            let previous = kept.remove(position);
            tracing::info!(
                catalog,
                key = %current,
                previous = previous.module,
                replacement = contribution.module,
                "module catalog entry replaced"
            );
            match conflicts
                .iter_mut()
                .find(|conflict| conflict.catalog == catalog && conflict.key == current)
            {
                Some(conflict) => conflict.modules.push(contribution.module),
                None => conflicts.push(CatalogConflict {
                    catalog,
                    key: current,
                    modules: vec![previous.module, contribution.module],
                }),
            }
            kept.insert(position, contribution);
        } else {
            kept.push(contribution);
        }
    }
    *items = kept;
}

impl Catalog {
    pub fn freeze(&mut self) {
        let mut conflicts = Vec::new();
        keep_last(
            "permission",
            &mut self.permissions,
            |definition| definition.permission.as_str().to_owned(),
            &mut conflicts,
        );
        keep_last(
            "public_event",
            &mut self.public_events,
            |definition| definition.event_type.to_owned(),
            &mut conflicts,
        );
        keep_last(
            "openapi_tag",
            &mut self.tags,
            |tag| tag.name.to_owned(),
            &mut conflicts,
        );
        self.conflicts = conflicts;
    }

    pub fn permissions(&self) -> impl Iterator<Item = &PermissionDefinition> {
        self.permissions
            .iter()
            .map(|contribution| &contribution.item)
    }

    pub fn permission_contributions(&self) -> &[Contribution<PermissionDefinition>] {
        &self.permissions
    }

    pub fn permission_owner(&self, permission: &Permission) -> Option<&'static str> {
        self.permissions
            .iter()
            .find(|contribution| &contribution.item.permission == permission)
            .map(|contribution| contribution.module)
    }

    pub fn grants_of<'a>(&'a self, module: &'a str) -> impl Iterator<Item = &'a PermissionGrant> {
        self.grants
            .iter()
            .filter(move |contribution| contribution.module == module)
            .map(|contribution| &contribution.item)
    }

    pub fn grant_contributions(&self) -> &[Contribution<PermissionGrant>] {
        &self.grants
    }

    pub fn public_events(&self) -> impl Iterator<Item = &PublicEventDefinition> {
        self.public_events
            .iter()
            .map(|contribution| &contribution.item)
    }

    pub fn public_event_contributions(&self) -> &[Contribution<PublicEventDefinition>] {
        &self.public_events
    }

    pub fn tags(&self) -> impl Iterator<Item = &OpenApiTag> {
        self.tags.iter().map(|contribution| &contribution.item)
    }

    pub fn tag_contributions(&self) -> &[Contribution<OpenApiTag>] {
        &self.tags
    }

    pub fn conflicts(&self) -> &[CatalogConflict] {
        &self.conflicts
    }
}

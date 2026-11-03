# P5.14 Registry-backed favorites and saved-view sources

Favorites owns only personal favorite identities, order and availability. Records resolve through the SDK's scoped record-label reader. The saved-view owner contributes its own visibility source through the SDK; favorites does not query saved-view storage or depend on its implementation.

## Static review

- [x] Record targets use registry definitions and the generic record engine, including owner availability, record scopes, title field policy and entity partitions. Missing/hidden titles use the authorized record identity as their label.
- [x] Saved views own visibility, names and target entity selection. The host rejects duplicate source registrations and unrequested target identities; an absent/disabled source leaves favorites unavailable.
- [x] Favorites has no SQL over another plugin's tables and no peer implementation dependency.
- [x] The saved-view foreign key was removed from the fresh-install schema. Favorites retain soft references, so deleted or inaccessible views are shown as unavailable rather than requiring cross-plugin cascades.
- [x] Uniqueness and lookup include the record type as well as target kind and ID, preventing collisions between distinct entity types using the same UUID.
- [x] Existing web presentation derives routes and icons from record resources; record and view actions remain scoped to their registered targets.
- [x] No migration/adoption path, legacy alias or compatibility transport was added.

## Manual acceptance pending

D4 prohibits tests, probes and runtime instances. Verification is compilation and strict lint/type checks.

- [ ] Favorite native, managed and dynamic records with own/team/organization scopes, hidden titles and missing title values.
- [ ] Favorite private/team/organization saved views; revoke access, disable/remove their owner, delete a view and inspect unavailable entries.
- [ ] Use identical UUIDs under different entity types and inspect creation, duplicate detection, list order and deletion.
- [ ] Inspect custom record links, icons, sidebar collapse, command actions and both locales.
- [ ] Inspect limits, reorder validation and concurrency with multiple tabs.

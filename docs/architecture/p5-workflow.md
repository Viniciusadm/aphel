# P5.3 Workflow review

The workflow capability owns pipelines, stages, guidance, checklist definitions and per-record checklist progress. Entity providers own their stage assignments and publish their own domain events. `aphel-contract-workflow` defines the staged field mapping, locations, usage counts and movement requests; native adapters implement the SDK's `StagedRecordSource` contribution.

## Implementation review

- [x] Canonical plugin, routes and entity identities use `workflow`.
- [x] The capability contains no deal-specific permissions, entity identifiers, queries or field allowlists.
- [x] Counts use each provider's authorization and field visibility. Integrity counts include deleted records and compiled providers whose plugin is disabled.
- [x] Stage movement delegates to each data owner in the caller's transaction. The deal adapter publishes domain events and records an audit entry.
- [x] Checklist progress is keyed by entity type, record ID and checklist item, with no foreign key to another plugin's record table. Record purging removes progress transactionally.
- [x] Stage requirements and checklist hooks use declared field mappings and support values under `custom_fields`.
- [x] Stalled-record computed fields accept a validated stage/state/time mapping. The deal-specific time trigger belongs to deals; the automation scheduler consumes contributed catch-up policies.
- [x] Generic record guidance and controls use the `workflow.staged` trait. Custom object metadata projects the same traits to the web registry.
- [x] Required-field choices come from readable staged entity definitions and scoped field sets. Requirements apply only to fields declared by the current entity; hidden known fields retain the inaccessible-field error.

## Manual acceptance pending

No runtime checks or test instances were executed, as required by the repository's no-tests decision.

- [ ] Install a fresh schema and enable workflow with an entity provider; verify isolated enablement and disabling.
- [ ] Check visible counts with restricted record scope and hidden stage fields.
- [ ] Move a record between stages with entry requirements, exit requirements and required checklist items.
- [ ] Verify checklist isolation between equal record IDs in different entity types, including a custom object with mapped custom fields.
- [ ] Remove a stage or pipeline, verifying reassignment, audit and event atomicity, including deleted records.
- [ ] Tighten a stalled-stage threshold and verify the provider's catch-up policy and normal time trigger.

Cross-plugin crate references used by existing configuration sections and consumers remain part of the final P5 dependency cleanup. They are not compatibility layers.

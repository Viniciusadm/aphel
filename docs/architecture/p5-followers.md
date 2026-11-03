# P5.15 Owner-provided followed events

Followers owns subscriptions and delivery coordination. Generic updates use the entity's declared event prefix; owners contribute related-event targets, localized headlines, source-record requirements and recipient visibility through the SDK.

## Static review

- [x] Tasks owns linked-task target discovery; notes owns private-note exclusion; email owns mailbox/thread visibility and email permission checks; deals owns stage-change signals.
- [x] Followers has no fixed list of business events or business-specific notification text branches. Localized headline metadata is persisted with the notification and rendered generically.
- [x] Source identities, ambiguous registrations, target counts and body-record requirements are validated. Targets are deduplicated; actor/already-notified suppression occurs alongside source and recipient checks.
- [x] Recipients must read the followed target and every source-required record. Source bodies are hidden by the source record's field policy; change descriptions retain per-recipient field/sensitivity masking.
- [x] Notification titles use the shared scoped record-label reader rather than raw event-name/title fallbacks. Followed-record listings apply the same title masking and owner availability.
- [x] Automatic owner following matches registry event prefixes, including native prefixes that differ from entity keys. It still requires the entity's declared owner column.
- [x] The web filter discovers entity metadata and labels from the generic catalog. Routes/icons come from registered record resources, without quote/custom route fallbacks or fixed CRM type lists.
- [x] The generic entity catalog includes localized entity labels, including administrator-defined objects.
- [x] Owners query only their own data and core identity/access data. No foreign keys, compatibility aliases, identity migration or adoption logic were introduced.

## Manual acceptance pending

D4 prohibits tests, probes and runtime instances. Verification is compilation, strict Clippy/lint, type checks, frontend build and route/catalog generation.

- [ ] Follow native, managed and dynamic records; inspect hidden/missing titles, disabled owners, access revocation, type filters and links.
- [ ] Create/update/change owners using native and custom event prefixes; inspect automatic/manual subscription behavior and retries.
- [ ] Inspect task links to several followed records, private notes, shared/metadata/private mailboxes, missing email permission and stage changes.
- [ ] Verify actor and previously-notified suppression, per-recipient source-record access, hidden body fields, sensitive change values and multiple-target deduplication.
- [ ] Inspect queued, in-app and emailed notifications in both locales, including rendering after a source is disabled.

Followers still consumes notification implementation interfaces. Their shared capability boundary, notification directory SQL and remaining notification-related implementation dependencies remain part of P5.22–P5.24.

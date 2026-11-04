# P5.22 remaining domain plugins

This phase is in progress. Completed entries describe static source validation; manual acceptance remains pending. The main P5.22 checkbox stays open until all remaining domain plugins have been reviewed.

## Booking identity

- [x] Replace the native module, Cargo package/feature, manifest, permissions, event/job namespaces and web module identity with `booking`.
- [x] Use `/booking` for authenticated screens/API and preserve the intentional public `/book` experience.
- [x] Rewrite the fresh availability schema and every consumer to use `booking_availability`; no copy migration or old identity alias.
- [x] Update assignment source metadata, profiles, generated registries, translations and API consumers.
- [ ] Remove remaining task, party, assignment and email implementation dependencies and foreign SQL.
- [ ] Manual acceptance: availability editing, meeting types, public booking, host assignment, cancellation, rescheduling and email delivery with module/permission combinations.

## Lead Pro fresh schema

- [x] Remove the legacy temperature-copy migration. The initial schema already creates the owned extension directly.
- [ ] Remove peer implementations and foreign party SQL; review behavior and manual acceptance.

## Notification extension boundary

- [x] Publish kernel-free notification kinds/channel metadata through `aphel-contract-notifications`.
- [x] Define native source, presentation, digest/follower, directory and delivery interfaces in the SDK. Owners contribute their own event policies and localized text.
- [x] Keep notification inserts, preference evaluation, deduplication, email/push queuing and delivery in `notifications`.
- [x] Replace peer implementation imports with SDK interfaces, including event/job consumers and the runtime notification adapter. Delivery resolves the enabled service at execution time; missing delivery is an explicit unavailable error.
- [x] Move generic mention parsing to SDK messaging utilities; note limits remain with notes. Generic local-day calculations belong to SDK calendar utilities; reminder schedules remain with reminders.
- [ ] Manual acceptance: source enable/disable, field/record visibility, recipient preferences, dedupe, actor suppression, localized mail, digest timezone/DST, reminders and runtime plugin notification capability checks.
- [ ] Review pending jobs when notification delivery is disabled and storage/queue failures; static validation does not establish retry or delivery acceptance.

## Configuration section boundary

- [x] Publish portable package, issue, decision and plan metadata in the kernel-free configuration contract. Header validation receives the installation's supported locales explicitly.
- [x] Define section factories, key lookup/write ports and version fragment layout contributions in the SDK.
- [x] Share native schema projection, field metadata and configuration-key helpers used by section contributors. Field apply operations and formula checks stay in the host.
- [x] Keep SQL key access, install/version repositories, installation planning/apply, template loading and version algorithms in `config_packages`.
- [x] Replace implementation dependencies for configuration section contributors with the shared boundary.
- [ ] Replace remaining direct install/version/template consumers in marketplace, demo, onboarding and organization export; onboarding's foreign install-table SQL is still pending.
- [ ] Manual acceptance: export/plan/apply for each contributed section, stable key resolution, conflicts and decisions, custom schema projections, formula validation, disabled sections, package versions and failed transactional writes.

## Outstanding scope

The remaining identity changes, shared capability interfaces, owner-only SQL, deletion policies and per-plugin manual acceptance matrices are still pending. Existing architectural debt is not considered resolved by renaming packages.

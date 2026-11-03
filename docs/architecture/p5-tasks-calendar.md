# P5.4 Tasks and calendar

`tasks` owns task records, task types, recurrence, links, reminders and its task inbox. Its canonical entities are `tasks.task` and `tasks.type`. Schemas, permissions and API routing use the task namespace directly for fresh installations.

`calendar` is an independent optional capability with no task plugin dependency or task-table queries. `aphel-contract-calendar` defines calendar queries, categories, owners and entries. The SDK's `CalendarSource` supplies authorized entries and delegates rescheduling to the record owner.

## Static review

- [x] Task SQL and schema ownership remain in tasks; calendar owns no record table.
- [x] Calendar aggregates enabled sources, validates a positive range of at most 62 days, normalizes start times to UTC, namespaces entry IDs by their provider, sorts entries and caps the combined response at 1000.
- [x] Task sources apply record scopes and scoped field metadata. Hidden title or date fields suppress calendar entries; hidden owner, category, duration and status metadata are omitted or blanked.
- [x] Rescheduling delegates to task update behavior, preserving field checks, record scopes, optimistic version checks, recurrence handling, audit and events.
- [x] Update availability is computed through a bulk reference visibility check using the update scope.
- [x] The calendar web package imports no task plugin. Generic entries drive its month, week, day and agenda views, and rescheduling uses the source route.
- [x] Task creation contributes through a typed UI slot; task mutations invalidate the calendar cache.
- [x] Calendar categories come from the authorized source catalog independently of the current event filters. Source/category pairs keep unrelated providers separate. Team options use privileged team listing only for settings managers and otherwise the session's own teams.
- [x] Generic record calendars retain separate platform translations and date/layout helpers.

## Manual acceptance pending

No database, test instance, automated test, probe or smoke run was executed under D4.

- [ ] Install a fresh schema and verify task creation, completion, reopening, recurrence and linked records.
- [ ] Enable calendar with and without tasks; a calendar without providers must remain usable and empty.
- [ ] Inspect month/week/day/agenda and multi-day rendering in both locales and different organization time zones.
- [ ] Check filters and hidden metadata for restricted users, including read-only and owner-only update scopes.
- [ ] Drag or reschedule a task, verifying conflicts on stale versions and rollback on invalid recurrence changes.
- [ ] Create a task from a calendar day and confirm cache refresh after create/update/delete elsewhere.

Existing domain-specific task inbox value projections and native consumer dependencies remain in the later P5 source/contract cleanup. They are not owned by calendar.

# Sharing and following records

Two generic features on top of the `RecordTypeRegistry`, valid for every record type (core, `quote`, `product`, `custom.<key>`):

- **Sharing** gives a user or team read or edit access to a record beyond what their role allows.
- **Following** makes the user get notified about changes, activities, notes and emails of the record.

Sharing is kernel: `back/kernel/src/application/record_sharing` (use cases and ports), `back/kernel/src/infrastructure/record_sharing` (MySQL), routes in `kernel/api/routes/v1/record_shares.rs` and web in `web/src/features/records/sharing`. Followers are the `followers` module: `back/modules/followers` (`service.rs`, `store.rs`, `http.rs`, the `followers.owners` consumer in `hooks.rs`) and `web/src/modules/followers` (button in the `record.detail.actions` slot, `/following` page). Kernel notifications get the followers through the `dyn RecordFollowerSource` service (`kernel/application/notifications/ports.rs`), provided by the module; without the module, nobody is notified as a follower.

## Data

- `record_shares` (migration `2670`): `record_type` (the type key, including `custom.<key>`), `record_id`, `grantee_type` (`user` or `team`), `grantee_id`, `access` (`read` or `edit`), `shared_by`, dates. Unique per record and grantee. The `(record_type, grantee_type, grantee_id, access, record_id)` index serves the scope filter subquery.
- `record_followers` (migration `2671`): `record_type`, `record_id`, `user_id`, `origin` (`manual` or `owner`). Unique per record and user.
- Administrative permission `records.share` (migration `2672` grants it to the system `admin` role; the admin already receives the whole catalog).

## Access to shared records

`AccessScope::filter_for(permission)` builds the `RecordScopeFilter` with `shares: ShareReach`, derived from the requested permission:

| Permission | `ShareReach` | Effect |
| --- | --- | --- |
| `*.read`, `*.export` | `Read` | read and edit shares (and membership, such as the deal team) |
| `*.update`, `*.archive` | `Edit` | only shares with `access = 'edit'` |
| any other (`*.delete`, `*.send`, `records.share`…) | `None` | sharing does not count |

With `own` or `team` scope, `push_record_scope_keyed` (called by `push_record_scope`/`push_record_scope_as`) adds to the filter `OR <key> IN (SELECT rs.record_id FROM record_shares rs WHERE rs.record_type = ? AND ((grantee_type = 'user' AND grantee_id = <user>) OR (grantee_type = 'team' AND grantee_id IN (<user's teams>))) [AND rs.access = 'edit'])`. With `organization` scope nothing changes. Since every record access goes through this filter, sharing applies to lists and detail pages, global search (the index uses `record_id` as the key), reports, aggregations and dashboard, export, timeline, attachments and notes (`MySqlRecordAccessChecker`), favorites, duplicates and notifications. The per-team pipeline restriction on deals still applies.

Types without an owner column: with `own`/`team` scope nothing was visible (`1 = 0`); now the shared record becomes visible (`1 = 0 OR <share>`).

Editing (updating, moving stage, archiving) a record shared with `edit` is allowed, as long as the user has the type's update permission in some scope. Deleting and re-sharing are never allowed through sharing alone.

## Who shares

Shares can be created, changed and removed by anyone who reads the record and is its owner (owner column) or has `records.share`. Someone who only reads the record sees the list (`can_manage = false`). A user grantee must be an active member (and cannot be the owner themselves); the team must exist in the organization. At most 100 shares per record.

API: `GET/POST /api/v1/records/{record_type}/{record_id}/shares`, `PATCH/DELETE /api/v1/records/{record_type}/{record_id}/shares/{share_id}` (`If-Match` on `PATCH`).

Each change publishes `record.shared`, `record.share_updated` or `record.unshared` (with the record, `share_id`, grantee, `access` and `changes`) and writes an audit entry in the same transaction. After the commit, it records a `permission_changed` security event with `metadata.change = "record_share"` (`action`, record type and id, grantee, access, actor). The events are not public (they are not in the webhook catalog).

Orphan shares are ignored, not deleted: a record in the trash stays out of queries (and comes back with its shares if restored); a user who leaves the organization has no scope; a deleted team is no longer among anyone's teams and appears without a name in the list.

## Following

- `POST/DELETE /api/v1/records/{record_type}/{record_id}/follow` and `GET .../followers` (active followers and whether the user follows). Following requires reading the record; unfollowing always works. It is a personal preference: it publishes no event and writes no audit entry (like favorites).
- `GET /api/v1/me/followed-records?record_type=`: followed records the user can still read (up to 2000 followed, filtered by each type's read scope and paginated).
- The owner follows automatically: the `followers.owners` consumer (`CatalogEventConsumer`) receives `<type>.created`, `<type>.updated` with `owner_id` in `changes` and `<type>.owner_changed`, reads the current owner and writes `origin = owner` (idempotent). The previous owner keeps following until they unfollow.

## Notifications to followers

The `notifications.events` consumer generates `followed_record_activity` (in-app on by default, email off; email always through the `notification.email` job) from the `FollowedRecordNotifications` source (`back/modules/followers/src/notifications.rs`), contributed by the `followers` module as a `NotificationSource` (`back/kernel/src/application/notifications/sources.rs`, order 100: it runs after the other sources for the same event), for:

- `<type>.updated` of the record itself and `deal.stage_changed`;
- `activity.created` (main record and the activity's links);
- `note.created` (not private) on the record;
- `email.received`/`email.sent` linked to the record (not private; without content).

It never notifies the actor, anyone who already got another notification for the same event (assignment, stage change to the owner, mention), nor anyone who can no longer read the record. `data.activity` indicates `update`, `activity`, `note` or `email`.

On `update`, `data.changes` carries the event's changes (`[{ field, label, old, new }]`) and the body lists up to three as `Label: before → after` (options by label, null values as `—`, `+N` when there are more). The changes respect the recipient: fields hidden for their role (`user_field_redaction`) are left out, `is_sensitive` fields appear only with the label and `"[redacted]"` as values, and if all the event's changes are hidden for them the notification is not created.

## Web

`RecordPageLayout` mounts, in the header of every record detail page, the Follow button (with the count and the list of followers) and the Share button (dialog with users/teams and read/edit access). The `/following` page ("Records I follow") has its own navigation item.

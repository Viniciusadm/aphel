# Scheduling links

The `scheduling` module (`back/modules/scheduling`, depends on `tasks` and `people`, optional `assignment`): use cases at the module root, `store/` (MySQL, CRM gateway, confirmation email), internal routes in `http.rs` and public ones in `public_http.rs`, the module migration `0001_create_scheduling.sql` (formerly `2330`–`2332`), the booking email worker and the `meeting.*` triggers registered in the module's `register`. Web: `web/src/modules/scheduling` (`/scheduling`, and the public pages `/book/:slug`, `/book/manage/:token`).

## Model

| Table | Contents |
| --- | --- |
| `scheduling_availability` | Per user: time zone, weekly hours (`weekly_hours`, 1 = Monday … 7 = Sunday), minimum notice, daily limit, buffers before/after and horizon in days |
| `meeting_types` | Link (globally unique `slug`, page `/book/<slug>`): duration, step between slots, location (`none`/`video`/`address`/`phone`), questions (JSON, with an optional `contact_field`), spam protection and `distribution_rule_id` (team link) |
| `meeting_bookings` | Booking: host, contact, activity, state (`confirmed` → `canceled`; the booking is created already confirmed in the same transaction as the contact and the activity), answers and `revision` (becomes the `.ics` `SEQUENCE`) |

Per-record permission `scheduling.manage`: `own` manages one's own availability and links; `team`/`organization` manage other members' and create team links.

## Free slots

`slots::free_slots` is pure: it generates the start times in the host's time zone, applies minimum notice, horizon, daily limit (counting bookings) and before/after buffers against the busy times. Busy =

- the host's open tasks with `due_at` and `duration_minutes` (from any record);
- `pending`/`confirmed` bookings;
- external calendars through the `CalendarAvailability` port, contributed by modules with `app.contribute::<dyn aphel_sdk::calendar::CalendarAvailability>(..)` (only enabled modules and connected users). Google and Microsoft implement it via `tasks::remote_calendar::availability` (`CalendarApi::busy`: Google `events.list` without transparent, canceled or declined events; Microsoft `calendarView` without `free`/`workingElsewhere`), with the OAuth scopes already requested (`calendar.events` and `Calendars.ReadWrite`; Google's `freeBusy` would require a new scope).

If an external calendar fails, the host has no slots (fail closed). The public API only returns free `{ starts_at, ends_at }` pairs, never appointment data.

Team link: the members come from `resolve_pool` of the `assignment` rule (active, not absent); the slots are the union of each member's free slots; when booking, among those free at that time, the host is chosen by the rule's strategy (`pick`, counters and `record_pick` with origin `scheduling`).

## Booking, rescheduling, canceling

Booking is a single transaction:

1. Before the transaction, the availability row of each candidate host is created with the defaults if it does not exist yet (`INSERT IGNORE`, outside the transaction). That way the following `SELECT .. FOR UPDATE` always locks an existing row, with no gap lock or deadlock between concurrent bookings for the same new host; whoever loses the race waits for the lock and gets 409 `booking_slot_unavailable` if the slot was taken.
2. In the transaction: choose the host, lock the availability, check the bookings again, find the contact by email or create it through the people use case (`people::service::create_in_tx`), owner = host, with the mapped answers (if the mapping is rejected, a savepoint is rolled back and the contact is created with only name and email). Create the `meeting` activity linked to the contact and assigned to the host through the tasks use case (`prepare_create` + `insert_prepared`), write the booking already `confirmed`, publish `meeting.booked` (`MeetingEventPayload`, record = contact, actor `system`), audit and enqueue `scheduling.booking_email` for the guest and the host. Any failure rolls everything back: no contact, activity or booking is left behind. The writes use the host's scope with full permissions (`booking_scope`), allowed only if the host is an active member with `scheduling.manage`.
3. After the commit, the `RecordCommitted` hook of the created contact runs.

Existing contact: if there is already a contact with the guest's email, the booking is linked to it (product decision: the email is the guest's identity). The existing contact is not changed and does not get mapped answers, and the person booking sees nothing about it (the public API returns only the booking).

Daylight saving time: weekly hours are interpreted in the host's time zone. Start times that fall in the gap skipped at the clock change (nonexistent local time) are not offered; for the repeated hour, the first occurrence applies. This is the expected behavior, not a bug.

Creating a user's first link without saved availability writes the default availability in the same transaction, with the `scheduling_availability.updated` event and an audit entry.

The email goes out through the installation's outgoing email (`smtp` integration, Settings → Email), with `text/calendar` + an `invite.ics` attachment (`METHOD:REQUEST`; cancellation `METHOD:CANCEL`). The management link is `/book/manage/<token>`, an HMAC-SHA256 token of the booking with an installation key in the `SecretVault` (`scheduling`/`default`/`booking_token_key`), generated once by `SecretVault::store_organization_if_absent` (race between the first two bookings: only the first key is stored and both reread the same one). Rescheduling moves the booking and the activity and publishes `meeting.rescheduled`; canceling (through the token or by the host) cancels the activity and publishes `meeting.canceled`. The three events are automation triggers (contact record) and public events.

## Public route protection

`/api/v1/public/book/*` requires neither a session nor `X-Requested-With` (allowed in `kernel/api/csrf.rs`). In-memory limits per IP (queries 120/min, bookings 10/h and 200/h per link, changes 30/h) and a `website` honeypot when `spam_protection` is on.

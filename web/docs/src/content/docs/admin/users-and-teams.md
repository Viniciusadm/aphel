---
title: Users and teams
description: Invite people, manage their status, and group them into teams that drive team-scoped access.
sidebar:
  order: 3
---

People who use an installation are **members**. You manage them under Settings, Users, and group them under Settings, Teams. Both screens are backed by the members and teams API, so everything here can be automated.

| Task | Permission needed |
| --- | --- |
| Invite, change the role of, suspend, reactivate or remove members; start a password reset; read the organization security events | `users.manage` |
| Create, edit and delete teams; manage roles | `settings.manage` |

See [Roles and permissions](/docs/admin/roles-and-permissions/) for how permissions are granted.

## Statuses

| Status | Meaning |
| --- | --- |
| `invited` | An invitation was sent and not accepted yet. The member cannot sign in. |
| `active` | The member can sign in and has the access of their role. |
| `deactivated` | The member cannot sign in and has no access. Existing sessions were ended. |

A member always has exactly one role. The first administrator is created by the installer.

## Invitations

1. Open Settings, Users and choose Invite user.
2. Enter the email address and pick a role. The address must not belong to an existing member.
3. Aphel creates the member as `invited` and emails a link to the public address of the installation. The link is valid for 7 days.
4. The person opens the link, optionally sets a display name, chooses a password (8 to 128 characters) and becomes `active`.

If outgoing email is not configured, Aphel cannot send the message. The invitation is still created and the screen shows the link to copy, so you can hand it over through another channel. See [Configuration](/docs/admin/configuration/).

For a pending invitation you can:

- **Resend** it. Previous links stop working and a new one is sent (or shown).
- **Cancel** it. This deletes the pending member and invalidates the links.

## Change a role

Choose Change role on a member. Aphel refuses a change that would leave the organization without an active administrator (`last_admin`). Permission changes are recorded as security events.

## Suspend, reactivate and remove

| Action | Effect |
| --- | --- |
| Suspend | An `active` member becomes `deactivated` and is signed out. Team memberships are kept. |
| Reactivate | A `deactivated` member becomes `active` again. |
| Remove | The member becomes `deactivated`, is signed out and is taken out of every team. A pending invitation is deleted outright. |

Records and history that mention the person are not deleted by any of these actions. Aphel never removes or suspends the last active administrator.

## Password reset

For an active member with a password, choose the password reset action. Earlier reset links are invalidated and an email goes out; the link is valid for 1 hour. Without outgoing email, nothing is sent and the screen shows the link for you to hand over. Members who sign in only through single sign-on have no password to reset.

Members can change their own password under Account security. That signs out their other sessions and sends them a notice by email.

## Teams

A team is a named group of members.

- A team can have a **parent team**, which builds a hierarchy. A change that would create a cycle is rejected (`team_cycle`).
- Each member of a team can be marked as a **manager** of it.
- Deleting a team moves its sub-teams to the top level. It does not delete members.
- Replacing the member list of a team replaces it as a whole.

Teams matter for access. A role grant with the `team` scope reaches the records owned by members of the user's teams **and of their sub-teams**. See [Roles and permissions](/docs/admin/roles-and-permissions/#scopes).

## Via the API

| Operation | Request |
| --- | --- |
| List members | `GET /api/v1/members` |
| Invite | `POST /api/v1/members/invitations` with `email` and `role_id` |
| Change role | `PATCH /api/v1/members/{user_id}` |
| Resend or cancel an invitation | `POST /api/v1/members/{user_id}/resend-invitation`, `DELETE /api/v1/members/{user_id}/invitation` |
| Suspend or reactivate | `POST /api/v1/members/{user_id}/suspend`, `POST /api/v1/members/{user_id}/reactivate` |
| Remove | `DELETE /api/v1/members/{user_id}` |
| Password reset | `POST /api/v1/members/{user_id}/password-reset` |
| Teams | `GET`, `POST /api/v1/teams`; `GET`, `PATCH`, `DELETE /api/v1/teams/{team_id}` |

Updates accept an `If-Match` header with the `updated_at` you read. If the member or team changed meanwhile, the API answers `409 version_conflict`. See the [REST API reference](/docs/reference/rest-api/).

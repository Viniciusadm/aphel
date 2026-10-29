# Approvals

A generic approval engine over the installation's `RecordTypeRegistry` (`state.registry()`): it applies to the core types, to `quote` and to custom objects (`co_<key>`).

## Model

| Table | Content |
| --- | --- |
| `approval_policies` | Policy: stable `key`, `record_type`, `moment` (`stage_change`, `quote_send`, `on_demand`), `target_id` (stage in `stage_change`), condition (`always`, `filter` with the AST from `kernel/application/query/filter.rs`, boolean `formula`), `locked_fields` (empty = whole record), `due_hours`, `is_active`, `position` |
| `approval_policy_steps` | Ordered steps: approver `user`, `role` or `owner_team_manager` and rule `any`/`all` |
| `approval_requests` | Request with a snapshot of the policy (`policy_name`, `steps`, `locked_fields`), `status` (`pending`, `approved`, `rejected`, `cancelled`), `current_step`, `due_at`, final decision and `consumed_at`. Only one pending request per policy + record + moment + target |
| `approval_request_approvers` | Approvers materialized per step (only when the step starts), with decision and comment |

Code: `modules/approvals/domain.rs` (enums), `modules/approvals` (`ApprovalGate` contract, `ApprovalEngine`, policy and request use cases, package section, automation, consumer and job), `modules/approvals/store.rs` (MySQL, directory, `MySqlApprovalGate`, guard), `api/routes/v1/approvals.rs`.

## Approvers

- `user`: the user, if they are an active member.
- `role`: active members of the role.
- `owner_team_manager`: managers (`team_memberships.is_manager`) of the record owner's teams; if there are none, the managers of the parent teams.
- The requester never approves their own request. If the step resolves nobody other than them, members with `approvals.manage` approve. If there is still nobody, the request is not opened: `submit` checks all steps before saving and responds 409 `approval_no_approver` (`details` with `policy_id`, `policy_key`, `policy_name`, `record_type`, `record_id`, `step`, `step_name`), and the change that depended on the approval stays blocked until the policy or the roles are fixed. If the next step loses all eligible approvers after the request is open (user deactivated, role changed), approving the current step is refused with the same code; the request stays pending and can be rejected or cancelled. There is never a pending request without an approver.
- `any`: one approval closes the step. `all`: every materialized approver must approve. A rejection ends the request.

## Condition

The condition is evaluated with an elevated scope (all permissions, the same user and teams as the requester) to see the record and all fields: the policy belongs to the organization and cannot depend on the field permissions of whoever triggers the change.

- `filter`: validated with `validate_view_query_related` when the policy is saved and evaluated as a `RecordQuery` with the filter **and** `id = <record>` (including archived), compiled by `build_query` and executed **in the write's transaction**.
- `formula`: compiled by the `kernel/application/fields/formulas` engine as a provisional boolean formula field (like the `preview`) and evaluated by the `FormulaEngine`. The formula reads through the pool, so it sees the record's already saved state, not the transaction's uncommitted changes.

A condition that no longer compiles (removed field) does not apply.

## Checkpoints and consumption

`ApprovalGate` (implementation `MySqlApprovalGate::new(pool, registry, events, audit)`; in routes use `approval_gate(&state, &scope)` from `api/routes/v1/approvals.rs`):

- `checkpoint(tx, scope, cp)`: active policies of the type + moment (+ stage) whose condition holds. None → `Ok`. A pending request for any of them → 409 `approval_pending`. All with an approved, unconsumed request → sets `consumed_at` in the transaction and `Ok`. Otherwise → 409 `approval_required` with `details { record_type, record_id, moment, target_id, policies: [{ id, key, name }] }`.
- `submit(tx, scope, submission)`: requires edit access to the record (the type's `update` permission + visible record). Opens one request per applicable policy that does not yet have a pending or approved-but-unconsumed request, materializes the first step's approvers and publishes `approval.requested`. Returns `NotRequired`, `Pending(..)` or `Approved(..)`.
- `cancel_open` cancels the record's pending requests; `requests_for` lists the record's requests (for users who can read the record).

An approval is valid for **one** change: the next `checkpoint` that passes consumes it.

## Write guard

`ApprovalWriteGuard` is the priority 10 handler of the `RecordValidating` hook ([`record-pipeline.md`](record-pipeline.md)), registered by the `approvals` module (`back/modules/approvals/src/mod.rs`), and runs on every edit (`WriteMode::Update`) of the record use cases, before the validation rule (409 before 422):

1. If there is a pending request for the record and any changed field is covered by `locked_fields` (`locks_field`), it responds 409 `approval_pending` with `details { request_id, fields, ... }`.
2. If `stage_id` changed, it calls `checkpoint(stage_change(record, new stage))`.

The guard only reads and consumes (no events). Note: a write to the record itself in the same transaction **after** a `submit` that opened a request with the whole record locked is refused; save the record before the `submit`.

## How a module uses it

```rust
let gate = approval_gate(&state, &scope).await?;
// inside the use case's transaction
match gate.submit(&mut tx, scope, &ApprovalSubmission { checkpoint: ApprovalCheckpoint::quote_send(record), comment, ip }).await? {
    SubmissionOutcome::NotRequired | SubmissionOutcome::Approved(_) => { /* continue */ }
    SubmissionOutcome::Pending(_) => { /* wait for approval.approved/rejected/cancelled */ }
}
// when sending
gate.checkpoint(&mut tx, scope, &ApprovalCheckpoint::quote_send(record)).await?;
```

To react to the decision, consume `approval.approved|rejected|cancelled` filtering by `record_type`.

## Events

Published with the target record's `RecordRef` and payload `{ request_id, policy_id, policy_key, policy_name, moment, target_id, record_type, record_id, requested_by, step, comment, resolution_comment }` (+ `approvers` when there are new approvers):

- `approval.requested`, `approval.approved`, `approval.rejected`, `approval.cancelled` (public only for deals in `public_event_catalog()`: the kernel catalog ties each event type to a single resource, so requests for quotes and other types produce the internal event, the notification and the automation trigger, but no webhook; automation triggers with an optional `policy_key` filter).
- Internal: `approval.step_advanced` (next step), `approval.decision_recorded` (vote on an `all` step that is still open), `approval.overdue` (deadline passed, once per request), `approval_policy.created|updated|deleted|reordered`.

Every mutation writes an audit entry with the target record in the same transaction.

## Notifications, deadline and cleanup

- `notifications.events` consumer: `approval_requested` to the step's approvers (request and next step), `approval_decided` to the requester, `approval_overdue` to the approvers who have not decided yet. Link `/approvals/<id>`.
- The `approvals.overdue` job (every 10 minutes) sets `overdue_notified_at` and publishes `approval.overdue` once per overdue request.
- `approvals.cleanup` consumer: on `<type>.deleted`, cancels the record's pending requests.
- Deleting a policy cancels its pending requests.

## Automation

- Triggers `approval.requested|approved|rejected|cancelled` (optional `policy_key`).
- Action `request_approval` (`policy_key` of an `on_demand` policy, optional `comment`) via `AutomationGateway::request_approval`, which reuses `ApprovalRequestUseCases::submit` restricted to the policy.

## Configuration packages

Section `approval_policies` (`ApprovalPoliciesSection`): reference by `key`, target stage by `<pipeline>.<stage>`, role by `role_key`, user by `user_email`. Installing creates or updates, never deletes and does not touch requests.

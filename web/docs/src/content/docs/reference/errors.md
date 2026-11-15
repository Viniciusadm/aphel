---
title: Errors
description: The error envelope of the Aphel API, validation details, the generic status codes and a table of every error code the platform and its modules can return.
sidebar:
  order: 3
---

Every error response uses the same JSON envelope, whether it comes from the platform, a module, a plugin or a middleware layer:

```json
{
  "error": {
    "code": "version_conflict",
    "message": "the resource changed since it was read; reload it and apply the change again",
    "details": null
  }
}
```

| Property | Type | Meaning |
| --- | --- | --- |
| `code` | string | A stable, machine-readable identifier in snake case. Decide what to do based on the code (and the HTTP status), never on the message. |
| `message` | string | A human-readable explanation, in English, meant for logs and developers. It can change between versions. |
| `details` | object or null | Extra structured information. Present for validation errors (`fields`), rate limits (`retry_after`, `limit`) and a few specific codes. |

Server errors (`5xx`) never expose internals: an unexpected failure answers `500 internal_error` with the message `internal server error` and the details go to the server log. The exceptions are `503 configuration_error` and the `503` codes of unavailable services, whose messages explain what to configure.

## Validation errors

Invalid input answers `422`. Most validation errors have `code: "validation_error"` and a `details.fields` array with one entry per invalid field:

```json
{
  "error": {
    "code": "validation_error",
    "message": "invalid record values",
    "details": {
      "fields": [
        { "field": "email", "message": "is not a valid email address" },
        { "field": "custom_fields.budget", "message": "must be a number" }
      ]
    }
  }
}
```

| Field of an entry | Meaning |
| --- | --- |
| `field` | Path of the invalid input. Body properties use their name, custom fields use `custom_fields.<key>`, query parameters use their name (`page`, `filter`, `sort`), filter conditions use paths such as `filter.conditions.0.value`, and headers use their name (`If-Match`). |
| `message` | What is wrong with it. |
| `code` | Optional machine code of that single violation. Module settings use codes such as `setting_required`, `setting_too_long`, `setting_not_number`, `setting_out_of_range`, `setting_invalid_option`, `setting_invalid_email`, `setting_invalid_url`, `setting_not_boolean`, `setting_not_text`, `setting_unknown` and `setting_invalid_field`. |

Some validation failures carry a dedicated top-level code instead of `validation_error`, with the same `details.fields` structure. Examples are `field_not_accessible` (the role cannot see a field used in a filter, sort or write), `field_read_only`, `validation_rule_failed`, `stage_requirements_missing` and `lead_form_invalid_submission`. A body that is not valid JSON, or has the wrong `Content-Type`, answers `422 validation_error` without `fields`.

## Generic errors

These apply to every route.

| HTTP | Code | Meaning |
| --- | --- | --- |
| 401 | `unauthorized` | No valid session, or the account is not active. |
| 401 | `invalid_api_token` | The Bearer token is malformed, unknown, expired, revoked or its creator is deactivated. |
| 403 | `forbidden` | Generic refusal without a more specific code. |
| 403 | `permission_denied` | The caller lacks the permission or the record is outside their record scope. |
| 403 | `insufficient_scope` | The API token does not carry the scope the route needs. |
| 403 | `csrf_header_missing` | A session request that changes data did not send `X-Requested-With: aphel`. |
| 403 | `api_token_not_allowed` | The route accepts only browser sessions. |
| 403 | `two_factor_setup_required` | The organization requires two-factor authentication and the user has not enabled it. |
| 404 | `not_found` | Generic missing resource. Most routes use a specific code such as `deal_not_found`. |
| 404 | `module_disabled` | The route belongs to a module that is disabled. |
| 409 | `conflict` | Generic state conflict. |
| 409 | `version_conflict` | The `If-Match` version no longer matches. |
| 413 | `payload_too_large` | The request body is larger than the route accepts. |
| 422 | `validation_error` | Invalid input; see above. |
| 429 | `rate_limited` | A rate limit was exceeded. `Retry-After` and `details.retry_after` tell how long to wait. |
| 500 | `internal_error` | Unexpected failure. |
| 503 | `configuration_error` | The installation lacks required configuration, such as the database. |

## Error codes by area

The tables below list every code that the platform and the built-in modules define: 556 codes. A code can be returned with more than one status when it is raised in different situations; all statuses are listed. Runtime plugins can also raise errors with codes of their own; those are not listed here. Messages shown here are summaries; the exact text in a response can carry more detail, such as the name of a field or a limit.

### Platform

| Code | Status | Meaning |
| --- | --- | --- |
| `already_attached` | 409 | The file is already attached to this record. |
| `already_member` | 409 | This user already belongs to the organization. |
| `api_token_not_allowed` | 403 | This route only accepts a browser session, not an API token. |
| `api_token_not_found` | 404 | API token was not found. |
| `approval_moment_unknown` | 422 | The approval moment is not one of the moments known to this installation. |
| `approval_not_revocable` | 409 | Only an approval that was not used yet can be revoked. |
| `attachment_not_found` | 404 | Attachment was not found. |
| `concurrent_modification` | 409 | The record was changed by another request; retry. |
| `contract_unavailable` | 503 | No enabled module provides the operation or contract that was called. |
| `csrf_header_missing` | 403 | Requests that change data must send the X-Requested-With: aphel header. |
| `current_session_not_revocable` | 422 | The current session is ended by signing out. |
| `custom_object_not_found` | 404 | Custom object was not found. |
| `custom_record_not_found` | 404 | Record was not found. |
| `duplicate_values` | 409 | Existing records share values of this field, so it cannot be unique. |
| `duplicated_values` | 409 | Existing records share values of the field, so it cannot be made unique. |
| `email_taken` | 409 | Email is already registered. |
| `empty_file` | 422 | The file is empty. |
| `entity_unique_conflict` | 409 | Record violates a unique entity index. |
| `field_key_taken` | 409 | A field with that key already exists on the record type. |
| `field_managed_by_module` | 409 | The field is managed by a module or plugin and cannot be changed. |
| `field_not_accessible` | 422 | The role of the user cannot see these fields. |
| `field_not_found` | 404 | Field was not found. |
| `field_option_not_found` | 404 | Option was not found. |
| `field_permission_locked` | 422 | Required and structural fields are always editable. |
| `field_read_only` | 422 | The role of the user cannot change these fields, or the fields are kept by their owner (plugin `read_only` fields change only through the plugin or a module listed in `writers`). |
| `field_without_options` | 422 | Only select and multi_select fields have options. |
| `file_not_found` | 404 | File was not found. |
| `file_too_large` | 413/422 | The file is too large to be read by a module. |
| `file_type_not_allowed` | 422 | This file type is not allowed. |
| `idempotency_key_in_progress` | 409 | A request with this Idempotency-Key is still being processed; retry later. |
| `idempotency_key_reused` | 422 | This Idempotency-Key was already used with a different request. |
| `incompatible_field_type` | 422 | The field cannot change to that type. |
| `insufficient_scope` | 403 | The API token does not have the scope required by the route. |
| `invalid_api_token` | 401 | The API token is malformed, unknown, expired, revoked or belongs to a deactivated user. |
| `invalid_current_password` | 422 | The current password is not correct. |
| `invalid_member_status` | 409 | The member status does not allow the action. |
| `invalid_plugin_manifest` | 422 | The plugin manifest is invalid. |
| `invalid_token` | 422 | The link token is invalid, already used or expired. |
| `invalid_two_factor_code` | 422 | The two-factor code is wrong, expired or already used. |
| `job_not_found` | 404 | Failed job was not found. |
| `last_admin` | 409 | The organization needs at least one active administrator. |
| `linked_record_not_found` | 404 | Linked record was not found. |
| `login_locked` | 429 | Too many failed sign-in attempts; the account is locked for that IP until `details.retry_after` seconds pass. |
| `logo_not_found` | 404 | The organization has no logo. |
| `mail_attachments_unsupported` | 422 | This mailer cannot send attachments. |
| `mail_not_configured` | 422 | No outgoing mail transport is configured. |
| `member_not_found` | 404 | Member was not found. |
| `metrics_disabled` | 404 | Metrics are disabled. |
| `metrics_unauthorized` | 401 | A valid metrics token is required. |
| `module_confirmation_required` | 422 | Type the module key to confirm. |
| `module_dependency_cycle` | 409 | Modules depend on each other in a cycle. |
| `module_dependency_disabled` | 409 | A module the requested one depends on is disabled; enable it first. |
| `module_disabled` | 404 | This module is disabled. |
| `module_enabled` | 409 | Disable the module before removing its data. |
| `module_export_required` | 409 | Export the data of the module before removing it. |
| `module_held_by_extension` | 409 | A plugin that provides the same contract keeps the module disabled. |
| `module_incompatible` | 409 | This module is not compatible with the installed Aphel or its dependencies. |
| `module_not_found` | 404 | Module not found. |
| `module_required` | 409 | This module is required by this build and cannot be disabled. |
| `module_required_by` | 409 | Other enabled modules depend on this one; disable them first. |
| `module_required_by_extension` | 409 | An enabled plugin requires a contract, operation or permission of this module; disable or uninstall the plugin first. |
| `module_settings_required` | 422 | Configure the required settings before enabling the module. |
| `module_setup_unsupported` | 404 | This module has no setup hints. |
| `module_test_unsupported` | 404 | This module has no settings test. |
| `mutation_already_applied` | 409 | The durable mutation was already committed. |
| `mutation_multiple_transactions` | 409 | A durable mutation must use one committing transaction. |
| `not_invited` | 409 | This member has no pending invitation. |
| `operation_key_taken` | 422 | An operation with that key is already provided natively. |
| `option_value_taken` | 409 | The field already has an option with that value. |
| `payload_too_large` | 413 | The request body is larger than this route accepts. |
| `permission_already_registered` | 409 | Another owner already registered the permission. |
| `permission_denied` | 403 | The caller lacks the permission, or the record is outside the caller's record scope. |
| `plugin_authority_inactive` | 403 | The original authority is no longer active. |
| `plugin_execution_cycle` | 409 | The plugin execution chain is cyclic or exceeds 16 entries. |
| `publication_approvals_unavailable` | 409 / 503 | A configured approval provider is disabled (blocked publication), or an enabled provider cannot be evaluated (retryable). Enable or restore the provider. |
| `media_asset_in_use` | 409 | The library file is attached to other records; remove it from them before deleting the asset. |
| `site_target_unavailable` | 422 | The page chosen for the site home page or menu is not published on the site. |
| `publication_definition_invalid` | 409 | The publication definition of the record type does not match its fields. |
| `publication_idempotency_conflict` | 409 | The idempotency key was already used for a different publication command. |
| `publication_not_publishable` | 422 | The record type does not take part in publication. |
| `publication_payload_too_large` | 413 | The content is larger than 2 MiB and cannot be published. |
| `publication_record_not_found` | 404 | The record does not exist or you cannot see it. |
| `publication_request_not_failed` | 409 | Only a failed publication request can be retried. |
| `publication_request_not_found` | 404 | The publication request does not exist or you cannot see it. |
| `publication_request_not_open` | 409 | The publication request is no longer open. |
| `publication_revision_not_found` | 404 | The revision does not exist or you cannot see it. |
| `rate_limited` | 429 | Too many requests; wait for `Retry-After` seconds. Also returned for an API token above its limit and by a few module-specific throttles. |
| `record_has_relations` | 409 | The record still has linked records that block its deletion. |
| `record_not_accessible` | 403 | The record does not exist or you cannot access it. |
| `record_not_found` | 404/422 | Record was not found. |
| `record_operation_not_supported` | 422 | The record type does not support the operation. |
| `record_referenced` | 409 | Another record still references this one. |
| `record_share_exists` | 409 | The record is already shared with this user or team. |
| `record_share_not_allowed` | 403 | Only the owner of the record or someone with records.share can manage its shares. |
| `record_share_not_found` | 404 | Record share was not found. |
| `record_shares_limit_reached` | 422 | A record reached the maximum number of shares. |
| `record_type_not_found` | 404 | The record type is not registered. |
| `relation_limit_reached` | 409 | The relationship allows a single linked record on this side. |
| `relation_not_found` | 404 | Relation was not found on this record type. |
| `relation_required` | 409 | The relationship requires at least one linked record. |
| `role_in_use` | 409 | The role is assigned to members of the organization. |
| `role_key_taken` | 409 | Another role already uses this key. |
| `role_not_found` | 404 | Role was not found. |
| `session_not_found` | 404 | Session not found. |
| `sso_required` | 403 | This email must sign in with single sign-on. |
| `system_field_locked` | 422 | Only the label and the position of system fields can be changed. |
| `system_role_locked` | 409 | System roles cannot be deleted, renamed by key or, for the administrator, have permissions changed. |
| `team_cycle` | 422 | A team cannot be placed under itself or one of its sub-teams. |
| `team_not_found` | 404 | Team was not found. |
| `too_many_api_tokens` | 422 | The user already has the maximum number of active API tokens (50). |
| `tenant_not_found` | 404 | No tenant is served at the requested host (multi-tenant installations only). |
| `tenant_unavailable` | 503 | The tenant at the requested host is suspended, being provisioned or being deleted, or could not be started. |
| `too_many_fields` | 422 | The record type reached the maximum number of custom fields. |
| `two_factor_already_enabled` | 409 | Two-factor authentication is already enabled. |
| `two_factor_attempts_exceeded` | 401 | Too many wrong codes; the partial session was revoked and the user must sign in again. |
| `two_factor_not_enabled` | 409 | Enable two-factor authentication on your account before requiring it. |
| `two_factor_setup_not_started` | 409 | Start the two-factor setup before confirming it. |
| `two_factor_setup_required` | 403 | This organization requires two-factor authentication; enable it in your security settings. |
| `unsupported_record_type` | 422 | The record type is not supported for this operation. |
| `version_conflict` | 409 | The record changed since the `If-Match` version; read it again and re-apply the change. |

### Multi-tenant console

Returned by `/api/console/v1` on `CONSOLE_HOST` and by `aphel tenants`.

| Code | Status | Meaning |
| --- | --- | --- |
| `console_invalid_credentials` | 401 | Wrong email or password at the console sign-in. |
| `console_session_required` | 401 | The console request has no valid `aphel_console_session` cookie; sign in again. |
| `console_sign_in_locked` | 429 | Too many wrong passwords for this operator; sign-in is locked for a few minutes. |
| `operator_email_invalid` | 422 | The operator email is not valid. |
| `operator_email_taken` | 409 | Another operator already uses this email. |
| `operator_invitation_invalid` | 404 | The invitation link is invalid, expired or already used. |
| `operator_name_invalid` | 422 | The operator name needs 1 to 140 characters. |
| `operator_not_found` | 404 | Operator was not found, or is already deactivated. |
| `operator_not_invited` | 409 | Only invited operators get a new invitation link. |
| `operator_password_invalid` | 422 | The password needs 8 to 128 characters. |
| `operator_password_wrong` | 422 | The current password is wrong. |
| `operator_self_deactivation` | 409 | Operators cannot deactivate themselves. |
| `tenant_deletion_unconfirmed` | 422 | The deletion was not confirmed by repeating the tenant slug. |
| `tenant_host_invalid` | 422 | The host is not a valid host name (no scheme, path or port). |
| `tenant_host_not_found` | 404 | The tenant has no such host. |
| `tenant_host_taken` | 409 | Another tenant already uses this host. |
| `tenant_last_host` | 409 | A tenant needs at least one host. |
| `tenant_name_invalid` | 422 | The tenant name needs 1 to 140 characters. |
| `tenant_slug_invalid` | 422 | The slug needs 3 to 40 lowercase letters, digits or single hyphens, and must fit the database name. |
| `tenant_slug_taken` | 409 | Another tenant already uses this slug. |
| `tenant_status_conflict` | 409 | The tenant is not in the state the action needs (for example resuming a tenant that is not suspended). |

### AI assistant (`ai`)

| Code | Status | Meaning |
| --- | --- | --- |
| `ai_empty_response` | 503 | The AI provider returned an empty answer. |
| `ai_feature_disabled` | 403 | The requested AI feature is turned off in the module settings. |
| `ai_invalid_calls_query` | 422 | The query parameter that filters AI calls must be a positive integer. |
| `ai_invalid_instruction` | 422 | The writing assistant request has invalid fields. |
| `ai_invalid_prompt` | 422 | The prompt has invalid fields. |
| `ai_invalid_response` | 503 | The AI provider answered with something the module could not use. |
| `ai_limit_reached` | 429 | The organization reached its daily limit of AI requests. |
| `ai_not_configured` | 503 | The AI provider is not configured yet. |
| `ai_provider_auth_failed` | 503 | The AI provider rejected the configured credentials. |
| `ai_provider_error` | 503 | The AI provider could not be reached or answered with an error. |
| `ai_provider_rate_limited` | 429 | The AI provider is rate limiting this organization, try again later. |
| `ai_provider_timeout` | 503 | The AI provider did not answer in time. |
| `ai_query_invalid` | 422 | The natural language question has invalid fields. |
| `ai_query_no_fields` | 422 | There are no filterable fields you can use on this record type. |
| `ai_query_unanswerable` | 422 | The question could not be turned into a query over the record fields. |
| `ai_record_not_found` | 404 | Record not found. |
| `ai_refused` | 422 | The AI model declined this request. |
| `ai_unsupported_record_type` | 422 | The record type is not supported by the AI feature. |
| `ai_user_limit_reached` | 429 | You reached your hourly limit of AI requests, try again later. |
| `invalid_filter` | 422 | The filter is not valid for the record type. |
| `organization_not_found` | 404 | Organization not found. |

### Approvals (`approvals`)

| Code | Status | Meaning |
| --- | --- | --- |
| `approval_no_approver` | 409 | No eligible approver exists for an approval step. |
| `approval_not_approver` | 403 | You are not an approver of the current step of this request. |
| `approval_not_pending` | 409 | The approval request is no longer pending. |
| `approval_pending` | 409 | An approval request is pending for the record, so it cannot change. |
| `approval_policy_key_taken` | 409 | Another approval policy already uses this key. |
| `approval_policy_not_found` | 404 | Approval policy was not found. |
| `approval_request_not_found` | 404 | Approval request was not found. |
| `approval_required` | 409 | The change needs an approval first. |

### Assignment (`assignment`)

| Code | Status | Meaning |
| --- | --- | --- |
| `assignment_no_eligible_member` | 422 | No member of the lead distribution rule is available. |
| `assignment_record_type_mismatch` | 422 | The distribution rule distributes a different record type than the one given. |
| `assignment_rule_inactive` | 422 | The lead distribution rule is not active. |
| `assignment_rule_key_taken` | 409 | Another lead distribution rule already uses this key. |
| `assignment_rule_not_found` | 404 | Lead distribution rule was not found. |

### Automations (`automations`)

| Code | Status | Meaning |
| --- | --- | --- |
| `automation_key_taken` | 409 | Another automation already uses this key. |
| `automation_limit_reached` | 409 | The organization reached the maximum number of automations. |
| `automation_not_active` | 409 | Activate the automation before running it. |
| `automation_not_found` | 404 | Automation not found. |
| `automation_not_manual` | 409 | Only automations with the manual trigger can be run on demand; use the dry run to test the others. |
| `automation_not_published` | 409 | Publish the automation before activating or pausing it. |
| `automation_run_not_found` | 404 | Automation run not found. |
| `invalid_automation` | 422 | The automation definition has invalid fields. |

### Booking (`booking`)

| Code | Status | Meaning |
| --- | --- | --- |
| `booking_not_changeable` | 409 | This booking was canceled or already happened. |
| `booking_not_found` | 404 | Booking was not found. |
| `booking_rejected` | 422 | The booking could not be accepted. |
| `booking_slot_unavailable` | 409 | This time is no longer available; pick another one. |
| `meeting_type_not_found` | 404 | Meeting type was not found. |
| `meeting_type_slug_taken` | 409 | Another booking link already uses this address. |

### Bulk actions (`bulk_actions`)

| Code | Status | Meaning |
| --- | --- | --- |
| `bulk_action_not_supported` | 422 | The action cannot be applied to this record type. |
| `bulk_empty_selection` | 422 | No records were selected. |
| `bulk_not_supported` | 422 | The record type does not support bulk actions. |
| `bulk_operation_not_found` | 404 | Bulk operation was not found. |
| `bulk_too_large` | 422 | The selection has more than 10,000 records; narrow the filter. |

### Calls (`telephony`)

| Code | Status | Meaning |
| --- | --- | --- |
| `call_activity_type_missing` | 422 | The organization has no call activity type. |
| `call_not_found` | 404 | Call was not found. |
| `call_outcome_key_taken` | 409 | Another call outcome already uses this key. |
| `call_outcome_not_found` | 404 | Call outcome was not found. |
| `call_recording_not_found` | 404 | The call has no recording. |
| `call_session_completed` | 409 | The call session is already completed. |
| `call_session_empty` | 422 | The view has no people to call. |
| `call_session_item_not_found` | 404 | The call session has no contact at this position. |
| `call_session_not_found` | 404 | Call session was not found. |
| `call_session_view_not_found` | 404 | Saved view of people was not found. |
| `last_active_call_outcome` | 409 | The organization needs at least one active call outcome. |
| `telephony_provider_not_found` | 404 | Telephony provider is not available. |

### CNPJ enrichment (`enrichment`)

| Code | Status | Meaning |
| --- | --- | --- |
| `company_not_found` | 404 | Company was not found. |
| `enrichment_automatic_disabled` | 422 | Automatic enrichment is turned off in the module settings. |
| `enrichment_cnpj_missing` | 422 | Informe o CNPJ da empresa. |
| `enrichment_cnpj_not_found` | 404 | The CNPJ was not found in the data source. |
| `enrichment_invalid_cnpj` | 422 | CNPJ inválido. |
| `enrichment_invalid_request` | 422 | The request has invalid fields. |
| `enrichment_lookup_expired` | 422 | The lookup expired; query the CNPJ again. |
| `enrichment_lookup_not_found` | 404 | The lookup was not found. |
| `enrichment_nothing_to_apply` | 422 | No selected item has a mapped field and a value different from the current one. |
| `enrichment_source_rate_limited` | 429 | A ReceitaWS aceita poucas consultas por minuto; tente novamente em instantes. |
| `enrichment_source_unavailable` | 503 | Enrichment source unavailable. |

### Configuration packages (`config_packages`)

| Code | Status | Meaning |
| --- | --- | --- |
| `aphel_too_old` | 422 | The package requires a newer Aphel version than the one installed. |
| `checksum_mismatch` | 422 | The package does not match the SHA-256 it declares and was not installed. |
| `config_package_install_not_found` | 404 | Installation was not found. |
| `invalid_decision` | 422 | The conflict decisions of the configuration package are invalid. |
| `invalid_package` | 422 | The configuration package is not valid. |
| `invalid_resolution` | 422 | The conflict resolutions of the configuration package version are invalid. |
| `plan_outdated` | 409 | The organization changed since the plan was made; review the plan again. |
| `template_not_found` | 404 | Template was not found. |

### Dashboard (`dashboard`)

| Code | Status | Meaning |
| --- | --- | --- |
| `invalid_aggregate` | 422 | The aggregation request of a widget is not valid. |
| `widget_not_found` | 404 | Widget was not found. |

### Deals (`deals`)

| Code | Status | Meaning |
| --- | --- | --- |
| `deal_amount_managed_by_items` | 422 | The amount of this deal is the sum of its items. |
| `deal_credit_exceeds_total` | 422 | The revenue credit of the members would exceed 100 percent; `details` tells how much is left. |
| `deal_member_exists` | 409 | The user is already a member of this deal. |
| `deal_member_not_found` | 404 | Deal member was not found. |
| `deal_member_role_key_taken` | 409 | Another deal member role already uses this key. |
| `deal_member_role_not_found` | 404 | Deal member role was not found. |
| `deal_members_limit_reached` | 422 | A deal reached the maximum number of team members. |
| `no_default_pipeline` | 409 | The organization has no pipeline. |
| `pipeline_not_found` | 404 | Pipeline was not found. |
| `workflow_catalog_unavailable` | 503 | Workflow metadata is not enabled. |

### Duplicates (`duplicates`)

| Code | Status | Meaning |
| --- | --- | --- |
| `duplicate_candidate_not_found` | 404 | Duplicate candidate was not found. |
| `duplicate_candidate_resolved` | 409 | The duplicate candidate was already resolved. |
| `merge_same_record` | 422 | A record cannot be merged into itself. |

### Electronic signature (`esign`)

| Code | Status | Meaning |
| --- | --- | --- |
| `esign_delivery_claimed` | 409 | The signature request is no longer pending. |
| `esign_delivery_unknown` | 409 | The previous provider call has an unknown outcome; reconcile it with the provider before retrying. |
| `esign_file_not_attached` | 422 | The chosen file is not attached to the deal. |
| `esign_file_not_pdf` | 422 | Only PDF files can be sent for signature. |
| `esign_file_too_large` | 422 | The PDF exceeds the 20 MB limit for signature requests. |
| `esign_invalid_request` | 422 | The signature request has invalid fields. |
| `esign_invalid_webhook_payload` | 422 | The webhook body must be JSON. |
| `esign_not_configured` | 503 | The Clicksign access token is not configured in the module settings. |
| `esign_owner_unavailable` | 503 | No active member can act on this signature request. |
| `esign_provider_rejected` | 422 | The signature provider rejected the request. |
| `esign_provider_unauthorized` | 503 | O provedor de assinatura recusou o token de acesso configurado. |
| `esign_provider_unavailable` | 503 | The signature provider could not be reached. |
| `esign_request_closed` | 409 | This signature request cannot be cancelled. |
| `esign_request_not_found` | 404 | The signature request was not found. |
| `esign_signed_document_unavailable` | 503 | The signed PDF is not available at the provider yet. |
| `esign_webhook_not_configured` | 503 | The webhook secret is not configured. |
| `esign_webhook_payload_too_large` | 422 | The webhook body exceeds 512 KB. |
| `invalid_webhook_signature` | 401 | The webhook signature is missing, malformed, outside the time window or wrong. |

### Email (`email`)

| Code | Status | Meaning |
| --- | --- | --- |
| `email_attachment_not_found` | 404 | Attachment was not found. |
| `email_attachments_too_large` | 422 | The attachments of the e-mail exceed the size limit. |
| `email_block_exists` | 409 | This entry is already in the blocklist. |
| `email_block_not_found` | 404 | Blocklist entry was not found. |
| `email_delivery_unknown` | 409 | The previous send has an unknown outcome; confirm delivery with the provider before sending again. |
| `email_link_not_found` | 404 | The conversation is not linked to this record. |
| `email_not_cancellable` | 409 | Only scheduled e-mails that were not sent yet can be cancelled. |
| `email_not_found` | 404 | E-mail was not found. |
| `email_template_key_taken` | 409 | An e-mail template with that key already exists. |
| `email_template_not_found` | 404 | E-mail template was not found. |
| `email_thread_not_found` | 404 | E-mail conversation was not found. |
| `email_tracking_link_not_found` | 404 | Link was not found. |
| `email_tracking_objection_exists` | 409 | This address already objected to e-mail tracking. |
| `email_tracking_objection_not_found` | 404 | Tracking objection was not found. |
| `invalid_state` | 422 | The OAuth state parameter is invalid or expired. |
| `mail_oauth_provider_unavailable` | 503 | Mail OAuth provider is not enabled. |
| `mailbox_address_mismatch` | 422 | The new settings belong to a different mailbox address. |
| `mailbox_already_connected` | 409 | This mailbox is already connected. |
| `mailbox_disconnected` | 409 | The mailbox is disconnected. |
| `mailbox_not_found` | 404 | Mailbox was not found. |
| `mailbox_unavailable` | 422 | The mailbox is disconnected; reconnect it before sending. |
| `no_mailbox_available` | 422 | The user that would send the sequence is not an active member. |
| `sso_not_configured` | 422 | Configure the client id and secret first. |

### Exports (`exports`)

| Code | Status | Meaning |
| --- | --- | --- |
| `export_in_progress` | 409 | The export is still being prepared and cannot be deleted yet. |
| `export_not_found` | 404 | Export was not found. |
| `export_not_supported` | 422 | The record type cannot be exported. |
| `export_too_large` | 422 | The export exceeds the maximum number of records; narrow the filter. |

### Favorites (`favorites`)

| Code | Status | Meaning |
| --- | --- | --- |
| `favorite_already_exists` | 409 | This item is already a favorite. |
| `favorite_limit_reached` | 409 | The user reached the maximum number of favorites. |
| `favorite_not_found` | 404 | Favorite not found. |
| `favorite_target_not_found` | 404 | The record or view does not exist or is not visible. |

### Formulas (`formulas`)

| Code | Status | Meaning |
| --- | --- | --- |
| `formula_cycle` | 422 | The formula depends on itself through other formulas. |

### Goals (`goals`)

| Code | Status | Meaning |
| --- | --- | --- |
| `goal_already_exists` | 409 | A goal with this owner, metric and period already exists. |
| `goal_not_found` | 404 | Goal was not found. |

### Imports (`imports`)

| Code | Status | Meaning |
| --- | --- | --- |
| `empty_import` | 422 | The file must have a header line and at least one data row. |
| `import_not_editable` | 409 | The import was already started and can no longer be changed. |
| `import_in_progress` | 409 | The import is queued or running and cannot be deleted. |
| `import_not_found` | 404 | Import was not found. |
| `import_not_retryable` | 409 | Privacy-redacted imports cannot be changed or retried. |
| `import_not_supported` | 422 | The record type cannot be imported. |
| `invalid_csv` | 422 | The file is not a valid CSV. |
| `invalid_spreadsheet` | 422 | The file is not a valid spreadsheet. |
| `privacy_import_busy` | 409 | An affected import is still queued or running. |
| `too_many_columns` | 422 | The file has more columns than the import allows. |
| `too_many_rows` | 422 | The file has more rows than the import allows. |

### Integrations (`integrations`)

| Code | Status | Meaning |
| --- | --- | --- |
| `integration_activation_failed` | 422 | The integration could not be activated; the message carries the provider reason. |
| `integration_not_configurable` | 422 | This integration has no configuration of its own. |
| `integration_not_configured` | 422 | The integration is not configured yet. |
| `integration_provider_not_found` | 404 | Integration provider was not found. |
| `integration_test_failed` | 422 | The connection test failed; the message carries the provider reason. |

### Lead forms (`forms`)

| Code | Status | Meaning |
| --- | --- | --- |
| `lead_form_invalid` | 422 | The lead form definition has invalid fields. |
| `lead_form_invalid_payload` | 422 | O corpo precisa ser um objeto JSON. |
| `lead_form_invalid_query` | 422 | A query parameter of the lead form routes must be a positive integer. |
| `lead_form_invalid_submission` | 422 | Revise os campos destacados. |
| `lead_form_limit_reached` | 409 | The limit of 200 lead forms was reached. |
| `lead_form_not_found` | 404 | Lead form not found. |
| `lead_form_origin_not_allowed` | 403 | The form does not accept submissions from this site. |
| `lead_form_owner_unavailable` | 503 | The form is not accepting submissions right now. |
| `lead_form_payload_too_large` | 422 | The submission exceeds the 64 KB limit. |
| `lead_form_submission_failed` | 503 | The submission could not be recorded; try again later. |

### Leads (`leads`)

| Code | Status | Meaning |
| --- | --- | --- |
| `contact_not_lead` | 409 | The contact is not in a lifecycle stage that allows this action. |
| `disqualification_reason_key_taken` | 409 | Another disqualification reason already uses this key. |
| `disqualification_reason_not_found` | 404 | Disqualification reason was not found. |

### Line items (`line_items`)

| Code | Status | Meaning |
| --- | --- | --- |
| `line_item_not_found` | 404 | Line item was not found. |
| `line_items_limit_reached` | 422 | A record reached the maximum number of line items. |
| `product_currency_mismatch` | 422 | The product is priced in another currency. |
| `record_currency_has_items` | 422 | The currency of a record with items cannot be changed. |
| `record_has_no_items` | 422 | The record has no items to sum. |

### Messaging channels (`messaging`)

| Code | Status | Meaning |
| --- | --- | --- |
| `communication_opt_out_exists` | 409 | This address already has an opt-out. |
| `communication_opt_out_not_found` | 404 | The opt-out does not exist. |
| `communication_recipient_opted_out` | 403 | The recipient asked not to receive this kind of message. |
| `invalid_job_payload` | 422 | A background job payload is missing a required value. |
| `messaging_channel_in_use` | 409 | The channel has conversations; deactivate it to keep the history. |
| `messaging_channel_inactive` | 409 | The channel of this conversation is inactive. |
| `messaging_channel_invalid` | 422 | The channel definition has invalid fields. |
| `messaging_channel_limit_reached` | 409 | The limit of 50 channels was reached. |
| `messaging_channel_not_found` | 404 | Messaging channel not found. |
| `messaging_conversation_not_found` | 404 | Conversation not found. |
| `messaging_invalid_assignee` | 422 | The user must be an active member with access to the inbox. |
| `messaging_invalid_payload` | 422 | The webhook body is not valid JSON. |
| `messaging_invalid_query` | 422 | A query parameter of the inbox is invalid. |
| `messaging_invalid_reply` | 422 | Write a message. |
| `messaging_payload_too_large` | 422 | Payload above 1 MB. |
| `messaging_record_not_accessible` | 422 | The record does not exist or the user cannot read it. |
| `whatsapp_invalid_signature` | 401 | The WhatsApp webhook signature is wrong. |
| `whatsapp_invalid_verify_token` | 403 | The WhatsApp verification token is wrong. |

### Money (`money`)

| Code | Status | Meaning |
| --- | --- | --- |
| `exchange_rate_exists` | 409 | The currency already has a rate on this date. |
| `exchange_rate_not_found` | 404 | Exchange rate was not found. |
| `exchange_rate_provider_not_configured` | 422 | No exchange rate provider is configured. |
| `exchange_rate_provider_unavailable` | 503 | The exchange rate provider could not be reached. |

### Notes (`notes`)

| Code | Status | Meaning |
| --- | --- | --- |
| `note_not_found` | 404 | Note was not found. |

### Notifications (`notifications`)

| Code | Status | Meaning |
| --- | --- | --- |
| `notification_not_found` | 404 | Notification was not found. |

### Objects (`objects`)

| Code | Status | Meaning |
| --- | --- | --- |
| `custom_object_has_records` | 409 | A custom object with records cannot be deleted; deactivate it instead. |
| `custom_object_key_taken` | 409 | A custom object with that key already exists. |
| `custom_object_managed_by_module` | 409 | The custom object is managed by a module or plugin and cannot be changed here. |
| `relationship_key_taken` | 409 | A relationship with that key already exists. |
| `relationship_not_found` | 404 | Relationship was not found. |
| `too_many_custom_objects` | 422 | The organization reached the maximum number of custom objects. |
| `too_many_relationships` | 422 | The organization reached the maximum number of relationships. |

### Full export (`org_export`)

| Code | Status | Meaning |
| --- | --- | --- |
| `invalid_organization_export` | 422 | The backup file is not a valid organization export. |
| `organization_export_in_progress` | 409 | An organization export is already queued or running. |
| `organization_export_not_found` | 404 | Organization export was not found. |
| `organization_export_not_ready` | 409 | The organization export has no file to download. |
| `organization_not_empty` | 409 | This installation is already set up; a backup is only restored into an empty installation. |

### Organizations (`organizations`)

| Code | Status | Meaning |
| --- | --- | --- |
| `company_hierarchy_cycle` | 422 | The parent company would create a cycle in the company hierarchy. |
| `company_hierarchy_too_deep` | 422 | The company hierarchy would exceed the maximum depth. |

### People (`people`)

| Code | Status | Meaning |
| --- | --- | --- |
| `already_linked` | 409 | The contact is already linked to this company. |
| `contact_not_found` | 404 | Contact was not found. |
| `link_not_found` | 404 | The contact is not linked to this company. |

### Price lists (`pricing`)

| Code | Status | Meaning |
| --- | --- | --- |
| `price_list_key_taken` | 409 | Another price list already uses this key. |
| `price_list_not_found` | 404 | Price list was not found. |
| `product_bundle_nested` | 409 | The product is a component of another bundle and cannot become a bundle. |
| `product_bundle_not_found` | 404 | The product is not a bundle. |

### Privacy (`privacy`)

| Code | Status | Meaning |
| --- | --- | --- |
| `invalid_subject_term` | 422 | The data subject search term is not valid. |
| `privacy_export_unavailable` | 404 | The export file of this request is not available. |
| `privacy_request_not_found` | 404 | Privacy request was not found. |
| `privacy_request_not_pending` | 409 | Only pending requests can be cancelled or executed. |
| `privacy_targets_empty` | 422 | Select at least one existing record or source item. |
| `retention_not_supported` | 422 | This record type cannot be archived. |
| `retention_policy_limit` | 422 | The organization reached the maximum number of retention policies. |
| `retention_policy_not_found` | 404 | Retention policy was not found. |
| `retention_preview_required` | 409 | Preview the policy (less than an hour ago and after its last change) before enabling or running it. |

### Proposals (`proposals`)

| Code | Status | Meaning |
| --- | --- | --- |
| `proposal_invalid_request` | 422 | The request has invalid fields. |
| `proposal_template_inactive` | 422 | The proposal template is inactive. |
| `proposal_template_invalid` | 422 | The proposal template is invalid. |
| `proposal_template_key_taken` | 409 | A template with this key already exists. |
| `proposal_template_not_found` | 404 | Proposal template not found. |
| `proposal_template_required` | 422 | Choose a template: there is no active template. |
| `quote_not_found` | 404 | Quote was not found. |

### Quotes (`quotes`)

| Code | Status | Meaning |
| --- | --- | --- |
| `currency_mismatch` | 422 | The currency of the quote differs from the currency of the deal. |
| `deal_not_found` | 404 | Deal was not found. |
| `quote_already_accepted` | 409 | An accepted quote cannot be deleted. |
| `quote_closed` | 409 | This quote was already answered. |
| `quote_expired` | 409 | This quote is no longer valid. |
| `quote_has_no_items` | 422 | Add at least one item to the quote. |
| `quote_invalid_transition` | 409 | The quote cannot move from its current state to the requested one. |
| `quote_items_limit_reached` | 422 | A quote reached the maximum number of items. |
| `quote_not_current` | 409 | Only the current version of a quote can change; create a new version. |
| `quote_not_editable` | 409 | The quote was already sent or closed; create a new version to change it. |
| `quote_not_sent` | 409 | The quote has not been sent yet. |
| `quote_price_list_not_found` | 422 | The price list was not found. |
| `quote_superseded` | 409 | This quote was replaced by a newer version. |

### Record layouts (`record_layouts`)

| Code | Status | Meaning |
| --- | --- | --- |
| `layout_context_not_found` | 404 | Layout context must be create, edit or view. |

### Reminders (`reminders`)

| Code | Status | Meaning |
| --- | --- | --- |
| `reminder_not_found` | 404 | Reminder was not found. |
| `task_reminders_unavailable` | 503 | Task reminders are not enabled. |

### Reports (`reports`)

| Code | Status | Meaning |
| --- | --- | --- |
| `report_not_found` | 404 | Report was not found. |
| `report_period_required` | 422 | The report has no period field to override. |
| `report_schedule_not_found` | 404 | Report schedule was not found. |

### Saved views (`saved_views`)

| Code | Status | Meaning |
| --- | --- | --- |
| `view_not_found` | 404 | View was not found. |

### Scoring (`scoring`)

| Code | Status | Meaning |
| --- | --- | --- |
| `scoring_model_not_found` | 404 | Scoring model was not found. |

### Segments (`segments`)

| Code | Status | Meaning |
| --- | --- | --- |
| `segment_name_taken` | 409 | A segment with that name already exists. |
| `segment_not_found` | 404 | Segment was not found. |
| `too_many_segments` | 422 | The record type reached the maximum number of segments. |

### Sequences (`sequences`)

| Code | Status | Meaning |
| --- | --- | --- |
| `email_consent_unavailable` | 503 | Email consent checks are not enabled. |
| `email_template_catalog_unavailable` | 503 | A referenced email template is unavailable. |
| `sequence_already_enrolled` | 409 | The contact is already in this sequence. |
| `sequence_contact_objected` | 422 | The contact objected to receiving tracked e-mails. |
| `sequence_contact_without_email` | 422 | The contact has no e-mail address. |
| `sequence_empty` | 422 | The sequence has no steps to execute. |
| `sequence_enrollment_finished` | 409 | The enrollment is not in a state that allows this change. |
| `sequence_enrollment_not_found` | 404 | Sequence enrollment was not found. |
| `sequence_inactive` | 422 | The sequence is inactive. |
| `sequence_key_taken` | 409 | Another sequence already uses this key. |
| `sequence_not_found` | 404 | Sequence was not found. |
| `sequence_record_type_unsupported` | 422 | The record type cannot be enrolled in a sequence. |

### Single sign-on (OIDC) (`sso`)

| Code | Status | Meaning |
| --- | --- | --- |
| `invalid_grant` | 422 | The authorization was revoked or expired. |
| `not_a_member` | 403 | The user is not an active member of the organization. |

### Tags (`tagging`)

| Code | Status | Meaning |
| --- | --- | --- |
| `record_type_not_taggable` | 422 | The record type cannot be tagged. |
| `tag_has_children` | 409 | The term has terms below it; move or delete them first. |
| `tag_in_use` | 409 | The term is selected in records that keep their own classification; `details.records` counts them per entity. |
| `tag_name_taken` | 409 | A term of the same vocabulary already has that name. |
| `tag_not_allowed` | 422 | The tag cannot be used on that record type, or the term belongs to a vocabulary the record does not use. |
| `tag_not_found` | 404 | Tag was not found. |
| `tag_slug_taken` | 409 | A term of the same vocabulary already uses that address. |
| `tag_vocabulary_has_hierarchy` | 409 | The hierarchy cannot be turned off while terms are nested. |
| `tag_vocabulary_key_taken` | 409 | A vocabulary with that key already exists. |
| `tag_vocabulary_not_empty` | 409 | The vocabulary still has terms. |
| `tag_vocabulary_not_found` | 404 | The vocabulary was not found. |
| `tag_vocabulary_protected` | 409 | The `general` vocabulary cannot be deleted. |
| `too_many_tag_vocabularies` | 422 | The organization reached the maximum of 50 vocabularies. |
| `too_many_tags` | 422 | The organization reached the maximum number of tags. |

### Tasks (`tasks`)

| Code | Status | Meaning |
| --- | --- | --- |
| `activity_already_completed` | 409 | The activity is already done. |
| `activity_already_open` | 409 | The activity is already open. |
| `activity_not_found` | 404 | Activity was not found. |
| `activity_type_key_taken` | 409 | Another activity type already uses this key. |
| `activity_type_not_found` | 404 | Activity type was not found. |
| `calendar_api_failed` | 403/422 | The calendar provider refused the access; check the granted permissions. |
| `calendar_authorization_revoked` | 422 | The calendar authorization was revoked or expired; connect the calendar again. |
| `calendar_cursor_expired` | 422 | The sync cursor expired. |
| `calendar_event_not_found` | 404 | The calendar event no longer exists. |
| `calendar_meeting_type_not_found` | 422 | The activity type used for the meeting does not exist or is inactive. |
| `calendar_no_refresh_token` | 422 | The provider did not grant offline access. |
| `calendar_not_connected` | 404/422 | The calendar is not connected. |
| `calendar_provider_unavailable` | 503 | The calendar provider is not enabled. |
| `last_active_activity_type` | 409 | The organization needs at least one active activity type. |
| `task_transactions_unavailable` | 503 | Task transactions are not enabled. |

### Trash (`trash`)

| Code | Status | Meaning |
| --- | --- | --- |
| `trash_item_not_found` | 404 | The record is not in the trash. |

### Validation rules (`validation_rules`)

| Code | Status | Meaning |
| --- | --- | --- |
| `validation_rule_failed` | 422 | The record breaks validation rules. |
| `validation_rule_not_found` | 404 | Validation rule was not found. |

### WebAssembly runtime (`wasm_runtime`)

| Code | Status | Meaning |
| --- | --- | --- |
| `organization_not_installed` | 404 | The organization is not installed yet. |
| `plugin_backend_invalid` | 422 | backend.wasm is not a valid component of the plugin world. |
| `plugin_backend_missing` | 422 | The plugin declares runtime callbacks but has no backend.wasm. |
| `plugin_batch_invalid` | 422 | An item of a batch of record writes is invalid; the message names the item. |
| `plugin_catalog_changed` | 503 | The plugin changed while its backend was loading. |
| `plugin_catalog_unavailable` | 503 | The plugin catalog is not ready; retry the request. |
| `plugin_conflicts_unacknowledged` | 409 | The plugin overlaps with another installed plugin; send `acknowledge_conflicts=true` to proceed. |
| `plugin_contract_unavailable` | 422 | No enabled provider exists for a contract the plugin requires. |
| `plugin_contribution_unknown` | 404 | The plugin does not contribute the requested item to the extension point. |
| `plugin_dependency_unavailable` | 422 | A declared permission depends on a module that is not enabled, or the plugin declares entities while the module that administers objects is disabled. |
| `plugin_directory_unavailable` | 403 | The directory needs a signed-in actor or the plugin installer. |
| `plugin_disabled` | 403/503 | The plugin is disabled or uninstalled. |
| `plugin_effect_invalid` | 422 | Effect keys must have 1 to 128 bytes; commands are limited to 64 KiB and 100 per hook. |
| `plugin_effect_key_conflict` | 409 | The effect key was already used with another command. |
| `plugin_effect_limit` | 422 | An execution can defer at most 100 distinct effects. |
| `plugin_effect_not_supported` | 422 | The record type does not support durable single-transaction mutations. |
| `plugin_effect_required` | 409 | Retryable plugin handlers must use defer-effect for side effects. |
| `plugin_email_not_allowed` | 403 | Declare emails.send in requires to send e-mail. |
| `plugin_event_not_declared` | 403/404 | The plugin did not declare the event in `[[emits]]`. |
| `plugin_failed` | 503 | The plugin backend failed to run; the plugin fails closed. |
| `plugin_field_conflict` | 409 | The field already exists on the record type. |
| `plugin_field_not_writable` | 403 | The hook cannot change that field. |
| `plugin_field_type_changed` | 409 | A new plugin version tries to change the type of an existing field. |
| `plugin_field_unique_conflict` | 409 | Existing records share values of fields the plugin declares unique. |
| `plugin_file_too_large` | 413 | The file is larger than plugins are allowed to read. |
| `plugin_has_dependents` | 409 | The change would break other enabled plugins that depend on it. |
| `plugin_hook_patch_invalid` | 422 | Hook patches must be JSON objects. |
| `plugin_host_not_allowed` | 403/422 | The URL host is not allowed by the plugin's `allowed_hosts` or by the network policy. |
| `plugin_http_effect_failed` | 503 | The remote service answered with an error; the request is retried with the same Idempotency-Key. |
| `plugin_http_failed` | 503 | The outbound HTTP call of the plugin failed. |
| `plugin_kv_not_a_counter` | 409 | The sequence was set to a negative value. |
| `plugin_manifest_invalid` | 422 | plugin.toml is invalid. |
| `plugin_module_conflict_blocked` | 409 | The module cannot be disabled while a plugin uses it. |
| `plugin_not_found` | 404 | The plugin is not installed. |
| `plugin_notification_invalid` | 422 | The notification sent by the plugin is invalid. |
| `plugin_notification_not_declared` | 403/404/503 | The plugin did not declare the notification kind. |
| `plugin_notification_recipient_invalid` | 422 | The recipient must be a user id. |
| `plugin_notification_record_not_readable` | 403 | The recipient cannot read the record of the notification. |
| `plugin_object_conflict` | 409 | A custom object with that key already exists. |
| `plugin_operation_not_declared` | 403 | The plugin did not list the operation in `operations`. |
| `plugin_operation_permission_invalid` | 422 | The operation requires an invalid permission. |
| `plugin_operation_timeout` | 503 | The operation did not finish within the plugin call time. |
| `plugin_operation_unknown` | 404 | The operation does not exist or its module is disabled. |
| `plugin_management_disabled` | 403 | The platform operator turned plugin and module management off for this organization. |
| `plugin_installer_missing` | 409 | A plugin installed from the operator console needs an active administrator to run on behalf of, and the organization has none. |
| `plugin_owner_inactive` | 503 | The plugin acts with the permissions of the member who installed it, who is no longer active. |
| `plugin_pack_aphel_incompatible` | 422 | The pack needs a newer version of Aphel (`requires_aphel`). |
| `plugin_pack_dependency_cycle` | 422 | Two plugins of the pack depend on each other. |
| `plugin_pack_invalid` | 422 | The `.aphelpack` file or its `pack.toml` is invalid. |
| `plugin_pack_missing_dependency` | 422 | A plugin of the pack needs an operation that neither Aphel nor the pack offers. |
| `plugin_pack_module_unavailable` | 422 | The pack needs a module that is not part of this Aphel build (`requires_modules`). |
| `plugin_pack_plugins_invalid` | 422 | One or more plugins of the pack are invalid; each plugin lists its own error. |
| `plugin_pack_provision_reference` | 422 | A pack resource references a resource or field the pack does not create. |
| `plugin_pack_provisioner_unknown` | 422 | No enabled module creates resources of that `kind`. |
| `plugin_pack_setting_invalid` | 422 | A setting of the pack is unknown, a secret or has an invalid value. |
| `plugin_package_invalid` | 422 | The `.aphelplugin` package is invalid. |
| `plugin_query_authority_changed` | 403 | The authority of the cached external query has changed. |
| `plugin_query_outcome_unknown` | 409 | The external query already started and has no completed result; it will not be called again automatically. |
| `plugin_raw_body_invalid` | 422 | The content type of the raw route body is not allowed. |
| `plugin_raw_body_outside_route` | 409 | Respond-bytes is only available while handling a route. |
| `plugin_raw_body_too_large` | 413 | The route body exceeds the maximum size. |
| `plugin_read_only` | 403 | This plugin execution cannot produce side effects. |
| `plugin_site_invalid` | 422 | The `[site]` section or the `site/` templates of the package break a rule; the message lists every problem. |
| `site_route_conflict` | 409 | A site page of the plugin has the same path shape as a page of another installed plugin. |
| `site_path_taken` | 422 | The address is already used by another page of the site. |
| `site_request_forbidden` | 403 | A site route call that changes data came without the `X-Aphel-Site` header sent by the site scripts. |
| `site_route_not_found` | 404 | No installed plugin has this site route. |
| `site_path_redirect_shadowed` | 409 | A page of the site already answers at the old address of the redirect, so the redirect would never apply. |
| `site_redirect_not_found` | 404 | The site redirect does not exist. |
| `plugin_reference_permission_required` | 422 | A reference field requires a permission the plugin does not declare. |
| `plugin_role_preset_unknown` | 422 | Unknown role presets. |
| `plugin_route_not_found` | 404 | The plugin has no route for that method and path. |
| `plugin_secret_not_declared` | 404 | The plugin did not declare that secret setting. |
| `plugin_settings_invalid` | 422 | The plugin settings have invalid fields. |
| `plugin_unavailable` | 503 | The plugin is not installed or is disabled. |
| `plugin_web_unacknowledged` | 409 | The plugin ships integrated web code that an administrator must explicitly accept for this version. |
| `plugin_write_in_hook` | 409 | Plugins cannot execute side effects inside a record hook; use a durable event handler. |

### Webhooks (`webhooks`)

| Code | Status | Meaning |
| --- | --- | --- |
| `incoming_webhook_disabled` | 403 | The incoming webhook is disabled. |
| `incoming_webhook_not_found` | 404 | Incoming webhook was not found. |
| `incoming_webhook_owner_unavailable` | 403 | The member who created the incoming webhook is no longer active. |
| `incoming_webhook_permission_denied` | 403 | The creator of the incoming webhook lacks the permission for an action. |
| `incoming_webhook_request_not_found` | 404 | Incoming webhook request was not found. |
| `invalid_webhook_payload` | 422 | The body has an unsupported content type or is malformed. |
| `too_many_incoming_webhooks` | 422 | The organization reached the maximum number of incoming webhooks. |
| `too_many_webhook_endpoints` | 422 | The organization reached the maximum number of webhook endpoints (20). |
| `webhook_delivery_not_found` | 404 | Webhook delivery was not found. |
| `webhook_endpoint_disabled` | 409 | The webhook endpoint is disabled; enable it before resending. |
| `webhook_endpoint_not_found` | 404 | Webhook endpoint was not found. |
| `webhook_mapping_failed` | 422 | An action rejected the values mapped from the payload; earlier actions are kept. |
| `webhook_payload_too_large` | 413 | The incoming webhook body exceeds 256 KB. |
| `webhook_replayed` | 409 | A delivery with the same id or signature was already received in the last 24 hours. |
| `webhook_url_not_allowed` | 422 | The webhook URL points to an address that is not allowed. |

### Workflow (`workflow`)

| Code | Status | Meaning |
| --- | --- | --- |
| `checklist_item_not_found` | 404 | Checklist item was not found. |
| `closing_stages_required` | 409 | A pipeline needs at least one active open, won and lost stage. |
| `default_pipeline_required` | 409 | An inactive pipeline cannot be the default one. |
| `last_lost_reason` | 409 | At least one lost reason must stay active. |
| `lost_reason_not_found` | 404 | Lost reason was not found. |
| `pipeline_has_records` | 409 | The pipeline has records; choose a target pipeline to move them to. |
| `pipeline_limit_reached` | 409 | The organization reached the maximum number of pipelines. |
| `pipeline_stage_not_found` | 404 | Pipeline stage was not found. |
| `stage_checklist_incomplete` | 422 | The checklist of the current stage has required items that are not done. |
| `stage_has_records` | 409 | The kind of a stage with records cannot change. |
| `stage_requirements_inaccessible` | 422 | The stage requires fields that are empty and that you cannot fill. |
| `stage_requirements_missing` | 422 | The stage requires fields that are empty. |

### Optional services

Answered when a module needs a capability that another module provides and that module is not enabled. The status is `503`.

| Code | Status | Meaning |
| --- | --- | --- |
| `catalog_references_unavailable` | 503 | Catalog references are not enabled. |
| `configuration_keys_unavailable` | 503 | Configuration key service is not enabled. |
| `configuration_packages_unavailable` | 503 | Configuration packages is not enabled. |
| `configuration_versions_unavailable` | 503 | Configuration versioning is not enabled. |
| `currency_not_enabled` | 422 | The currency is not enabled for the organization. |
| `deal_probability_reader_unavailable` | 503 | Deal probability reading is not enabled. |
| `document_signature_provider_unavailable` | 503 | The document signature provider is unavailable. |
| `document_source_unavailable` | 404 | The document source is unavailable. |
| `email_sender_unavailable` | 503 | Email sending is not enabled. |
| `exports_unavailable` | 503 | Record exports are not enabled. |
| `formula_planner_unavailable` | 503 | Formula definition planning is not enabled. |
| `formula_predicates_unavailable` | 503 | Formula predicates are not enabled. |
| `identity_flows_unavailable` | 503 | Identity flows are not enabled. |
| `import_drafts_unavailable` | 503 | Import drafts are not enabled. |
| `import_sources_unavailable` | 503 | Import sources are not enabled. |
| `mailbox_oauth_unavailable` | 503 | Mailbox OAuth is not enabled. |
| `managed_object_writer_unavailable` | 503 | Managed object administration is not enabled. |
| `meeting_scheduler_unavailable` | 503 | Meeting scheduling is not enabled. |
| `messaging_provider_unavailable` | 409 | The messaging provider is not available. |
| `note_writer_unavailable` | 503 | Note creation is not enabled. |
| `notification_delivery_unavailable` | 503 | Notification delivery is not enabled. |
| `oauth_protocol_unavailable` | 503 | OAuth protocol support is not enabled. |
| `oauth_states_unavailable` | 503 | OAuth state support is not enabled. |
| `person_writer_unavailable` | 503 | Person creation is not enabled. |
| `prospective_predicates_unavailable` | 503 | Prospective formula evaluation is not enabled. |
| `remote_calendars_unavailable` | 503 | Remote task calendars are not enabled. |
| `sequence_enrollment_unavailable` | 503 | Sequence enrollment is not enabled. |
| `task_types_unavailable` | 503 | Task types are not enabled. |
| `task_writer_unavailable` | 503 | Task creation is not enabled. |
| `workflow_views_unavailable` | 503 | Workflow views are not enabled. |

### Site appearance

- `site_appearance_invalid`: unsupported scope, property, value, image or collection target. Use the appearance catalog and an available target.
- `site_page_invalid`: a site page has an address that is not lowercase letters, digits and hyphens, or blocks and translations that cannot be saved. Fix the field named in the message in Site → Pages.
- `site_design_invalid`: a template, part, pattern, menu or font has an invalid block, link, CSS or file. Fix the item named in the message in Site → Site editor.
- `block_style_invalid`: a block style uses an unknown property or a value outside the site tokens.
- `blocks_too_deep`: blocks are nested more than 6 levels.
- `site_styles_invalid`: the styles document has an unknown key, a color that is not `#rrggbb`, a size that is not in rem, an unknown font or block style value. Fix the value named in the message.
- `site_appearance_unavailable`: Publication is disabled. Enable it before editing or publishing appearance.
- `site_appearance_pending`: appearance provisioned by a pack has not completed publication. Resolve its publication request, then retry the import.

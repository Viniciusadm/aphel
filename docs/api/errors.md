# Error codes

Every error comes out in the `{ "error": { "code", "message", "details" } }` envelope. The `code` is stable: integrations must decide based on the code, never on the message. A new code is only ever added; renaming or removing a code is a breaking change (see [`versioning.md`](versioning.md)).

## General

| Code | HTTP | When |
| --- | --- | --- |
| `validation_error` | 422 | Validation; `details.fields[]` lists `{ field, message }`. Also for malformed JSON bodies and invalid parameters |
| `unauthorized` | 401 | No session or invalid session |
| `forbidden` | 403 | Generic access denied |
| `not_found` | 404 | Route or resource does not exist |
| `conflict` | 409 | Generic conflict |
| `payload_too_large` | 413 | Request body above the route limit (2 MB by default for JSON; uploads follow the limit in Settings → General). Routes with their own limit use a specific code (e.g. `webhook_payload_too_large`) |
| `rate_limited` | 429 | Request limit (login, password recovery, API tokens, public module routes such as capture forms and webhooks); `details` carries `retry_after` (seconds) and `limit` when known, and then the response includes `Retry-After`. The counters are shared by all server processes |
| `internal_error` | 500 | Internal error; details only in the log |
| `configuration_error` | 503 | Server without a database or a required configuration |

## By resource

| Code | HTTP | Area |
| --- | --- | --- |
| `activity_already_completed` | 409 | activities |
| `activity_already_open` | 409 | activities |
| `activity_not_found` | 404 | activities |
| `activity_type_key_taken` | 409 | activities |
| `activity_type_not_found` | 404 | activities |
| `already_attached` | 409 | files |
| `already_linked` | 409 | contacts |
| `already_member` | 409 | organization_admin |
| `api_token_not_allowed` | 403 | auth |
| `api_token_not_found` | 404 | api_tokens |
| `attachment_not_found` | 404 | files |
| `automation_key_taken` | 409 | automations |
| `automation_limit_reached` | 409 | automations |
| `automation_not_active` | 409 | automations |
| `automation_not_found` | 404 | automations |
| `automation_not_manual` | 409 | automations |
| `automation_not_published` | 409 | automations |
| `automation_run_not_found` | 404 | automations |
| `bulk_action_not_supported` | 422 | bulk_actions |
| `bulk_empty_selection` | 422 | bulk_actions |
| `bulk_not_supported` | 422 | bulk_actions |
| `bulk_operation_not_found` | 404 | bulk_actions |
| `bulk_too_large` | 422 | bulk_actions |
| `closing_stages_required` | 409 | pipelines |
| `company_not_found` | 404 | companies |
| `config_package_install_not_found` | 404 | config_packages |
| `contact_not_found` | 404 | contacts |
| `csrf_header_missing` | 403 | geral |
| `current_session_not_revocable` | 422 | security |
| `custom_object_has_records` | 409 | custom_objects |
| `custom_object_key_taken` | 409 | custom_objects |
| `custom_object_not_found` | 404 | custom_objects |
| `custom_record_not_found` | 404 | custom_objects |
| `deal_amount_managed_by_items` | 422 | deals |
| `deal_currency_has_items` | 422 | deal_items (the currency of a deal with items cannot be changed: the item prices are in the current currency; remove the items first) |
| `deal_has_no_items` | 422 | deal_items |
| `deal_item_not_found` | 404 | deal_items |
| `deal_items_limit_reached` | 422 | deal_items |
| `deal_not_found` | 404 | deals |
| `default_pipeline_required` | 409 | pipelines |
| `demo_already_seeded` | 409 | demo (the installation already has records; use `aphel demo remove` or `--force`) |
| `demo_not_installed` | 409 | demo (installation without an active administrator; run the installer first) |
| `duplicate_candidate_not_found` | 404 | duplicates |
| `duplicate_candidate_resolved` | 409 | duplicates |
| `duplicate_values` | 409 | fields |
| `email_taken` | 409 | auth, installer |
| `empty_file` | 422 | files |
| `empty_import` | 422 | imports |
| `export_not_found` | 404 | exports |
| `export_not_supported` | 422 | exports |
| `export_too_large` | 422 | exports |
| `favorite_already_exists` | 409 | favorites |
| `favorite_limit_reached` | 409 | favorites |
| `favorite_not_found` | 404 | favorites |
| `favorite_target_not_found` | 404 | favorites |
| `field_key_taken` | 409 | fields |
| `field_managed_by_module` | 409 | fields (field managed by a module: it cannot be changed through the UI or the API) |
| `field_not_accessible` | 422 | field_permissions |
| `field_not_found` | 404 | fields |
| `field_option_not_found` | 404 | fields |
| `field_permission_locked` | 422 | field_permissions |
| `field_read_only` | 422 | field_permissions |
| `field_without_options` | 422 | fields |
| `file_not_found` | 404 | files |
| `file_too_large` | 422 | files |
| `file_type_not_allowed` | 422 | files |
| `formula_cycle` | 422 | fields (`formula` field that depends on itself through other formulas; path in `details.fields`) |
| `goal_already_exists` | 409 | goals |
| `goal_not_found` | 404 | goals |
| `idempotency_key_in_progress` | 409 | idempotency |
| `idempotency_key_reused` | 422 | idempotency |
| `import_not_editable` | 409 | imports |
| `import_not_found` | 404 | imports |
| `import_not_retryable` | 409 | imports |
| `import_not_supported` | 422 | imports |
| `incoming_webhook_disabled` | 403 | incoming_webhooks |
| `incoming_webhook_not_found` | 404 | incoming_webhooks |
| `incoming_webhook_owner_unavailable` | 403 | incoming_webhooks |
| `incoming_webhook_permission_denied` | 403 | incoming_webhooks |
| `incoming_webhook_request_not_found` | 404 | incoming_webhooks |
| `incompatible_field_type` | 422 | fields |
| `insufficient_scope` | 403 | auth |
| `integration_activation_failed` | 422 | integrations |
| `integration_not_configurable` | 422 | integrations |
| `integration_not_configured` | 422 | integrations |
| `integration_provider_not_found` | 404 | integrations |
| `integration_test_failed` | 422 | integrations |
| `invalid_api_token` | 401 | api_tokens |
| `invalid_automation` | 422 | automations |
| `invalid_csv` | 422 | imports |
| `invalid_current_password` | 422 | security |
| `invalid_decision` | 422 | config_packages |
| `invalid_member_status` | 409 | organization_admin |
| `invalid_organization_export` | 422 | organization-exports (`aphel org import` and `POST /api/v1/installer/restore`: invalid zip, manifest of another format or version, checksum mismatch) |
| `invalid_package` | 422 | config_packages |
| `invalid_token` | 422 | auth |
| `invalid_two_factor_code` | 422 | auth |
| `invalid_webhook_payload` | 422 | incoming_webhooks |
| `invalid_webhook_signature` | 401 | incoming_webhooks |
| `last_active_activity_type` | 409 | activities |
| `last_admin` | 409 | organization_admin |
| `last_lost_reason` | 409 | pipelines |
| `layout_context_not_found` | 404 | layouts |
| `lead_distribution_no_eligible_member` | 422 | lead_distribution (automation `assign_by_rule` step) |
| `lead_distribution_record_type_mismatch` | 422 | lead_distribution |
| `lead_distribution_rule_inactive` | 422 | lead_distribution (automation `assign_by_rule` step) |
| `lead_distribution_rule_key_taken` | 409 | lead_distribution |
| `lead_distribution_rule_not_found` | 404 | lead_distribution |
| `link_not_found` | 404 | contacts |
| `linked_record_not_found` | 404 | custom_objects |
| `login_locked` | 429 | auth |
| `logo_not_found` | 404 | organizations |
| `lost_reason_not_found` | 404 | pipelines |
| `member_not_found` | 404 | organization_admin |
| `merge_same_record` | 422 | duplicates |
| `migration_source_not_found` | 404 | migration-assistant |
| `no_default_pipeline` | 409 | deals |
| `not_a_member` | 403 | auth |
| `not_invited` | 409 | organization_admin |
| `note_not_found` | 404 | notes |
| `notification_not_found` | 404 | notifications |
| `onboarding_not_pending` | 409 | onboarding |
| `option_value_taken` | 409 | fields |
| `organization_export_in_progress` | 409 | organization-exports |
| `organization_export_not_found` | 404 | organization-exports |
| `organization_export_not_ready` | 409 | organization-exports |
| `organization_not_empty` | 409 | organization-exports (the backup is only restored on an empty installation, before the installer) |
| `organization_not_found` | 404 | modules (organization data unavailable to the module) |
| `permission_denied` | 403 | activities, auth, companies, dashboard, fields, files, notes, organization_admin, pipelines, tags, timeline, views |
| `pipeline_has_deals` | 409 | pipelines |
| `pipeline_limit_reached` | 409 | pipelines |
| `pipeline_not_found` | 404 | pipelines |
| `pipeline_stage_not_found` | 404 | pipelines |
| `plan_outdated` | 409 | config_packages |
| `product_currency_mismatch` | 422 | deal_items |
| `product_not_found` | 404 | products |
| `quote_already_accepted` | 409 | quotes |
| `quote_closed` | 409 | quotes (public link: already accepted or declined) |
| `quote_expired` | 409 | quotes |
| `quote_has_no_items` | 422 | quotes |
| `quote_invalid_transition` | 409 | quotes |
| `quote_items_limit_reached` | 422 | quotes |
| `quote_not_current` | 409 | quotes (an old version is read-only) |
| `quote_not_editable` | 409 | quotes (sent or closed: create a new version) |
| `quote_not_found` | 404 | quotes |
| `quote_not_sent` | 409 | quotes |
| `quote_price_list_not_found` | 422 | quotes |
| `quote_superseded` | 409 | quotes (public link of a superseded version) |
| `record_has_relations` | 409 | custom_objects |
| `record_not_accessible` | 403 | custom_objects |
| `record_operation_not_supported` | 422 | imports, bulk actions and automations (record type without a handler or without the operation) |
| `record_not_found` | 404 | fields (formula preview record does not exist or is outside the read scope); lifecycle (stage history of a record that does not exist or is outside the read scope); record_sharing/followers (record does not exist or is outside the read scope) |
| `record_share_exists` | 409 | record_sharing |
| `record_share_not_allowed` | 403 | record_sharing (only the owner or `records.share`) |
| `record_share_not_found` | 404 | record_sharing |
| `record_shares_limit_reached` | 422 | record_sharing |
| `record_type_not_found` | 404 | fields |
| `record_type_not_taggable` | 422 | tags |
| `relation_limit_reached` | 409 | custom_objects |
| `relation_not_found` | 404 | custom_objects |
| `relation_required` | 409 | custom_objects |
| `relationship_key_taken` | 409 | custom_objects |
| `relationship_not_found` | 404 | custom_objects |
| `reminder_not_found` | 404 | reminders |
| `report_not_found` | 404 | reports |
| `report_period_required` | 422 | reports |
| `report_schedule_not_found` | 404 | reports |
| `role_in_use` | 409 | organization_admin |
| `role_key_taken` | 409 | organization_admin |
| `role_not_found` | 404 | organization_admin |
| `segment_name_taken` | 409 | segments |
| `segment_not_found` | 404 | segments |
| `session_not_found` | 404 | security |
| `sso_required` | 403 | auth (password login refused: the organization requires single sign-on for the email's domain) |
| `stage_has_deals` | 409 | pipelines |
| `stage_requirements_missing` | 422 | deals |
| `system_field_locked` | 422 | fields |
| `system_role_locked` | 409 | organization_admin |
| `tag_name_taken` | 409 | tags |
| `tag_not_allowed` | 422 | tags |
| `tag_not_found` | 404 | tags |
| `team_cycle` | 422 | organization_admin |
| `team_not_found` | 404 | organization_admin |
| `template_not_found` | 404 | config_packages |
| `too_many_api_tokens` | 422 | api_tokens |
| `too_many_columns` | 422 | imports |
| `too_many_custom_objects` | 422 | custom_objects |
| `too_many_fields` | 422 | fields |
| `too_many_incoming_webhooks` | 422 | incoming_webhooks |
| `too_many_relationships` | 422 | custom_objects |
| `too_many_rows` | 422 | imports |
| `too_many_segments` | 422 | segments |
| `too_many_tags` | 422 | tags |
| `too_many_webhook_endpoints` | 422 | webhooks |
| `trash_item_not_found` | 404 | trash |
| `two_factor_already_enabled` | 409 | auth |
| `two_factor_attempts_exceeded` | 401 | auth |
| `two_factor_not_enabled` | 409 | auth |
| `two_factor_setup_not_started` | 409 | auth |
| `two_factor_setup_required` | 403 | auth |
| `unsupported_record_type` | 422 | files |
| `version_conflict` | 409 | geral |
| `view_not_found` | 404 | views |
| `webhook_delivery_not_found` | 404 | webhooks |
| `webhook_endpoint_disabled` | 409 | webhooks |
| `webhook_endpoint_not_found` | 404 | webhooks |
| `webhook_mapping_failed` | 422 | incoming_webhooks |
| `webhook_payload_too_large` | 413 | incoming_webhooks |
| `webhook_replayed` | 409 | incoming_webhooks |
| `webhook_url_not_allowed` | 422 | webhooks |
| `widget_not_found` | 404 | dashboard |
| `job_not_found` | 404 | system (job does not exist or is not failed when retrying/discarding) |
| `metrics_disabled` | 404 | system (`/metrics` without `METRICS_TOKEN` configured) |
| `metrics_unauthorized` | 401 | system (`/metrics` without the right token) |
| `invalid_resolution` | 422 | config_packages (resolution sent for an item without a conflict) |
| `package_not_versioned` | 422 | config_packages (package without `id`/`version` in the versions flow) |
| `invalid_package_id` | 422 | config_packages |
| `catalog_disabled` | 404 | catalog (catalog turned off in Settings → Integrations → Modules → Catalog) |
| `catalog_unavailable` | 422 | catalog (index unreachable, too large or invalid) |
| `catalog_entry_not_found` | 404 | catalog |
| `catalog_entry_not_installable` | 422 | catalog (modules are only listed) |
| `aphel_too_old` | 422 | catalog (minimum Aphel version higher than the installed one) |
| `checksum_mismatch` | 422 | catalog (package SHA-256 differs from the index) |
| `catalog_package_mismatch` | 422 | catalog (package id/version differ from the entry) |
| `demo_restricted` | 403 | demo (action blocked with `DEMO_MODE=true`) |

Whoever creates a new code adds the row to this table and cites it in the response description in `#[utoipa::path]`.
| `mail_not_configured` | 422 | mail (no outgoing email configured in Settings → Outgoing email; invitations and password resets by the administrator return the link to copy) |
| `mail_provider_authentication` | 422 | email (the provider rejected the mailbox credentials) |
| `mail_provider_unavailable` | 422 | email (provider server unreachable or down) |
| `mail_provider_rejected` | 422 | email (the provider rejected the message or the operation) |
| `mail_provider_unsupported` | 422 | email (operation not supported by the provider) |
| `mailbox_not_found` | 404 | email (mailbox does not exist or belongs to another user) |
| `mailbox_already_connected` | 409 | email (the user has already connected this address) |
| `mailbox_address_mismatch` | 422 | email (new settings belong to another mailbox) |
| `mailbox_disconnected` | 409 | email (a disconnected mailbox does not sync) |
| `email_thread_not_found` | 404 | email (conversation does not exist, is private or has no readable record) |
| `email_link_not_found` | 404 | email (the conversation is not linked to the record) |
| `email_attachment_not_found` | 404 | email (attachment does not exist or no full access to the conversation) |
| `email_block_not_found` | 404 | email (blocklist entry) |
| `email_block_exists` | 409 | email (domain or address already blocked) |
| `email_not_found` | 404 | email (message does not exist, belongs to another user, or the replied message is outside the user's mailboxes) |
| `no_mailbox_available` | 422 | email (the user has no connected mailbox to send from) |
| `mailbox_unavailable` | 422 | email (a disconnected mailbox does not send) |
| `email_not_cancellable` | 409 | email (only scheduled emails not yet sent can be canceled) |
| `email_attachments_too_large` | 422 | email (attachments add up to more than 25 MB) |
| `email_template_not_found` | 404 | email (template does not exist or is another user's personal template) |
| `email_template_key_taken` | 409 | email (template key already used in the organization or by the owner) |
| `email_tracking_link_not_found` | 404 | email (invalid tracking token or link that was not in the sent email) |
| `email_tracking_objection_exists` | 409 | email (the address has already objected to tracking) |
| `email_tracking_objection_not_found` | 404 | email (tracking objection does not exist) |
| `sequence_not_found` | 404 | sequences (sequence does not exist) |
| `sequence_key_taken` | 409 | sequences (sequence key already used in the organization) |
| `sequence_inactive` | 422 | sequences (a deactivated sequence does not accept enrollments) |
| `sequence_empty` | 422 | sequences (sequence without steps to run) |
| `sequence_already_enrolled` | 409 | sequences (contact already active or paused in the sequence) |
| `sequence_contact_without_email` | 422 | sequences (contact without an email in a sequence with automatic email) |
| `sequence_contact_objected` | 422 | sequences (the contact's address objected to email tracking) |
| `sequence_enrollment_not_found` | 404 | sequences (enrollment does not exist or is outside the enrolling user's scope) |
| `sequence_enrollment_finished` | 409 | sequences (the enrollment is not in a state that allows pausing, resuming or removing) |
| `meeting_type_not_found` | 404 | scheduling (meeting type does not exist, is inactive on the public page or is out of scope) |
| `meeting_type_slug_taken` | 409 | scheduling (scheduling link address already in use) |
| `booking_not_found` | 404 | scheduling (booking does not exist or invalid management token) |
| `booking_slot_unavailable` | 409 | scheduling (the slot is no longer free) |
| `booking_not_changeable` | 409 | scheduling (booking canceled or already past) |
| `booking_rejected` | 422 | scheduling (submission rejected by spam protection) |
| `call_not_found` | 404 | calls (the activity is not a logged call) |
| `call_recording_not_found` | 404 | calls (call without a recording at the provider or a saved link) |
| `call_activity_type_missing` | 422 | calls (the organization does not have the `call` activity type active) |
| `call_outcome_not_found` | 404 | calls (call outcome does not exist) |
| `call_outcome_key_taken` | 409 | calls (outcome key already used in the organization) |
| `last_active_call_outcome` | 409 | calls (the organization needs at least one active outcome) |
| `telephony_provider_not_found` | 404 | calls (telephony provider does not exist or belongs to a disabled module) |
| `call_session_not_found` | 404 | calls (call session does not exist or belongs to another user) |
| `call_session_view_not_found` | 404 | calls (saved view does not exist, is invisible to the user or is not a contacts view) |
| `call_session_empty` | 422 | calls (the view has no contacts for the queue) |
| `call_session_completed` | 409 | calls (the session has already ended) |
| `call_session_item_not_found` | 404 | calls (there is no contact at that position in the queue) |
| `deal_member_not_found` | 404 | deal-members (member does not exist on the deal team) |
| `deal_member_exists` | 409 | deal-members (the user is already a member of the deal team) |
| `deal_members_limit_reached` | 422 | deal-members (the deal already has 50 members) |
| `deal_credit_exceeds_total` | 422 | deal-members (the members' credit would add up to more than 100%; `details` carries `allocated_percent` and `available_percent`) |
| `deal_member_role_not_found` | 404 | deal-members (member role does not exist) |
| `deal_member_role_key_taken` | 409 | deal-members (role key already used in the organization) |
| `contact_not_lead` | 409 | lifecycle (qualifying requires the contact in `lead`; disqualifying and converting, in `lead` or `qualified`) |
| `disqualification_reason_not_found` | 404 | lifecycle (disqualification reason does not exist) |
| `disqualification_reason_key_taken` | 409 | lifecycle (reason key already used in the organization) |
| `currency_not_enabled` | 422 | currencies, deals (the currency is not enabled in the organization: creating a deal, changing the currency or registering an exchange rate) |
| `exchange_rate_not_found` | 404 | currencies (exchange rate does not exist) |
| `exchange_rate_exists` | 409 | currencies (the currency already has an exchange rate on that date) |
| `exchange_rate_provider_not_configured` | 422 | currencies (updating exchange rates now without a configured provider) |
| `exchange_rate_provider_unavailable` | 503 | currencies (the exchange rate provider did not respond; in a batch update the currency goes to `failed`) |
| `price_list_not_found` | 404 | price-lists, products (price list does not exist) |
| `price_list_key_taken` | 409 | price-lists (price list key already used in the organization) |
| `product_bundle_not_found` | 404 | products (the product is not a bundle) |
| `product_bundle_nested` | 409 | products (the product is a component of another bundle and cannot become a bundle) |
| `validation_rule_failed` | 422 | contacts, companies, deals, activities, products, quotes, objects (the write violates validation rules; `details.fields[]` carries `{ field, message }` for each violated rule: `field` is the highlighted field — `custom_fields.<key>` for a custom one — or `record`, and `message` is the rule's message) |
| `validation_rule_not_found` | 404 | validation-rules (rule does not exist) |
| `stage_checklist_incomplete` | 422 | deals (moving stage or winning with required checklist items of the current stage pending; `details.fields[]` carries `checklist.<key>` with the item label) |
| `company_hierarchy_cycle` | 422 | companies (`parent_company_id` is the company itself or one of its subsidiaries, at any level; `details.fields[]` carries `parent_company_id`) |
| `company_hierarchy_too_deep` | 422 | companies (the group would have more than 10 levels counting the subtree of the moved company; `details.fields[]` carries `parent_company_id`) |
| `checklist_item_not_found` | 404 | deals (checklist item does not exist or belongs to another pipeline) |
| `approval_required` | 409 | deals, objects, quotes and any write that goes through an approval point (the change requires approval; `details` carries `{ record_type, record_id, moment, target_id, policies: [{ id, key, name }] }` — request it with `POST /approval-requests`) |
| `approval_pending` | 409 | contacts, companies, deals, activities, products, quotes, objects (there is a pending approval request: locked field changed or a change awaiting approval; `details` carries `{ request_id, record_type, record_id, moment, target_id }` and, when it is a field lock, `fields`) |
| `approval_policy_not_found` | 404 | approvals (approval policy does not exist) |
| `approval_policy_key_taken` | 409 | approvals (policy key already used in the organization) |
| `approval_request_not_found` | 404 | approvals (approval request does not exist or is invisible to the user) |
| `approval_not_approver` | 403 | approvals (the user is not an approver of the current step or has already decided) |
| `approval_not_pending` | 409 | approvals (the request is no longer pending) |
| `scoring_model_not_found` | 404 | scoring (scoring model does not exist) |
| `approval_no_approver` | 409 | approvals (no eligible approver for the request's step; `details` carries `policy_id`, `policy_key`, `policy_name`, `record_type`, `record_id`, `step` and `step_name`) |
| `stage_requirements_inaccessible` | 422 | deals (the stage requires empty fields that the user cannot fill due to field permissions; `details.fields[]` points to the fields and the message says there are requirements on fields without access) |
| `currency_mismatch` | 422 | quotes (accepting the quote while syncing a deal in another currency) |
| `invalid_spreadsheet` | 422 | imports (the uploaded file is not a valid XLSX/CSV spreadsheet) |
| `migration_history_step` | 422 | migrations_assistant (history step — activities and notes — sent to the import route, or import step sent to the history route) |
| `retention_not_supported` | 422 | privacy (retention policy that archives a record type without archiving support) |
| `module_disabled` | 404 | modules (route of a disabled module) |
| `module_confirmation_required` | 422 | modules |
| `module_dependency_disabled` | 409 | modules |
| `module_enabled` | 409 | modules |
| `module_export_required` | 409 | modules |
| `module_incompatible` | 409 | modules |
| `module_not_found` | 404 | modules |
| `module_required_by` | 409 | modules |
| `module_settings_required` | 422 | modules |
| `module_required` | 409 | modules (disabling a module marked as required in the build) |
| `module_dependency_cycle` | 409 | modules (the build's modules depend on each other in a cycle; `App` assembly error) |
| `invalid_filter` | 422 | ai, insights (saved or generated filter that is not a valid record filter) |
| `invalid_aggregate` | 422 | insights (invalid widget aggregation query) |
| `invalid_job_payload` | 422 | messaging (job payload without the expected id; only appears in the job's `last_error`) |
| `esign_invalid_webhook_payload` | 422 | esign (Clicksign webhook body is not JSON) |
| `esign_webhook_payload_too_large` | 422 | esign (webhook body over 512 KB) |
| `esign_signed_document_unavailable` | 503 | esign (the signed PDF is not yet available at Clicksign; the job retries) |
| `messaging_invalid_payload` | 422 | messaging (WhatsApp webhook body is not JSON) |
| `messaging_payload_too_large` | 422 | messaging (webhook body over 1 MB) |
| `sso_not_configured` | 422 | oidc, google, microsoft (`client_id`/`client_secret` not configured) |
| `invalid_state` | 422 | oidc, google, microsoft (OAuth callback `state` does not exist, expired or was already used) |
| `invalid_grant` | 422 | oidc, google, microsoft (the provider rejected the code or the refresh token) |
| `calendar_not_connected` | 422 | google, microsoft (user's calendar not connected) |
| `calendar_no_refresh_token` | 422 | google, microsoft (the provider did not grant offline access to the calendar) |
| `calendar_authorization_revoked` | 422 | google, microsoft (calendar authorization revoked or expired; connect again) |
| `calendar_cursor_expired` | 422 | google, microsoft (calendar sync cursor expired; the next sync starts over) |
| `calendar_event_not_found` | 404 | google, microsoft (the event no longer exists in the remote calendar) |
| `calendar_meeting_type_not_found` | 422 | google, microsoft (activity type configured for meetings does not exist or is inactive) |
| `calendar_api_failed` | 403/422 | google, microsoft (the calendar API refused access — 403 — or the request — 422) |
| `mail_not_authorized` | 401 | google, microsoft (mailbox without an access or refresh token; connect again) |
| `mail_token_rejected` | 401 | google, microsoft (the provider rejected the access token) |
| `mail_authorization_revoked` | 401 | google, microsoft (mailbox authorization revoked or expired; connect again) |
| `mail_access_denied` | 403 | google, microsoft (the provider denied access to the mailbox; check the granted permissions) |
| `mail_not_found` | 404 | google, microsoft (message or item no longer exists at the provider) |
| `mail_cursor_expired` | 422 | google, microsoft (mailbox sync cursor expired) |
| `mail_invalid_message` | 422 | google, microsoft (invalid MIME message to read or build) |
| `mail_no_refresh_token` | 422 | google, microsoft (the provider did not grant offline access to the mailbox) |
| `mail_api_failed` | 422 | google, microsoft (the email API rejected the request) |
| `ai_feature_disabled` | 403 | ai (feature turned off in the settings) |
| `ai_invalid_calls_query` | 422 | ai (invalid filter or pagination in `/calls`) |
| `ai_invalid_instruction` | 422 | ai (invalid `/compose` body (`details.fields`)) |
| `ai_invalid_response` | 503 | ai (next actions outside the expected JSON) |
| `ai_limit_reached` | 429 | ai (installation's daily limit reached) |
| `ai_not_configured` | 503 | ai (no model, invalid URL or no key when the provider requires one) |
| `ai_provider_rate_limited` | 429 | ai (the provider responded 429) |
| `ai_query_invalid` | 422 | ai (invalid `/query` body or invalid filter after the correction) |
| `ai_query_no_fields` | 422 | ai (no filterable field available to the user) |
| `ai_query_unanswerable` | 422 | ai (the question cannot be turned into a filter (message with the AI's reason)) |
| `ai_record_not_found` | 404 | ai (record does not exist or is invisible to the user) |
| `ai_refused` | 422 | ai (the model refused the request) |
| `ai_unsupported_record_type` | 422 | ai (summary/actions/writing outside `contact`, `company`, `deal`) |
| `ai_user_limit_reached` | 429 | ai (per-user hourly limit reached) |
| `enrichment_automatic_disabled` | 422 | enrichment (automation action with `automatic` turned off) |
| `enrichment_cnpj_missing` | 422 | enrichment (no CNPJ typed nor in the configured field) |
| `enrichment_cnpj_not_found` | 404 | enrichment (the source does not know the CNPJ) |
| `enrichment_invalid_cnpj` | 422 | enrichment (invalid check digits (also accepts the alphanumeric CNPJ)) |
| `enrichment_invalid_request` | 422 | enrichment (invalid ids or too many items) |
| `enrichment_lookup_expired` | 422 | enrichment (lookup older than 60 min) |
| `enrichment_lookup_not_found` | 404 | enrichment (lookup does not exist) |
| `enrichment_nothing_to_apply` | 422 | enrichment (no item with a mapped field and a different value) |
| `enrichment_source_rate_limited` | 429 | enrichment (the source rate-limited (ReceitaWS accepts 3 lookups/min; Aphel respects that limit in-process)) |
| `enrichment_source_unavailable` | 503 | enrichment (source down or slow (15 s timeout)) |
| `esign_file_not_attached` | 422 | esign (the file is not attached to the deal) |
| `esign_file_not_pdf` | 422 | esign (the file is not a PDF) |
| `esign_file_too_large` | 422 | esign (PDF over 20 MB) |
| `esign_invalid_request` | 422 | esign (invalid data (signers: 1 to 20, name, valid email and no duplicates; message up to 1000 characters)) |
| `esign_not_configured` | 503 | esign (access token not configured) |
| `esign_owner_unavailable` | 503 | esign (no active member can act for the request) |
| `esign_provider_rejected` | 422 | esign (Clicksign rejected it (provider detail in the message)) |
| `esign_provider_unauthorized` | 503 | esign (Clicksign rejected the token) |
| `esign_provider_unavailable` | 503 | esign (Clicksign down, slow (30 s timeout) or rate-limiting) |
| `esign_request_closed` | 409 | esign (canceling a request that is already closed) |
| `esign_request_not_found` | 404 | esign (request does not exist) |
| `esign_webhook_not_configured` | 503 | esign (HMAC secret not configured) |
| `lead_form_invalid` | 422 | lead_forms (invalid configuration (`details.fields` with `name`, `fields.<i>.<field>`, `deal.*`, `acting_user_id`, `tag_id`, `redirect_url`, `allowed_origins.<i>`...)) |
| `lead_form_invalid_payload` | 422 | lead_forms (body that is neither a JSON object nor a urlencoded form) |
| `lead_form_invalid_query` | 422 | lead_forms (invalid filter or pagination in the history) |
| `lead_form_invalid_submission` | 422 | lead_forms (invalid answers (`details.fields[].field` = field identifier; messages in pt-BR to show to the visitor)) |
| `lead_form_limit_reached` | 409 | lead_forms (200 forms in the installation) |
| `lead_form_not_found` | 404 | lead_forms (form does not exist, is inactive or the module is disabled) |
| `lead_form_origin_not_allowed` | 403 | lead_forms (`Origin` outside the authorized sites) |
| `lead_form_owner_unavailable` | 503 | lead_forms (the acting user lost access or permission) |
| `lead_form_payload_too_large` | 422 | lead_forms (body over 64 KB) |
| `lead_form_submission_failed` | 503 | lead_forms (the CRM rejected the contact for another reason) |
| `messaging_channel_in_use` | 409 | messaging (deleting a channel with conversations) |
| `messaging_channel_inactive` | 409 | messaging (replying through an inactive channel) |
| `messaging_channel_invalid` | 422 | messaging (invalid channel, with `details.fields` (`config.*`, `secrets.*`, `name`, `kind`, `acting_user_id`)) |
| `messaging_channel_limit_reached` | 409 | messaging (more than 50 channels) |
| `messaging_channel_not_found` | 404 | messaging (channel does not exist (or webhook of a disabled module)) |
| `messaging_conversation_not_found` | 404 | messaging (conversation does not exist) |
| `messaging_invalid_assignee` | 422 | messaging (assignee without access to the inbox) |
| `messaging_invalid_query` | 422 | messaging (invalid filter) |
| `messaging_invalid_reply` | 422 | messaging (empty or too long reply (WhatsApp 4096, email 100 thousand characters)) |
| `messaging_record_not_accessible` | 422 | messaging (contact/deal does not exist or is not accessible) |
| `proposal_invalid_request` | 422 | proposals (malformed id or preview without `body` or `template_id`) |
| `proposal_template_inactive` | 422 | proposals (the chosen template is inactive) |
| `proposal_template_invalid` | 422 | proposals (invalid fields or problems in the body (with line and column)) |
| `proposal_template_key_taken` | 409 | proposals (key already used by another template) |
| `proposal_template_not_found` | 404 | proposals (template does not exist (or is inactive, for those who only have `generate`)) |
| `proposal_template_required` | 422 | proposals (no `template_id` and there is no active template) |
| `whatsapp_invalid_signature` | 401 | messaging (webhook without a valid signature) |
| `whatsapp_invalid_verify_token` | 403 | messaging (webhook verification with the wrong token) |
| `whatsapp_window_closed` | 422 | messaging (outside WhatsApp's 24 h window (also as the message's `error_code`)) |
| `invalid_subject_term` | 422 | privacy |
| `privacy_export_unavailable` | 404 | privacy |
| `privacy_request_not_found` | 404 | privacy |
| `privacy_request_not_pending` | 409 | privacy |
| `privacy_targets_empty` | 422 | privacy |
| `retention_policy_limit` | 422 | privacy |
| `retention_policy_not_found` | 404 | privacy |
| `retention_preview_required` | 409 | privacy |
| `mail_attachments_unsupported` | 422 | mail (the configured email transport does not send attachments) |

### Durable module delivery

| Code | HTTP | Meaning |
| --- | --- | --- |
| `email_delivery_unknown` | 409 | A previous queued send has an uncertain outcome and will not be repeated automatically |
| `esign_delivery_unknown` | 409 | Reconcile the previous signature provider call before submitting another request |
| `esign_delivery_claimed` | 409 | The signature request is no longer pending delivery |
| `plugin_query_outcome_unknown` | 409 | An external query started without a completed result; automatic repetition is refused |
| `plugin_query_authority_changed` | 403 | The authority fingerprint changed since the external query was claimed |

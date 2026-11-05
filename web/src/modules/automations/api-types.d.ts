export interface paths {
    "/api/v1/automations": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["automations_list"];
        put?: never;
        /** @description Creates a draft automation over a record type (`contact`, `company`, `deal`, `activity` or `custom.<key>`). `trigger` has a `type` from the catalog plus its options; `definition` is the step graph: `start` names the first step and each step is a `condition` (filter AST of the listings, with `trigger.*` paths evaluated over the event: `trigger.previous.<field>`, `trigger.changed_fields`, `trigger.payload.*`; when false the run ends), a `branch` (first matching `cases[]` or `default`) or an `action` (catalog `action` + `config`, whose strings accept templates such as `{{record.title}}`, `{{record.custom.cpf}}`, `{{trigger.previous.stage_id}}`, `{{now + 2d}}` and `{{today + 1w}}`). The graph must be acyclic, without orphan steps, and fields, templates, tags, stages and users are validated now (paths under `trigger.` and `definition.`). Write-only secrets of actions (such as `signing_secret` of `call_webhook`) go to the secret vault and come back only as `<field>_configured`. `key` defaults to a slug of the name. */
        post: operations["automations_create"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/automations/catalog": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["automations_catalog"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/automations/{automation_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["automations_get"];
        put?: never;
        post?: never;
        delete: operations["automations_delete"];
        options?: never;
        head?: never;
        patch: operations["automations_update"];
        trace?: never;
    };
    "/api/v1/automations/{automation_id}/activate": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["automations_activate"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/automations/{automation_id}/dry-run": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Tests the published version (or the draft with `use_draft`, or the draft when nothing was published) against an existing record with the permissions of the caller: conditions and branches are evaluated for real and each action reports what it would do (`status: planned`, rendered input with sensitive values masked and details such as the resolved assignee and due date), without changing anything. `changed_fields` and `previous` simulate the event for `trigger.*` conditions. */
        post: operations["automations_dry_run"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/automations/{automation_id}/pause": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["automations_pause"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/automations/{automation_id}/publish": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Validates the draft again (it needs at least one action) and checks that the caller has every permission its actions require; then snapshots it as the next version and activates the automation. Runs use the published version and act with the permissions of the user who published it, reloaded at each run. */
        post: operations["automations_publish"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/automations/{automation_id}/run": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Runs an active automation with the `manual` trigger on a record readable by the caller. The run is queued for the worker (`automation.run`) and acts with the permissions of the publisher of the active version. */
        post: operations["automations_run"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/automations/{automation_id}/runs": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["automations_runs"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/automations/{automation_id}/runs/{run_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["automations_run_details"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/automations/{automation_id}/versions": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["automations_versions"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
}
export type webhooks = Record<string, never>;
export interface components {
    schemas: {
        DryRunResponse: {
            error?: string | null;
            /** @example completed */
            outcome: string;
            record_available: boolean;
            record_id: string;
            record_type: string;
            steps: components["schemas"]["RunStepResponse"][];
            /** Format: int32 */
            version?: number | null;
        };
        RunStepResponse: {
            action?: string | null;
            condition_result?: boolean | null;
            /** Format: int32 */
            duration_ms: number;
            error?: string | null;
            input?: Record<string, never> | null;
            kind: components["schemas"]["AutomationStepType"];
            next_step_id?: string | null;
            output?: Record<string, never> | null;
            /** Format: int32 */
            position: number;
            started_at: string;
            status: components["schemas"]["AutomationStepStatus"];
            step_id: string;
        };
        /** @enum {string} */
        AutomationStepStatus: "succeeded" | "failed" | "skipped" | "planned" | "waiting";
        /** @enum {string} */
        AutomationStepType: "condition" | "branch" | "action" | "wait" | "wait_until";
        AutomationRunResponse: {
            /** Format: int32 */
            attempts: number;
            automation_id: string;
            chain: string[];
            /** Format: int32 */
            depth: number;
            error?: string | null;
            finished_at?: string | null;
            id: string;
            reason?: null | components["schemas"]["AutomationSkipReason"];
            record_id: string;
            record_type: string;
            resume_at?: string | null;
            started_at: string;
            status: components["schemas"]["AutomationRunStatus"];
            trigger_event_id?: string | null;
            trigger_type: string;
            /** Format: int32 */
            version: number;
            waiting_step_id?: string | null;
        };
        /** @enum {string} */
        AutomationRunStatus: "running" | "waiting" | "succeeded" | "failed" | "skipped" | "cancelled";
        /** @enum {string} */
        AutomationSkipReason: "max_depth" | "causation_cycle" | "rate_limited" | "record_not_found" | "record_not_accessible" | "publisher_unavailable" | "automation_inactive" | "record_deleted" | "automation_paused" | "version_published";
        AutomationStatsResponse: {
            /** Format: int64 */
            failed_30d: number;
            last_run_at?: string | null;
            /** Format: int64 */
            runs_30d: number;
            /** Format: int64 */
            waiting: number;
        };
        AutomationBranchCase: {
            filter: components["schemas"]["FilterNode"];
            key: string;
            label?: string | null;
            next?: string | null;
        };
        FilterNode: components["schemas"]["FilterGroup"] | components["schemas"]["FilterCondition"] | components["schemas"]["FilterRelated"];
        FilterRelated: {
            related: components["schemas"]["RelatedFilter"];
        };
        RelatedFilter: {
            filter: components["schemas"]["FilterNode"];
            none?: boolean;
            /** @example organization_profile.organization */
            relationship: string;
        };
        FilterCondition: {
            /** @example name */
            field: string;
            operator: components["schemas"]["FilterOperator"];
            value?: unknown;
        };
        /** @enum {string} */
        FilterOperator: "eq" | "neq" | "contains" | "not_contains" | "starts_with" | "gt" | "gte" | "lt" | "lte" | "between" | "in" | "not_in" | "is_empty" | "is_not_empty" | "today" | "yesterday" | "tomorrow" | "this_week" | "last_week" | "next_week" | "this_month" | "last_month" | "next_month" | "this_year" | "within_last_days" | "within_next_days" | "older_than_days" | "anniversary" | "in_saved_filter" | "in_group";
        FilterGroup: {
            conditions: components["schemas"]["FilterNode"][];
            op: components["schemas"]["LogicalOperator"];
        };
        /** @enum {string} */
        LogicalOperator: "and" | "or";
        Data_Vec_AutomationVersionResponse: {
            data: {
                definition: components["schemas"]["AutomationDefinition"];
                id: string;
                name: string;
                published_at: string;
                published_by?: string | null;
                record_type: string;
                trigger: components["schemas"]["AutomationTrigger"];
                /** Format: int32 */
                version: number;
            }[];
        };
        AutomationTrigger: {
            /** @example 09:00 */
            at?: string | null;
            /** @example 0 0 9 * * MON */
            cron?: string | null;
            /**
             * Format: int32
             * @example 7
             */
            days?: number | null;
            /** @example expected_close_date */
            field?: string | null;
            fields?: string[] | null;
            filter?: null | components["schemas"]["FilterNode"];
            from_stage_id?: string | null;
            incoming_webhook_id?: string | null;
            mode?: null | components["schemas"]["AutomationStaleMode"];
            /** @example -3d */
            offset?: string | null;
            segment_id?: string | null;
            /** @example 0192f0c4-7d0e-7a8a-9f3e-3c2b1a0d9e8f */
            to_stage_id?: string | null;
            /** @example deal.stage_changed */
            type: string;
        };
        /** @enum {string} */
        AutomationStaleMode: "no_update" | "no_activity" | "same_stage";
        AutomationDefinition: {
            options?: null | components["schemas"]["AutomationOptions"];
            /** @example check_open */
            start?: string | null;
            steps: components["schemas"]["AutomationStep"][];
        };
        AutomationStep: {
            /** @example create_task */
            action?: string | null;
            cases?: components["schemas"]["AutomationBranchCase"][] | null;
            config?: Record<string, never> | null;
            default?: string | null;
            /** @example 2d */
            duration?: string | null;
            filter?: null | components["schemas"]["FilterNode"];
            /** @example check_open */
            id: string;
            label?: string | null;
            next?: string | null;
            /** @example -1d */
            offset?: string | null;
            recheck?: null | components["schemas"]["FilterNode"];
            /** @example 7d */
            timeout?: string | null;
            timeout_next?: string | null;
            type: components["schemas"]["AutomationStepType"];
            /** @example expected_close_date */
            until_field?: string | null;
        };
        AutomationOptions: {
            /** @default true */
            cancel_waiting_on_pause: boolean | null;
            /** @default false */
            cancel_waiting_on_publish: boolean | null;
        };
        Data_AutomationRunDetailsResponse: {
            data: components["schemas"]["AutomationRunResponse"] & {
                steps: components["schemas"]["RunStepResponse"][];
            };
        };
        AutomationRunDetailsResponse: components["schemas"]["AutomationRunResponse"] & {
            steps: components["schemas"]["RunStepResponse"][];
        };
        Page_AutomationRunResponse: {
            data: {
                /** Format: int32 */
                attempts: number;
                automation_id: string;
                chain: string[];
                /** Format: int32 */
                depth: number;
                error?: string | null;
                finished_at?: string | null;
                id: string;
                reason?: null | components["schemas"]["AutomationSkipReason"];
                record_id: string;
                record_type: string;
                resume_at?: string | null;
                started_at: string;
                status: components["schemas"]["AutomationRunStatus"];
                trigger_event_id?: string | null;
                trigger_type: string;
                /** Format: int32 */
                version: number;
                waiting_step_id?: string | null;
            }[];
            meta: components["schemas"]["PageMeta"];
        };
        /**
         * @example {
         *       "page": 1,
         *       "per_page": 25,
         *       "total": 120
         *     }
         */
        PageMeta: {
            /** Format: int32 */
            page: number;
            /** Format: int32 */
            per_page: number;
            /** Format: int64 */
            total: number;
        };
        Data_AutomationRunResponse: {
            data: {
                /** Format: int32 */
                attempts: number;
                automation_id: string;
                chain: string[];
                /** Format: int32 */
                depth: number;
                error?: string | null;
                finished_at?: string | null;
                id: string;
                reason?: null | components["schemas"]["AutomationSkipReason"];
                record_id: string;
                record_type: string;
                resume_at?: string | null;
                started_at: string;
                status: components["schemas"]["AutomationRunStatus"];
                trigger_event_id?: string | null;
                trigger_type: string;
                /** Format: int32 */
                version: number;
                waiting_step_id?: string | null;
            };
        };
        RunAutomationRequest: {
            record_id: string;
        };
        Data_DryRunResponse: {
            data: {
                error?: string | null;
                /** @example completed */
                outcome: string;
                record_available: boolean;
                record_id: string;
                record_type: string;
                steps: components["schemas"]["RunStepResponse"][];
                /** Format: int32 */
                version?: number | null;
            };
        };
        DryRunRequest: {
            changed_fields?: string[];
            previous?: Record<string, never>;
            record_id: string;
            use_draft?: boolean;
        };
        UpdateAutomationRequest: {
            definition?: null | components["schemas"]["AutomationDefinition"];
            description?: string | null;
            name?: string | null;
            trigger?: null | components["schemas"]["AutomationTrigger"];
        };
        Data_AutomationCatalogResponse: {
            data: {
                actions: components["schemas"]["ActionCatalogEntry"][];
                limits: components["schemas"]["AutomationLimitsResponse"];
                template_roots: string[];
                trigger_paths: string[];
                triggers: components["schemas"]["TriggerCatalogEntry"][];
            };
        };
        TriggerCatalogEntry: {
            custom_objects: boolean;
            description: string;
            key: string;
            label: string;
            manual: boolean;
            options_schema: Record<string, never>;
            record_types: string[];
            time_based: boolean;
        };
        AutomationLimitsResponse: {
            /** Format: int32 */
            max_chain_depth: number;
            max_steps: number;
            /** Format: int64 */
            min_schedule_interval_minutes: number;
            /** Format: int64 */
            runs_per_record_limit: number;
            /** Format: int64 */
            runs_per_record_window_minutes: number;
            /** Format: int64 */
            wait_until_poll_minutes: number;
        };
        ActionCatalogEntry: {
            config_schema: Record<string, never>;
            custom_objects: boolean;
            description: string;
            key: string;
            label: string;
            record_types: string[];
            secret_fields: string[];
        };
        AutomationCatalogResponse: {
            actions: components["schemas"]["ActionCatalogEntry"][];
            limits: components["schemas"]["AutomationLimitsResponse"];
            template_roots: string[];
            trigger_paths: string[];
            triggers: components["schemas"]["TriggerCatalogEntry"][];
        };
        Data_AutomationResponse: {
            data: {
                created_at: string;
                created_by?: string | null;
                definition: components["schemas"]["AutomationDefinition"];
                description?: string | null;
                id: string;
                key: string;
                name: string;
                record_type: string;
                status: components["schemas"]["AutomationStatus"];
                trigger: components["schemas"]["AutomationTrigger"];
                updated_at: string;
                updated_by?: string | null;
                /** Format: int32 */
                version: number;
            };
        };
        /** @enum {string} */
        AutomationStatus: "draft" | "active" | "paused";
        AutomationResponse: {
            created_at: string;
            created_by?: string | null;
            definition: components["schemas"]["AutomationDefinition"];
            description?: string | null;
            id: string;
            key: string;
            name: string;
            record_type: string;
            status: components["schemas"]["AutomationStatus"];
            trigger: components["schemas"]["AutomationTrigger"];
            updated_at: string;
            updated_by?: string | null;
            /** Format: int32 */
            version: number;
        };
        CreateAutomationRequest: {
            definition?: null | components["schemas"]["AutomationDefinition"];
            description?: string | null;
            /** @example follow_up_proposal */
            key?: string | null;
            /** @example Follow up on sent proposal */
            name: string;
            /** @example deals.deal */
            record_type: string;
            trigger: components["schemas"]["AutomationTrigger"];
        };
        ErrorEnvelope: {
            error: components["schemas"]["ErrorBody"];
        };
        ErrorBody: {
            /** @example validation_error */
            code: string;
            details?: Record<string, never> | null;
            message: string;
        };
        Page_AutomationListItemResponse: {
            data: (components["schemas"]["AutomationResponse"] & {
                stats: components["schemas"]["AutomationStatsResponse"];
            })[];
            meta: components["schemas"]["PageMeta"];
        };
        AutomationListItemResponse: components["schemas"]["AutomationResponse"] & {
            stats: components["schemas"]["AutomationStatsResponse"];
        };
    };
    responses: never;
    parameters: never;
    requestBodies: never;
    headers: never;
    pathItems: never;
}
export type $defs = Record<string, never>;
export interface operations {
    automations_list: {
        parameters: {
            query?: {
                status?: components["schemas"]["AutomationStatus"];
                record_type?: string;
                search?: string;
                page?: number;
                per_page?: number;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Automations of the organization ordered by name, optionally filtered by `status`, `record_type` and `search` (name or key); each item carries `stats` (last run, runs and failures in the last 30 days, runs waiting) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page_AutomationListItemResponse"];
                };
            };
            /** @description Missing or invalid credentials (`unauthorized`, `invalid_api_token`) */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Missing `automations.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid filter or pagination (`validation_error`) */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description API token rate limit exceeded (`rate_limited`); see `Retry-After` and `X-RateLimit-*` */
            429: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    automations_create: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateAutomationRequest"];
            };
        };
        responses: {
            /** @description Draft created (`automation.created`) */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_AutomationResponse"];
                };
            };
            /** @description Missing or invalid credentials (`unauthorized`, `invalid_api_token`) */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Missing `automations.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Key already used (`automation_key_taken`) or limit of automations reached (`automation_limit_reached`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid name, trigger or definition (`invalid_automation`, `validation_error`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description API token rate limit exceeded (`rate_limited`); see `Retry-After` and `X-RateLimit-*` */
            429: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    automations_catalog: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Triggers and actions available for automations, with the record types each one supports, the JSON schema of their options, the template roots, the `trigger.*` paths and the loop protection limits */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_AutomationCatalogResponse"];
                };
            };
            /** @description Missing or invalid credentials (`unauthorized`, `invalid_api_token`) */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Missing `automations.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description API token rate limit exceeded (`rate_limited`); see `Retry-After` and `X-RateLimit-*` */
            429: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    automations_get: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Automation id */
                automation_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description The automation with its draft trigger and definition; `version` is the published version that runs (0 = never published) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_AutomationResponse"];
                };
            };
            /** @description Missing or invalid credentials (`unauthorized`, `invalid_api_token`) */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Missing `automations.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown automation (`automation_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description API token rate limit exceeded (`rate_limited`); see `Retry-After` and `X-RateLimit-*` */
            429: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    automations_delete: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Automation id */
                automation_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Automation deleted with its versions, runs and secrets (`automation.deleted`) */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Missing or invalid credentials (`unauthorized`, `invalid_api_token`) */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Missing `automations.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown automation (`automation_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description API token rate limit exceeded (`rate_limited`); see `Retry-After` and `X-RateLimit-*` */
            429: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    automations_update: {
        parameters: {
            query?: never;
            header?: {
                /** @description `updated_at` of the automation as last read; when it changed since, answers 409 `version_conflict` */
                "If-Match"?: string | null;
            };
            path: {
                /** @description Automation id */
                automation_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateAutomationRequest"];
            };
        };
        responses: {
            /** @description Draft updated (`automation.updated`); omitted properties stay unchanged. The record type cannot change. The published version keeps running until the next publish */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_AutomationResponse"];
                };
            };
            /** @description Missing or invalid credentials (`unauthorized`, `invalid_api_token`) */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Missing `automations.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown automation (`automation_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The automation changed since `If-Match` (`version_conflict`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid name, trigger or definition (`invalid_automation`) */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description API token rate limit exceeded (`rate_limited`); see `Retry-After` and `X-RateLimit-*` */
            429: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    automations_activate: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Automation id */
                automation_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Automation active again with its last published version (`automation.activated`) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_AutomationResponse"];
                };
            };
            /** @description Missing or invalid credentials (`unauthorized`, `invalid_api_token`) */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Missing `automations.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown automation (`automation_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The automation was never published (`automation_not_published`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description API token rate limit exceeded (`rate_limited`); see `Retry-After` and `X-RateLimit-*` */
            429: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    automations_dry_run: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Automation id */
                automation_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["DryRunRequest"];
            };
        };
        responses: {
            /** @description Steps that would run */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_DryRunResponse"];
                };
            };
            /** @description Missing or invalid credentials (`unauthorized`, `invalid_api_token`) */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Missing `automations.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown automation (`automation_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid record id (`validation_error`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description API token rate limit exceeded (`rate_limited`); see `Retry-After` and `X-RateLimit-*` */
            429: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    automations_pause: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Automation id */
                automation_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Automation paused (`automation.paused`); runs already queued end as `skipped` with `automation_inactive` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_AutomationResponse"];
                };
            };
            /** @description Missing or invalid credentials (`unauthorized`, `invalid_api_token`) */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Missing `automations.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown automation (`automation_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The automation was never published (`automation_not_published`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description API token rate limit exceeded (`rate_limited`); see `Retry-After` and `X-RateLimit-*` */
            429: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    automations_publish: {
        parameters: {
            query?: never;
            header?: {
                /** @description `updated_at` of the automation as last read */
                "If-Match"?: string | null;
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Automation id */
                automation_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Version published and automation active (`automation.published`) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_AutomationResponse"];
                };
            };
            /** @description Missing or invalid credentials (`unauthorized`, `invalid_api_token`) */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Missing `automations.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown automation (`automation_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The automation changed since `If-Match` (`version_conflict`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The draft is invalid or uses actions the caller cannot perform (`invalid_automation`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description API token rate limit exceeded (`rate_limited`); see `Retry-After` and `X-RateLimit-*` */
            429: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    automations_run: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Automation id */
                automation_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["RunAutomationRequest"];
            };
        };
        responses: {
            /** @description Run queued */
            202: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_AutomationRunResponse"];
                };
            };
            /** @description Missing or invalid credentials (`unauthorized`, `invalid_api_token`) */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Missing `automations.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown automation (`automation_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Not published (`automation_not_published`), paused (`automation_not_active`) or without the manual trigger (`automation_not_manual`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid or unknown record (`validation_error`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description API token rate limit exceeded (`rate_limited`); see `Retry-After` and `X-RateLimit-*` */
            429: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    automations_runs: {
        parameters: {
            query?: {
                status?: components["schemas"]["AutomationRunStatus"];
                record_id?: string;
                page?: number;
                per_page?: number;
            };
            header?: never;
            path: {
                /** @description Automation id */
                automation_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Runs of the automation, newest first, optionally filtered by `status` (`waiting` runs carry `resume_at` and `waiting_step_id`; `cancelled` runs carry the `reason`: `record_deleted`, `automation_paused` or `version_published`) and `record_id` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page_AutomationRunResponse"];
                };
            };
            /** @description Missing or invalid credentials (`unauthorized`, `invalid_api_token`) */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Missing `automations.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown automation (`automation_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid filter or pagination (`validation_error`) */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description API token rate limit exceeded (`rate_limited`); see `Retry-After` and `X-RateLimit-*` */
            429: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    automations_run_details: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Automation id */
                automation_id: string;
                /** @description Run id */
                run_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description The run with its steps: evaluated conditions (`condition_result`, chosen `next_step_id`), actions with their rendered input (values of sensitive fields masked, secrets never shown), output, error and duration */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_AutomationRunDetailsResponse"];
                };
            };
            /** @description Missing or invalid credentials (`unauthorized`, `invalid_api_token`) */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Missing `automations.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown automation or run (`automation_not_found`, `automation_run_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description API token rate limit exceeded (`rate_limited`); see `Retry-After` and `X-RateLimit-*` */
            429: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    automations_versions: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Automation id */
                automation_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Published versions, newest first */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_AutomationVersionResponse"];
                };
            };
            /** @description Missing or invalid credentials (`unauthorized`, `invalid_api_token`) */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Missing `automations.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown automation (`automation_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description API token rate limit exceeded (`rate_limited`); see `Retry-After` and `X-RateLimit-*` */
            429: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
}

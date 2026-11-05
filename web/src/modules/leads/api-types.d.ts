export interface paths {
    "/api/v1/lead-disqualification-reasons": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["lead_disqualification_reasons_list"];
        put?: never;
        post: operations["lead_disqualification_reasons_create"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/lead-disqualification-reasons/order": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put: operations["lead_disqualification_reasons_reorder"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/lead-disqualification-reasons/{reason_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        delete: operations["lead_disqualification_reasons_delete"];
        options?: never;
        head?: never;
        patch: operations["lead_disqualification_reasons_update"];
        trace?: never;
    };
    "/api/v1/leads/{contact_id}/convert": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Converts a contact in `lead` or `qualified`: creates a deal through the deals use case (permission `deals.create`; `deal.created`) in `pipeline_id` (and `stage_id`, default the first open stage), with the contact, its primary company and its source; the title defaults to the contact name. Then moves the contact to the target stage of the `deal_created` rule (`opportunity` by default) and publishes `contact.updated` and `lead.converted` with `deal_id`. Needs `people.update` on the contact. */
        post: operations["leads_convert"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/leads/{contact_id}/disqualify": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Disqualifies a contact in `lead` or `qualified` with an active reason and an optional note (kept in the disqualification and the audit, not in the event). The stage becomes the `disqualified_stage` of the lifecycle settings (none by default). Publishes `contact.updated` (when the stage changes) and `lead.disqualified`. Needs `people.update` on the contact. */
        post: operations["leads_disqualify"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/leads/{contact_id}/qualify": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Moves a contact in `lead` to `qualified` and publishes `contact.updated` (with `changes`) and `lead.qualified`, in the same transaction as the stage history. Needs `people.update` on the contact. */
        post: operations["leads_qualify"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/lifecycle/history/{record_type}/{record_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Lifecycle stage history of a contact or a company, newest first (at most 200 entries). Each change of `lifecycle_stage` (creation included) closes the open entry (`left_at`) and opens a new one in the same transaction as the record write. Needs read access to the record. */
        get: operations["lifecycle_history"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/lifecycle/report": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Lifecycle report built from the stage history of the records visible to the user (read scope of `people.read` or `organizations.read`). Stages follow the order of the active options of the field. For each stage: `current` records in it now (not archived), `entered` records that entered it in the period (UTC days, `from`–`to` inclusive, default the last 90 days, at most 731), `converted` of those that later reached any later stage (skipping counts), `conversion_rate` = converted / entered, and `average_days` spent in the stage by the entries of the period that already left it (`completed`). For people, also the disqualifications of the period by reason. */
        get: operations["lifecycle_report"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/lifecycle/settings": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Automatic lifecycle transitions and the stage given to disqualified leads, with the lifecycle stages of the contact field. While never saved, returns the defaults (`configured: false`): `deal_created` → `opportunity` (from lead, qualified, former customer or no stage; also on reopening), `deal_won` → `customer` (from any stage but customer), `all_deals_lost` → `qualified` (only from opportunity, when the contact or company has lost deals and no open or won deal; also evaluated when a deal is deleted). A rule never applies when the record is outside `from_stages` (and `from_empty` for records without stage), so it never demotes a record by itself. Needs `lifecycle.manage`. */
        get: operations["lifecycle_settings_get"];
        /** @description Saves the lifecycle settings and publishes `lifecycle.settings_updated`. Each item of `rules` changes one trigger (`deal_created`, `deal_won`, `all_deals_lost`); omitted triggers and properties keep their value. `target_stage` must be an active option of the `lifecycle_stage` field of people and `from_stages` options of the field. `disqualified_stage` is the stage given to a disqualified lead (`null` clears the stage). The transitions run in the worker (`lifecycle.transitions` consumer) and update the contact and the company of the deal through their use cases (`contact.updated`/`company.updated` with `changes`). Needs `lifecycle.manage`. */
        put: operations["lifecycle_settings_update"];
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
        LeadActionResponse: {
            contact_id: string;
            deal_id?: string | null;
            lifecycle_stage?: string | null;
        };
        LifecycleReportResponse: {
            disqualifications: components["schemas"]["LifecycleDisqualificationCountResponse"][];
            /** @example 2026-01-01 */
            from: string;
            /** @example people.person */
            record_type: string;
            stages: components["schemas"]["LifecycleStageReportResponse"][];
            /** @example 2026-03-31 */
            to: string;
        };
        LifecycleStageReportResponse: {
            /** Format: double */
            average_days?: number | null;
            color?: string | null;
            /** Format: int64 */
            completed: number;
            /** Format: double */
            conversion_rate?: number | null;
            /** Format: int64 */
            converted: number;
            /** Format: int64 */
            current: number;
            /** Format: int64 */
            entered: number;
            is_active: boolean;
            label: string;
            next_stage?: string | null;
            stage: string;
        };
        LifecycleDisqualificationCountResponse: {
            /** Format: int64 */
            count: number;
            reason_id?: string | null;
            reason_name: string;
        };
        LifecycleHistoryEntryResponse: {
            actor: components["schemas"]["LifecycleHistoryActorResponse"];
            /** Format: double */
            days_in_stage?: number | null;
            entered_at: string;
            from_stage?: string | null;
            id: string;
            left_at?: string | null;
            to_stage: string;
        };
        LifecycleHistoryActorResponse: {
            id?: string | null;
            name?: string | null;
            /** @example user */
            type: string;
        };
        DisqualificationReasonResponse: {
            created_at: string;
            id: string;
            is_active: boolean;
            /** @example no_budget */
            key: string;
            /** @example No budget */
            name: string;
            /** Format: int32 */
            position: number;
            updated_at: string;
        };
        LifecycleSettingsResponse: {
            configured: boolean;
            disqualified_stage?: string | null;
            rules: components["schemas"]["LifecycleTransitionRuleResponse"][];
            stages: components["schemas"]["LifecycleStageOptionResponse"][];
            updated_at?: string | null;
        };
        LifecycleStageOptionResponse: {
            color?: string | null;
            is_active: boolean;
            /** @example Qualified */
            label: string;
            /** Format: int32 */
            position: number;
            /** @example qualified */
            value: string;
        };
        LifecycleTransitionRuleResponse: {
            enabled: boolean;
            from_empty: boolean;
            from_stages: string[];
            /** @example customer */
            target_stage: string;
            /** @example deal_won */
            trigger: string;
        };
        LifecycleSettingsRequest: {
            disqualified_stage?: string | null;
            rules?: components["schemas"]["LifecycleTransitionRuleRequest"][];
        };
        LifecycleTransitionRuleRequest: {
            enabled?: boolean | null;
            from_empty?: boolean | null;
            from_stages?: string[] | null;
            target_stage?: string | null;
            /** @example all_deals_lost */
            trigger: string;
        };
        Data_LifecycleSettingsResponse: {
            data: {
                configured: boolean;
                disqualified_stage?: string | null;
                rules: components["schemas"]["LifecycleTransitionRuleResponse"][];
                stages: components["schemas"]["LifecycleStageOptionResponse"][];
                updated_at?: string | null;
            };
        };
        Data_LifecycleReportResponse: {
            data: {
                disqualifications: components["schemas"]["LifecycleDisqualificationCountResponse"][];
                /** @example 2026-01-01 */
                from: string;
                /** @example people.person */
                record_type: string;
                stages: components["schemas"]["LifecycleStageReportResponse"][];
                /** @example 2026-03-31 */
                to: string;
            };
        };
        Data_Vec_LifecycleHistoryEntryResponse: {
            data: {
                actor: components["schemas"]["LifecycleHistoryActorResponse"];
                /** Format: double */
                days_in_stage?: number | null;
                entered_at: string;
                from_stage?: string | null;
                id: string;
                left_at?: string | null;
                to_stage: string;
            }[];
        };
        DisqualifyLeadRequest: {
            /** @example Asked not to be contacted this semester */
            note?: string | null;
            reason_id: string;
        };
        Data_LeadActionResponse: {
            data: {
                contact_id: string;
                deal_id?: string | null;
                lifecycle_stage?: string | null;
            };
        };
        ConvertLeadRequest: {
            /** @example 15000.00 */
            amount?: string | null;
            /** @example BRL */
            currency?: string | null;
            pipeline_id: string;
            stage_id?: string | null;
            /** @example Onboarding - Acme */
            title?: string | null;
        };
        UpdateDisqualificationReasonRequest: {
            is_active?: boolean | null;
            name?: string | null;
        };
        ReorderDisqualificationReasonsRequest: {
            ids: string[];
        };
        Data_DisqualificationReasonResponse: {
            data: {
                created_at: string;
                id: string;
                is_active: boolean;
                /** @example no_budget */
                key: string;
                /** @example No budget */
                name: string;
                /** Format: int32 */
                position: number;
                updated_at: string;
            };
        };
        CreateDisqualificationReasonRequest: {
            /** @example wrong_region */
            key?: string | null;
            /** @example Outside service area */
            name: string;
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
        Data_Vec_DisqualificationReasonResponse: {
            data: {
                created_at: string;
                id: string;
                is_active: boolean;
                /** @example no_budget */
                key: string;
                /** @example No budget */
                name: string;
                /** Format: int32 */
                position: number;
                updated_at: string;
            }[];
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
    lead_disqualification_reasons_list: {
        parameters: {
            query?: {
                include_inactive?: boolean;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Reasons to disqualify a lead in their order (active only unless `include_inactive=true`). Six reasons are created with the organization */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_DisqualificationReasonResponse"];
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
            /** @description Missing `people.read` (`permission_denied`, `insufficient_scope`) */
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
    lead_disqualification_reasons_create: {
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
                "application/json": components["schemas"]["CreateDisqualificationReasonRequest"];
            };
        };
        responses: {
            /** @description Reason created at the end of the list (`lead_disqualification_reason.created`); `key` defaults to one derived from the name */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_DisqualificationReasonResponse"];
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
            /** @description Missing `lifecycle.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Another reason uses the key (`disqualification_reason_key_taken`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid name or key, or the limit of 50 reasons was reached (`validation_error`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    lead_disqualification_reasons_reorder: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ReorderDisqualificationReasonsRequest"];
            };
        };
        responses: {
            /** @description Reasons in the new order (`lead_disqualification_reason.reordered`); `ids` lists every reason, active or not, exactly once */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_DisqualificationReasonResponse"];
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
            /** @description Missing `lifecycle.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The list does not match the reasons of the organization (`validation_error`) */
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
    lead_disqualification_reasons_delete: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Reason id */
                reason_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Reason deleted (`lead_disqualification_reason.deleted`); past disqualifications keep the reason name */
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
            /** @description Missing `lifecycle.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Reason not found (`disqualification_reason_not_found`) */
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
    lead_disqualification_reasons_update: {
        parameters: {
            query?: never;
            header?: {
                /** @description `updated_at` of the reason as last read (ISO-8601); when it changed since, the update is refused with 409 `version_conflict` */
                "If-Match"?: string | null;
            };
            path: {
                /** @description Reason id */
                reason_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateDisqualificationReasonRequest"];
            };
        };
        responses: {
            /** @description Reason updated (`lead_disqualification_reason.updated`); the key never changes. Inactive reasons stay on past disqualifications but cannot be picked again */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_DisqualificationReasonResponse"];
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
            /** @description Missing `lifecycle.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Reason not found (`disqualification_reason_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The reason changed after the `If-Match` version (`version_conflict`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid name (`validation_error`) */
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
    leads_convert: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Contact id */
                contact_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ConvertLeadRequest"];
            };
        };
        responses: {
            /** @description Lead converted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_LeadActionResponse"];
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
            /** @description Missing `deals.create` or `people.update` on the contact (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Contact or pipeline not found (`contact_not_found`, `pipeline_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The contact is not in `lead` or `qualified` (`contact_not_lead`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid deal (`validation_error` with `details.fields`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    leads_disqualify: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Contact id */
                contact_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["DisqualifyLeadRequest"];
            };
        };
        responses: {
            /** @description Lead disqualified */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_LeadActionResponse"];
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
            /** @description Missing `people.update` on the contact or the field is read only (`permission_denied`, `field_read_only`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Contact not found or not visible (`contact_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The contact is not in `lead` or `qualified` (`contact_not_lead`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown or inactive reason, or note too long (`validation_error` with `details.fields`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    leads_qualify: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Contact id */
                contact_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Lead qualified */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_LeadActionResponse"];
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
            /** @description Missing `people.update` on the contact or the field is read only (`permission_denied`, `field_read_only`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Contact not found or not visible (`contact_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The contact is not in `lead` (`contact_not_lead`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The stage `qualified` is not an active option (`validation_error`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    lifecycle_history: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description `contact` or `company` */
                record_type: string;
                /** @description Record id */
                record_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Stage history */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_LifecycleHistoryEntryResponse"];
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
            /** @description Missing `people.read`/`organizations.read` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown record type, or record not found or not visible (`record_not_found`) */
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
    lifecycle_report: {
        parameters: {
            query?: {
                /** @example people.person */
                record_type?: string;
                /** @example 2026-01-01 */
                from?: string;
                /** @example 2026-03-31 */
                to?: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Lifecycle report */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_LifecycleReportResponse"];
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
            /** @description Missing `people.read`/`organizations.read` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid record type or period (`validation_error` with `details.fields`) */
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
    lifecycle_settings_get: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Lifecycle settings */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_LifecycleSettingsResponse"];
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
            /** @description Missing `lifecycle.manage` (`permission_denied`, `insufficient_scope`) */
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
    lifecycle_settings_update: {
        parameters: {
            query?: never;
            header?: {
                /** @description `updated_at` of the settings as read (ISO-8601); ignored while never saved */
                "If-Match"?: string | null;
            };
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["LifecycleSettingsRequest"];
            };
        };
        responses: {
            /** @description Lifecycle settings saved */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_LifecycleSettingsResponse"];
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
            /** @description Missing `lifecycle.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The settings changed since the `If-Match` version (`version_conflict`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown trigger or stage (`validation_error` with `details.fields`) */
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
}

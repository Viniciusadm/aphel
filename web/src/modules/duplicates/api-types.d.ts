export interface paths {
    "/api/v1/duplicates": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["duplicates_list"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/duplicates/check": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["duplicates_check"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/duplicates/merge": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["duplicates_merge"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/duplicates/rules": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["duplicates_get_rules"];
        put: operations["duplicates_update_rules"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/duplicates/scan": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["duplicates_scan"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/duplicates/types": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["duplicates_types"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/duplicates/{candidate_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["duplicates_get"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/duplicates/{candidate_id}/dismiss": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["duplicates_dismiss"];
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
        MergeResponse: {
            merged_id: string;
            /**
             * @example {
             *       "notes": 1,
             *       "tasks": 2
             *     }
             */
            moved: {
                [key: string]: number;
            };
            record_type: string;
            survivor_id: string;
        };
        DuplicateMatchResponse: {
            record_id: string;
            /**
             * @example [
             *       "email"
             *     ]
             */
            rules: string[];
            title: string;
        };
        DuplicateRulesResponse: {
            record_type: string;
            rules: components["schemas"]["DuplicateRuleResponse"][];
        };
        DuplicateRuleResponse: {
            enabled: boolean;
            /** @example email */
            rule: string;
        };
        DuplicateCandidateResponse: {
            detected_at: string;
            duplicate: components["schemas"]["DuplicateRecordSummaryResponse"];
            id: string;
            record: components["schemas"]["DuplicateRecordSummaryResponse"];
            /** @example people.person */
            record_type: string;
            resolved_at?: string | null;
            /**
             * @example [
             *       "email",
             *       "phone"
             *     ]
             */
            rules: string[];
            /** @example pending */
            status: string;
        };
        DuplicateRecordSummaryResponse: {
            created_at: string;
            id: string;
            owner_id?: string | null;
            subtitle?: string | null;
            title: string;
        };
        Data_DuplicateCandidateResponse: {
            data: {
                detected_at: string;
                duplicate: components["schemas"]["DuplicateRecordSummaryResponse"];
                id: string;
                record: components["schemas"]["DuplicateRecordSummaryResponse"];
                /** @example people.person */
                record_type: string;
                resolved_at?: string | null;
                /**
                 * @example [
                 *       "email",
                 *       "phone"
                 *     ]
                 */
                rules: string[];
                /** @example pending */
                status: string;
            };
        };
        Data_Vec_DuplicateTypeResponse: {
            data: {
                custom: boolean;
                label?: string | null;
                /**
                 * @example [
                 *       "first_name",
                 *       "email"
                 *     ]
                 */
                merge_fields: string[];
                permissions: components["schemas"]["DuplicateTypePermissionsResponse"];
                /** @example people.person */
                record_type: string;
                rules: components["schemas"]["DuplicateRuleResponse"][];
            }[];
        };
        DuplicateTypePermissionsResponse: {
            /** @example people.delete */
            delete: string;
            /** @example people.read */
            read: string;
            /** @example people.update */
            update: string;
        };
        UpdateDuplicateRulesRequest: {
            /** @example people.person */
            record_type: string;
            /**
             * @example {
             *       "email": true,
             *       "phone": false
             *     }
             */
            rules: {
                [key: string]: boolean;
            };
        };
        Data_DuplicateRulesResponse: {
            data: {
                record_type: string;
                rules: components["schemas"]["DuplicateRuleResponse"][];
            };
        };
        Data_MergeResponse: {
            data: {
                merged_id: string;
                /**
                 * @example {
                 *       "notes": 1,
                 *       "tasks": 2
                 *     }
                 */
                moved: {
                    [key: string]: number;
                };
                record_type: string;
                survivor_id: string;
            };
        };
        MergeRequest: {
            duplicate_id: string;
            /**
             * @example {
             *       "email": "duplicate",
             *       "phone": "primary"
             *     }
             */
            fields?: {
                [key: string]: string;
            };
            primary_id: string;
            /** @example people.person */
            record_type: string;
        };
        Data_Vec_DuplicateMatchResponse: {
            data: {
                record_id: string;
                /**
                 * @example [
                 *       "email"
                 *     ]
                 */
                rules: string[];
                title: string;
            }[];
        };
        DuplicateCheckRequest: {
            company_id?: string | null;
            domain?: string | null;
            email?: string | null;
            exclude_id?: string | null;
            first_name?: string | null;
            last_name?: string | null;
            mobile?: string | null;
            name?: string | null;
            phone?: string | null;
            /** @example people.person */
            record_type: string;
            /**
             * @example {
             *       "code": "SKU-1"
             *     }
             */
            values?: {
                [key: string]: string;
            };
            website?: string | null;
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
        Page_DuplicateCandidateResponse: {
            data: {
                detected_at: string;
                duplicate: components["schemas"]["DuplicateRecordSummaryResponse"];
                id: string;
                record: components["schemas"]["DuplicateRecordSummaryResponse"];
                /** @example people.person */
                record_type: string;
                resolved_at?: string | null;
                /**
                 * @example [
                 *       "email",
                 *       "phone"
                 *     ]
                 */
                rules: string[];
                /** @example pending */
                status: string;
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
    };
    responses: never;
    parameters: never;
    requestBodies: never;
    headers: never;
    pathItems: never;
}
export type $defs = Record<string, never>;
export interface operations {
    duplicates_list: {
        parameters: {
            query?: {
                /** @example people.person */
                record_type?: string;
                /** @example pending */
                status?: string;
                page?: number;
                per_page?: number;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Duplicate candidates of the record type (default `contact`) with the status (default `pending`), newest first. Only pairs whose two records the user can read are listed */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page_DuplicateCandidateResponse"];
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
            /** @description Missing the read permission of the record type (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid record type, status or pagination (`validation_error`) */
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
    duplicates_check: {
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
                "application/json": components["schemas"]["DuplicateCheckRequest"];
            };
        };
        responses: {
            /** @description Visible records that the enabled rules consider possible duplicates of the given values (at most 10), each with the rules that matched. Nothing is stored */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_DuplicateMatchResponse"];
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
            /** @description Missing the create or update permission of the record type (`permission_denied`, `insufficient_scope`) */
            403: {
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
            /** @description Invalid record type (`validation_error`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    duplicates_merge: {
        parameters: {
            query?: never;
            header?: {
                /** @description `updated_at` of the primary record as read (ISO-8601, quotes optional); the merge fails with 409 when it changed since */
                "If-Match"?: string | null;
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["MergeRequest"];
            };
        };
        responses: {
            /** @description Merges `duplicate_id` into `primary_id` in one transaction. `fields` chooses, per field key (system or custom), which record's value stays (`primary` or `duplicate`); omitted fields keep the primary value unless it is empty. Fields hidden from the user cannot be chosen: their values stay on the primary and are only filled from the duplicate when empty (redacted in the audit). Activities, notes, attachments, tags, relationships, record references, deals and company links (people), people and deals (organizations) or rows of other types that point to it (other types) move to the primary; the duplicate goes to the trash and its id redirects to the primary. The duplicate keeps its own custom values in the trash (restoring it brings them back), while the moved links stay with the primary. Both writes go through the record pipeline: the primary update runs validation rules, approvals and the other `RecordValidating` hooks and publishes `<type>.updated` (when something changed), the duplicate runs `RecordDeleting`/`RecordDeleted` and publishes `<type>.deleted`; `<type>.merged` is published as well */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_MergeResponse"];
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
            /** @description Missing update permission on the primary or delete permission on the duplicate (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Record not found or not visible (`contact_not_found`, `company_not_found`, `record_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The primary changed since the `If-Match` version, or one of the records changed or was merged while the merge was prepared (`version_conflict`); an approval blocks the update (`approval_required`, `approval_pending`); a restrict relationship blocks the deletion of the duplicate. The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid record type, unknown field or side (`fields.<key>`), invalid custom value, a validation rule (`validation_rule_failed`), the same record twice (`merge_same_record`) or, for organizations, moving the children of the duplicate would break the hierarchy (`company_hierarchy_cycle`, `company_hierarchy_too_deep`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    duplicates_get_rules: {
        parameters: {
            query: {
                /** @example people.person */
                record_type: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Detection rules of the record type. Contacts: `email`, `phone`, `name_company` (similar name and same company); organizations: `domain`, `name`; other types: `identifier` (same value in an identifier column, when the type declares one) and `name` (similar title). Every rule is enabled by default */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_DuplicateRulesResponse"];
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
            /** @description Missing the read permission of the record type (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid record type (`validation_error`) */
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
    duplicates_update_rules: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateDuplicateRulesRequest"];
            };
        };
        responses: {
            /** @description Rules of the record type replaced by the given switches (omitted rules keep their state). Publishes `duplicate_rules.updated` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_DuplicateRulesResponse"];
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
            /** @description Missing `settings.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid record type or unknown rule (`validation_error` on `rules.<key>`) */
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
    duplicates_scan: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Queues the `duplicates.scan` job (it also runs daily) */
            202: {
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
            /** @description Missing `settings.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
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
    duplicates_types: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Record types with duplicate detection that the user can read: people, organizations and every type that declares identifier columns (`label_aliases`, e.g. products by code) or is a custom object. Each comes with its rules (`identifier` and `name` for generic types), the system fields the merge screen may offer (fields hidden from the user are left out) and the permissions needed to merge */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_DuplicateTypeResponse"];
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
    duplicates_get: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Duplicate candidate id */
                candidate_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Duplicate candidate with a summary of both records and its status (`pending`, `dismissed` or `merged`; for a merged candidate the merged record is already in the trash) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_DuplicateCandidateResponse"];
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
            /** @description Missing the read permission of the record type (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Candidate not found or one of its records is not visible (`duplicate_candidate_not_found`) */
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
    duplicates_dismiss: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Duplicate candidate id */
                candidate_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Candidate marked as not a duplicate; later scans keep it dismissed. Publishes `duplicate.dismissed` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_DuplicateCandidateResponse"];
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
            /** @description Missing the update permission of the record type (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Candidate not found (`duplicate_candidate_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Candidate already dismissed or merged (`duplicate_candidate_resolved`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
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
}

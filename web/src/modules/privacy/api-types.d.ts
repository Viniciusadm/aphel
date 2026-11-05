export interface paths {
    "/api/v1/privacy/catalog": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["privacy_catalog"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/privacy/requests": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["privacy_list_requests"];
        put?: never;
        post: operations["privacy_create_request"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/privacy/requests/{request_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["privacy_get_request"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/privacy/requests/{request_id}/cancel": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["privacy_cancel_request"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/privacy/requests/{request_id}/download": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["privacy_download_request"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/privacy/requests/{request_id}/execute": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["privacy_execute_request"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/privacy/retention-policies": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["privacy_list_retention_policies"];
        put?: never;
        post: operations["privacy_create_retention_policy"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/privacy/retention-policies/{policy_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        delete: operations["privacy_delete_retention_policy"];
        options?: never;
        head?: never;
        patch: operations["privacy_update_retention_policy"];
        trace?: never;
    };
    "/api/v1/privacy/retention-policies/{policy_id}/disable": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["privacy_disable_retention_policy"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/privacy/retention-policies/{policy_id}/enable": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["privacy_enable_retention_policy"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/privacy/retention-policies/{policy_id}/preview": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["privacy_preview_retention_policy"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/privacy/retention-policies/{policy_id}/run": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["privacy_run_retention_policy"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/privacy/retention-runs": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["privacy_list_retention_runs"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/privacy/subjects/search": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["privacy_search_subject"];
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
        PrivacyCatalogResponse: {
            entities: components["schemas"]["PrivacyEntityResponse"][];
            sources: components["schemas"]["PrivacySourceResponse"][];
        };
        PrivacySourceResponse: {
            key: string;
            labels: {
                [key: string]: string;
            };
        };
        PrivacyEntityResponse: {
            labels: {
                [key: string]: string;
            };
            personal_data: boolean;
            record_type: string;
            retention: boolean;
        };
        RetentionRunResponse: {
            action: string;
            /** Format: int64 */
            affected: number;
            created_at: string;
            error_message?: string | null;
            /** Format: int32 */
            errors: number;
            /** Format: int64 */
            files_removed: number;
            finished_at?: string | null;
            id: string;
            policy_id?: string | null;
            policy_name: string;
            record_type?: string | null;
            requested_by?: null | components["schemas"]["OwnerResponse"];
            started_at?: string | null;
            /** @example completed */
            status: string;
            /** Format: int32 */
            threshold_days: number;
            /** @example schedule */
            trigger: string;
        };
        OwnerResponse: {
            display_name: string;
            id: string;
        };
        RetentionPreviewResponse: {
            /** Format: int64 */
            affected: number;
            previewed_at: string;
            threshold_at: string;
        };
        RetentionPolicyResponse: {
            /** @example archive_inactive */
            action: string;
            created_at: string;
            created_by?: null | components["schemas"]["OwnerResponse"];
            id: string;
            is_enabled: boolean;
            last_run_at?: string | null;
            name: string;
            preview_fresh: boolean;
            previewed_at?: string | null;
            /** Format: int64 */
            previewed_count?: number | null;
            record_type?: string | null;
            /** Format: int32 */
            threshold_days: number;
            updated_at: string;
        };
        PrivacyRequestDetailResponse: {
            items: components["schemas"]["PrivacySourceTargetResponse"][];
            records: components["schemas"]["PrivacyTargetRecordResponse"][];
            request: components["schemas"]["PrivacyRequestResponse"];
            sources: components["schemas"]["PrivacySourceResponse"][];
        };
        PrivacyRequestResponse: {
            created_at: string;
            download_available: boolean;
            due_at: string;
            executed_by?: null | components["schemas"]["OwnerResponse"];
            failure_reason?: string | null;
            finished_at?: string | null;
            id: string;
            /** @example export */
            kind: string;
            overdue: boolean;
            requested_by?: null | components["schemas"]["OwnerResponse"];
            result?: null | components["schemas"]["PrivacyReportResponse"];
            started_at?: string | null;
            /** @example pending */
            status: string;
            /** @example email */
            subject_kind: string;
            /** @example m***@e***.com */
            subject_label: string;
            targets: components["schemas"]["PrivacyTargetCountsResponse"];
        };
        PrivacyTargetCountsResponse: {
            items: number;
            records: number;
        };
        PrivacyReportResponse: {
            /** Format: int64 */
            attachments: number;
            /** Format: int64 */
            errors: number;
            /** Format: int64 */
            external_items: number;
            /** Format: int64 */
            files: number;
            /** Format: int64 */
            history_entries: number;
            /** Format: int64 */
            records: number;
            /** Format: int64 */
            skipped: number;
            source_items: {
                [key: string]: number;
            };
        };
        PrivacyTargetRecordResponse: {
            exists: boolean;
            id: string;
            /** @example people.person */
            record_type: string;
            title: string;
        };
        PrivacySourceTargetResponse: {
            id: string;
            source: string;
        };
        SubjectAttachmentResponse: {
            created_at: string;
            file_id: string;
            file_name: string;
            id: string;
            mime_type: string;
            record: components["schemas"]["RecordRefResponse"];
            /** Format: int64 */
            size: number;
        };
        RecordRefResponse: {
            id: string;
            /** @example people.person */
            record_type: string;
        };
        SubjectRecordResponse: {
            archived: boolean;
            deleted: boolean;
            id: string;
            matched_fields: string[];
            /** @example people.person */
            record_type: string;
            subtitle?: string | null;
            title: string;
        };
        SubjectSearchResponse: {
            attachments: components["schemas"]["SubjectAttachmentResponse"][];
            items: components["schemas"]["PrivacySourceItemResponse"][];
            kind: string;
            label: string;
            records: components["schemas"]["SubjectRecordResponse"][];
            sources: components["schemas"]["PrivacySourceResponse"][];
        };
        PrivacySourceItemResponse: {
            excerpt?: string | null;
            id: string;
            metadata: Record<string, never>;
            reason: string;
            record?: null | components["schemas"]["RecordRefResponse"];
            source: string;
            title: string;
        };
        Data_SubjectSearchResponse: {
            data: {
                attachments: components["schemas"]["SubjectAttachmentResponse"][];
                items: components["schemas"]["PrivacySourceItemResponse"][];
                kind: string;
                label: string;
                records: components["schemas"]["SubjectRecordResponse"][];
                sources: components["schemas"]["PrivacySourceResponse"][];
            };
        };
        SubjectSearchRequest: {
            /** @example maria@example.com */
            term: string;
        };
        Page_RetentionRunResponse: {
            data: {
                action: string;
                /** Format: int64 */
                affected: number;
                created_at: string;
                error_message?: string | null;
                /** Format: int32 */
                errors: number;
                /** Format: int64 */
                files_removed: number;
                finished_at?: string | null;
                id: string;
                policy_id?: string | null;
                policy_name: string;
                record_type?: string | null;
                requested_by?: null | components["schemas"]["OwnerResponse"];
                started_at?: string | null;
                /** @example completed */
                status: string;
                /** Format: int32 */
                threshold_days: number;
                /** @example schedule */
                trigger: string;
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
        Data_RetentionRunResponse: {
            data: {
                action: string;
                /** Format: int64 */
                affected: number;
                created_at: string;
                error_message?: string | null;
                /** Format: int32 */
                errors: number;
                /** Format: int64 */
                files_removed: number;
                finished_at?: string | null;
                id: string;
                policy_id?: string | null;
                policy_name: string;
                record_type?: string | null;
                requested_by?: null | components["schemas"]["OwnerResponse"];
                started_at?: string | null;
                /** @example completed */
                status: string;
                /** Format: int32 */
                threshold_days: number;
                /** @example schedule */
                trigger: string;
            };
        };
        Data_RetentionPreviewResponse: {
            data: {
                /** Format: int64 */
                affected: number;
                previewed_at: string;
                threshold_at: string;
            };
        };
        UpdateRetentionPolicyBody: {
            name?: string | null;
            /** Format: int32 */
            threshold_days?: number | null;
        };
        Data_RetentionPolicyResponse: {
            data: {
                /** @example archive_inactive */
                action: string;
                created_at: string;
                created_by?: null | components["schemas"]["OwnerResponse"];
                id: string;
                is_enabled: boolean;
                last_run_at?: string | null;
                name: string;
                preview_fresh: boolean;
                previewed_at?: string | null;
                /** Format: int64 */
                previewed_count?: number | null;
                record_type?: string | null;
                /** Format: int32 */
                threshold_days: number;
                updated_at: string;
            };
        };
        CreateRetentionPolicyBody: {
            /** @example archive_inactive */
            action: string;
            /** @example Archive inactive people */
            name: string;
            /** @example people.person */
            record_type?: string | null;
            /**
             * Format: int32
             * @example 1095
             */
            threshold_days: number;
        };
        Data_Vec_RetentionPolicyResponse: {
            data: {
                /** @example archive_inactive */
                action: string;
                created_at: string;
                created_by?: null | components["schemas"]["OwnerResponse"];
                id: string;
                is_enabled: boolean;
                last_run_at?: string | null;
                name: string;
                preview_fresh: boolean;
                previewed_at?: string | null;
                /** Format: int64 */
                previewed_count?: number | null;
                record_type?: string | null;
                /** Format: int32 */
                threshold_days: number;
                updated_at: string;
            }[];
        };
        Data_PrivacyRequestDetailResponse: {
            data: {
                items: components["schemas"]["PrivacySourceTargetResponse"][];
                records: components["schemas"]["PrivacyTargetRecordResponse"][];
                request: components["schemas"]["PrivacyRequestResponse"];
                sources: components["schemas"]["PrivacySourceResponse"][];
            };
        };
        Data_PrivacyRequestResponse: {
            data: {
                created_at: string;
                download_available: boolean;
                due_at: string;
                executed_by?: null | components["schemas"]["OwnerResponse"];
                failure_reason?: string | null;
                finished_at?: string | null;
                id: string;
                /** @example export */
                kind: string;
                overdue: boolean;
                requested_by?: null | components["schemas"]["OwnerResponse"];
                result?: null | components["schemas"]["PrivacyReportResponse"];
                started_at?: string | null;
                /** @example pending */
                status: string;
                /** @example email */
                subject_kind: string;
                /** @example m***@e***.com */
                subject_label: string;
                targets: components["schemas"]["PrivacyTargetCountsResponse"];
            };
        };
        CreatePrivacyRequestBody: {
            due_at?: string | null;
            /** @example export */
            kind: string;
            targets?: components["schemas"]["PrivacyTargetsBody"];
            /** @example maria@example.com */
            term: string;
        };
        PrivacyTargetsBody: {
            items?: components["schemas"]["PrivacySourceTargetResponse"][];
            records?: components["schemas"]["RecordRefBody"][];
        };
        RecordRefBody: {
            id: string;
            /** @example people.person */
            record_type: string;
        };
        Page_PrivacyRequestResponse: {
            data: {
                created_at: string;
                download_available: boolean;
                due_at: string;
                executed_by?: null | components["schemas"]["OwnerResponse"];
                failure_reason?: string | null;
                finished_at?: string | null;
                id: string;
                /** @example export */
                kind: string;
                overdue: boolean;
                requested_by?: null | components["schemas"]["OwnerResponse"];
                result?: null | components["schemas"]["PrivacyReportResponse"];
                started_at?: string | null;
                /** @example pending */
                status: string;
                /** @example email */
                subject_kind: string;
                /** @example m***@e***.com */
                subject_label: string;
                targets: components["schemas"]["PrivacyTargetCountsResponse"];
            }[];
            meta: components["schemas"]["PageMeta"];
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
        Data_PrivacyCatalogResponse: {
            data: {
                entities: components["schemas"]["PrivacyEntityResponse"][];
                sources: components["schemas"]["PrivacySourceResponse"][];
            };
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
    privacy_catalog: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Available privacy sources and entity policies */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PrivacyCatalogResponse"];
                };
            };
            /** @description Missing privacy.manage */
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
    privacy_list_requests: {
        parameters: {
            query?: {
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
            /** @description Data subject requests of the organization, most recent first; `overdue` is true for open requests past `due_at` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page_PrivacyRequestResponse"];
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
            /** @description Missing `privacy.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid status or pagination (`validation_error`) */
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
    privacy_create_request: {
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
                "application/json": components["schemas"]["CreatePrivacyRequestBody"];
            };
        };
        responses: {
            /** @description Request registered as `pending` (`privacy_request.created`) with the selected records (contact, company, deal, activity), notes and import rows that still exist. Only a SHA-256 hash and a masked label of the term are stored. `due_at` defaults to 15 days (LGPD art. 19) */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PrivacyRequestResponse"];
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
            /** @description Missing `privacy.manage` (`permission_denied`, `insufficient_scope`) */
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
            /** @description Invalid kind, term, targets or due date (`validation_error`, `invalid_subject_term`), or none of the targets exists (`privacy_targets_empty`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    privacy_get_request: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Privacy request id */
                request_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Request with its targets; record titles are read now (`exists` is false for records already removed) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PrivacyRequestDetailResponse"];
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
            /** @description Missing `privacy.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Request not found (`privacy_request_not_found`) */
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
    privacy_cancel_request: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Privacy request id */
                request_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Pending request cancelled (`privacy_request.cancelled`) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PrivacyRequestResponse"];
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
            /** @description Missing `privacy.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Request not found (`privacy_request_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The request is not pending (`privacy_request_not_pending`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
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
    privacy_download_request: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Privacy request id */
                request_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Zip of a completed export request (`dados.json` + `anexos/`), served as an attachment. Each download is audited (`privacy_request.downloaded`). The file is removed 30 days after the request finished */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/zip": unknown;
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
            /** @description Missing `privacy.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Request not found (`privacy_request_not_found`) or without an available file (`privacy_export_unavailable`) */
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
    privacy_execute_request: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Privacy request id */
                request_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Request moved to `processing` and queued (`privacy.request.run`). Export writes a zip with `dados.json` and the attachments; anonymize replaces personal fields with neutral values and removes notes, attachments (and their files) of the records while keeping deals, amounts, stages and dates; delete removes the records for good. Both redact personal values from the history (`audit_logs`, `domain_events`). Finishes with `privacy_request.completed` or `privacy_request.failed` */
            202: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PrivacyRequestResponse"];
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
            /** @description Missing `privacy.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Request not found (`privacy_request_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The request is not pending (`privacy_request_not_pending`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
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
    privacy_list_retention_policies: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Retention policies of the organization, oldest first */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_RetentionPolicyResponse"];
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
            /** @description Missing `privacy.manage` (`permission_denied`, `insufficient_scope`) */
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
    privacy_create_retention_policy: {
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
                "application/json": components["schemas"]["CreateRetentionPolicyBody"];
            };
        };
        responses: {
            /** @description Policy created disabled (`retention_policy.created`). Actions: `archive_inactive` (people, organizations or deals without updates, tasks or notes for N days), `purge_archived` (deletes for good records archived N days ago, with their files), `purge_audit_logs`, `purge_security_events`, `purge_domain_events` (dispatched events; the timeline loses that history) and `purge_orphan_files` (files no longer referenced, also removed from the storage). The trash keeps its own retention (`trash.purge`) */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_RetentionPolicyResponse"];
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
            /** @description Missing `privacy.manage` (`permission_denied`, `insufficient_scope`) */
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
            /** @description Invalid name, action, record type or threshold (`validation_error`), or 50 policies already (`retention_policy_limit`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    privacy_delete_retention_policy: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Retention policy id */
                policy_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Policy deleted (`retention_policy.deleted`); its runs stay in the history */
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
            /** @description Missing `privacy.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Policy not found (`retention_policy_not_found`) */
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
    privacy_update_retention_policy: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Retention policy id */
                policy_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateRetentionPolicyBody"];
            };
        };
        responses: {
            /** @description Policy updated (`retention_policy.updated`). Changing the threshold disables the policy until a new preview is confirmed */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_RetentionPolicyResponse"];
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
            /** @description Missing `privacy.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Policy not found (`retention_policy_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid name or threshold (`validation_error`) */
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
    privacy_disable_retention_policy: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Retention policy id */
                policy_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Policy disabled (`retention_policy.disabled`) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_RetentionPolicyResponse"];
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
            /** @description Missing `privacy.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Policy not found (`retention_policy_not_found`) */
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
    privacy_enable_retention_policy: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Retention policy id */
                policy_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Policy enabled (`retention_policy.enabled`); the `privacy.retention` job runs it daily at 04:00 UTC */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_RetentionPolicyResponse"];
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
            /** @description Missing `privacy.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Policy not found (`retention_policy_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description No recent preview (`retention_preview_required`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
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
    privacy_preview_retention_policy: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Retention policy id */
                policy_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description How many items the policy would affect now. Enabling or running the policy requires a preview from the last hour, made after its last change */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_RetentionPreviewResponse"];
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
            /** @description Missing `privacy.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Policy not found (`retention_policy_not_found`) */
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
    privacy_run_retention_policy: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Retention policy id */
                policy_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Manual run queued (`retention_run.requested`, job `privacy.retention.run`), even for a disabled policy. Finishes with `retention_run.completed` or `retention_run.failed` */
            202: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_RetentionRunResponse"];
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
            /** @description Missing `privacy.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Policy not found (`retention_policy_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description No recent preview (`retention_preview_required`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
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
    privacy_list_retention_runs: {
        parameters: {
            query?: {
                policy_id?: string;
                page?: number;
                per_page?: number;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Retention runs (scheduled and manual), most recent first, with the number of affected items, removed files and errors */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page_RetentionRunResponse"];
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
            /** @description Missing `privacy.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid policy id or pagination (`validation_error`) */
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
    privacy_search_subject: {
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
                "application/json": components["schemas"]["SubjectSearchRequest"];
            };
        };
        responses: {
            /** @description Where the data subject appears: records whose personal fields (system or custom, `is_personal`) match the e-mail (lowercase), phone (E.164 or digits) or document (digits) — or contain the text otherwise —, notes that mention the term or belong to those records, their attachments and tasks, and import rows whose raw data contains the term (up to 100 per group). The term goes in the body so it never reaches URLs or access logs; the search is audited (`privacy.subject_searched`) without the term */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_SubjectSearchResponse"];
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
            /** @description Missing `privacy.manage` (`permission_denied`, `insufficient_scope`) */
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
            /** @description Term shorter than 3 or longer than 255 characters (`invalid_subject_term`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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

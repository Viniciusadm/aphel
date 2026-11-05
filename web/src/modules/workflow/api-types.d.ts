export interface paths {
    "/api/v1/lost-reasons": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["lost_reasons_list"];
        put?: never;
        post: operations["lost_reasons_create"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/lost-reasons/order": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put: operations["lost_reasons_reorder"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/lost-reasons/{reason_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["lost_reasons_get"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch: operations["lost_reasons_update"];
        trace?: never;
    };
    "/api/v1/pipeline-stages": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["workflow_stages_list"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/pipeline-stages/{stage_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["workflow_stages_get"];
        put?: never;
        post?: never;
        delete: operations["workflow_stages_delete"];
        options?: never;
        head?: never;
        patch: operations["workflow_stages_update"];
        trace?: never;
    };
    "/api/v1/pipeline-stages/{stage_id}/activate": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["workflow_stages_activate"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/pipeline-stages/{stage_id}/deactivate": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["workflow_stages_deactivate"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/pipeline-stages/{stage_id}/guidance": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Guidance (markdown) and checklist of a pipeline stage. Readable by anyone who reads records. */
        get: operations["workflow_stage_guidance_get"];
        /** @description Saves the guidance and the checklist of a stage and publishes `pipeline.stage_guidance_updated`. `items` replaces the whole checklist in the given order: items are matched by `id` (or `key`), new ones get a `key` from the label, and items left out are removed with their progress. `required` items must be done before a record **advances** from the stage (moves to a stage of greater position in the same pipeline, or is won); moving back, to another pipeline or losing does not check the checklist. Needs `settings.manage`. */
        put: operations["workflow_stage_guidance_update"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/workflow": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["pipelines_list"];
        put?: never;
        post: operations["pipelines_create"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/workflow/order": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put: operations["pipelines_reorder"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/workflow/records/{record_type}/{record_id}/checklist": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Guidance and checklist of the current stage of the record with the progress of the record (who checked each item and when). Needs read access to the record. */
        get: operations["record_checklist_get"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/workflow/records/{record_type}/{record_id}/checklist/{item_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** @description Checks (`done: true`) or unchecks an item of a stage checklist of the record's pipeline and publishes `record.checklist_item_checked` or `record.checklist_item_unchecked`. Needs permission to update the record. */
        put: operations["record_checklist_item_update"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/workflow/required-fields": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["workflow_required_fields"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/workflow/{pipeline_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["pipelines_get"];
        put?: never;
        post?: never;
        delete: operations["pipelines_delete"];
        options?: never;
        head?: never;
        patch: operations["pipelines_update"];
        trace?: never;
    };
    "/api/v1/workflow/{pipeline_id}/activate": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["pipelines_activate"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/workflow/{pipeline_id}/deactivate": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["pipelines_deactivate"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/workflow/{pipeline_id}/default": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["pipelines_set_default"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/workflow/{pipeline_id}/stages": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["pipelines_create_stage"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/workflow/{pipeline_id}/stages/order": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put: operations["pipelines_reorder_stages"];
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
        RecordChecklistResponse: {
            guidance: string;
            items: components["schemas"]["RecordChecklistItemResponse"][];
            record_id: string;
            record_type: string;
            required_pending: number;
            stage_id: string;
            stage_name: string;
        };
        RecordChecklistItemResponse: {
            done: boolean;
            /** @example 2026-10-20T13:45:00Z */
            done_at?: string | null;
            done_by?: null | components["schemas"]["ChecklistUserResponse"];
            id: string;
            key: string;
            label: string;
            /** Format: int32 */
            position: number;
            required: boolean;
        };
        ChecklistUserResponse: {
            display_name?: string | null;
            id: string;
        };
        ChecklistItemRequest: {
            id?: string | null;
            /** @example send_proposal */
            key?: string | null;
            /** @example Send the proposal */
            label: string;
            required?: boolean;
        };
        StageGuidanceResponse: {
            /** @example Confirm the budget and the decision maker before sending the proposal. */
            guidance: string;
            items: components["schemas"]["ChecklistItemResponse"][];
            pipeline_id: string;
            stage_id: string;
            stage_name: string;
            /** @example 2026-10-20T13:45:00Z */
            updated_at?: string | null;
            updated_by?: string | null;
        };
        ChecklistItemResponse: {
            id: string;
            /** @example send_proposal */
            key: string;
            /** @example Send the proposal */
            label: string;
            /** Format: int32 */
            position: number;
            required: boolean;
        };
        CreateStageRequest: {
            entry_required_fields?: string[];
            exit_required_fields?: string[];
            kind?: null | components["schemas"]["StageKind"];
            /** @example Demo */
            name: string;
            /**
             * Format: int64
             * @example 40
             */
            probability?: number | null;
            /**
             * Format: int64
             * @example 7
             */
            rotting_days?: number | null;
        };
        /** @enum {string} */
        StageKind: "open" | "won" | "lost";
        UpdatePipelineRequest: {
            name?: string | null;
            team_ids?: string[] | null;
        };
        PipelineRemovalRequest: {
            stage_mapping?: {
                [key: string]: string;
            };
            target_pipeline_id?: string | null;
        };
        Data_Vec_StageFieldChoice: {
            data: {
                key: string;
                label: string;
            }[];
        };
        ToggleChecklistItemRequest: {
            done: boolean;
        };
        Data_RecordChecklistResponse: {
            data: {
                guidance: string;
                items: components["schemas"]["RecordChecklistItemResponse"][];
                record_id: string;
                record_type: string;
                required_pending: number;
                stage_id: string;
                stage_name: string;
            };
        };
        Data_Vec_PipelineResponse: {
            data: {
                created_at: string;
                id: string;
                is_active: boolean;
                is_default: boolean;
                /** @example Sales pipeline */
                name: string;
                /** Format: int32 */
                position: number;
                stages: components["schemas"]["PipelineStageResponse"][];
                team_ids: string[];
                updated_at: string;
            }[];
        };
        PipelineStageResponse: {
            created_at: string;
            /**
             * @example [
             *       "owner_id",
             *       "due_at"
             *     ]
             */
            entry_required_fields: string[];
            exit_required_fields: string[];
            id: string;
            is_active: boolean;
            kind: components["schemas"]["StageKind"];
            /** @example Proposal */
            name: string;
            pipeline_id: string;
            /** Format: int32 */
            position: number;
            /**
             * Format: int32
             * @example 50
             */
            probability: number;
            /** Format: int64 */
            record_count?: number | null;
            /**
             * Format: int32
             * @example 7
             */
            rotting_days?: number | null;
            updated_at: string;
        };
        Data_PipelineResponse: {
            data: {
                created_at: string;
                id: string;
                is_active: boolean;
                is_default: boolean;
                /** @example Sales pipeline */
                name: string;
                /** Format: int32 */
                position: number;
                stages: components["schemas"]["PipelineStageResponse"][];
                team_ids: string[];
                updated_at: string;
            };
        };
        PipelineResponse: {
            created_at: string;
            id: string;
            is_active: boolean;
            is_default: boolean;
            /** @example Sales pipeline */
            name: string;
            /** Format: int32 */
            position: number;
            stages: components["schemas"]["PipelineStageResponse"][];
            team_ids: string[];
            updated_at: string;
        };
        CreatePipelineRequest: {
            /** @example Rentals */
            name: string;
            stages?: components["schemas"]["NewStageRequest"][];
            team_ids?: string[];
        };
        NewStageRequest: {
            kind?: null | components["schemas"]["StageKind"];
            /** @example Visit */
            name: string;
            /** Format: int64 */
            probability?: number | null;
        };
        Page_PipelineResponse: {
            data: {
                created_at: string;
                id: string;
                is_active: boolean;
                is_default: boolean;
                /** @example Sales pipeline */
                name: string;
                /** Format: int32 */
                position: number;
                stages: components["schemas"]["PipelineStageResponse"][];
                team_ids: string[];
                updated_at: string;
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
        StageGuidanceRequest: {
            guidance?: string | null;
            items?: components["schemas"]["ChecklistItemRequest"][] | null;
        };
        Data_StageGuidanceResponse: {
            data: {
                /** @example Confirm the budget and the decision maker before sending the proposal. */
                guidance: string;
                items: components["schemas"]["ChecklistItemResponse"][];
                pipeline_id: string;
                stage_id: string;
                stage_name: string;
                /** @example 2026-10-20T13:45:00Z */
                updated_at?: string | null;
                updated_by?: string | null;
            };
        };
        StageRemovalRequest: {
            target_stage_id?: string | null;
        };
        UpdateStageRequest: {
            /**
             * @example [
             *       "owner_id",
             *       "due_at"
             *     ]
             */
            entry_required_fields?: string[] | null;
            exit_required_fields?: string[] | null;
            kind?: null | components["schemas"]["StageKind"];
            name?: string | null;
            /** Format: int64 */
            probability?: number | null;
            /**
             * Format: int64
             * @example 7
             */
            rotting_days?: number | null;
        };
        Data_PipelineStageResponse: {
            data: {
                created_at: string;
                /**
                 * @example [
                 *       "owner_id",
                 *       "due_at"
                 *     ]
                 */
                entry_required_fields: string[];
                exit_required_fields: string[];
                id: string;
                is_active: boolean;
                kind: components["schemas"]["StageKind"];
                /** @example Proposal */
                name: string;
                pipeline_id: string;
                /** Format: int32 */
                position: number;
                /**
                 * Format: int32
                 * @example 50
                 */
                probability: number;
                /** Format: int64 */
                record_count?: number | null;
                /**
                 * Format: int32
                 * @example 7
                 */
                rotting_days?: number | null;
                updated_at: string;
            };
        };
        Page_PipelineStageResponse: {
            data: {
                created_at: string;
                /**
                 * @example [
                 *       "owner_id",
                 *       "due_at"
                 *     ]
                 */
                entry_required_fields: string[];
                exit_required_fields: string[];
                id: string;
                is_active: boolean;
                kind: components["schemas"]["StageKind"];
                /** @example Proposal */
                name: string;
                pipeline_id: string;
                /** Format: int32 */
                position: number;
                /**
                 * Format: int32
                 * @example 50
                 */
                probability: number;
                /** Format: int64 */
                record_count?: number | null;
                /**
                 * Format: int32
                 * @example 7
                 */
                rotting_days?: number | null;
                updated_at: string;
            }[];
            meta: components["schemas"]["PageMeta"];
        };
        UpdateLostReasonRequest: {
            is_active?: boolean | null;
            name?: string | null;
        };
        Data_Vec_LostReasonResponse: {
            data: {
                created_at: string;
                id: string;
                is_active: boolean;
                /** @example Price */
                name: string;
                /** Format: int32 */
                position: number;
                updated_at: string;
            }[];
        };
        ReorderRequest: {
            ids: string[];
        };
        Data_LostReasonResponse: {
            data: {
                created_at: string;
                id: string;
                is_active: boolean;
                /** @example Price */
                name: string;
                /** Format: int32 */
                position: number;
                updated_at: string;
            };
        };
        LostReasonResponse: {
            created_at: string;
            id: string;
            is_active: boolean;
            /** @example Price */
            name: string;
            /** Format: int32 */
            position: number;
            updated_at: string;
        };
        CreateLostReasonRequest: {
            /** @example Delivery time */
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
        Page_LostReasonResponse: {
            data: {
                created_at: string;
                id: string;
                is_active: boolean;
                /** @example Price */
                name: string;
                /** Format: int32 */
                position: number;
                updated_at: string;
            }[];
            meta: components["schemas"]["PageMeta"];
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
    lost_reasons_list: {
        parameters: {
            query?: {
                include_inactive?: boolean;
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
            /** @description Lost reasons in order; inactive ones only with `include_inactive=true` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page_LostReasonResponse"];
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
            /** @description Missing `records.read` and `settings.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid parameters (`validation_error`) */
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
    lost_reasons_create: {
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
                "application/json": components["schemas"]["CreateLostReasonRequest"];
            };
        };
        responses: {
            /** @description Lost reason created at the end; publishes `lost_reason.created` */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_LostReasonResponse"];
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
            /** @description The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid name (`validation_error`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    lost_reasons_reorder: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ReorderRequest"];
            };
        };
        responses: {
            /** @description Lost reasons reordered; publishes `lost_reason.reordered` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_LostReasonResponse"];
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
            /** @description The ids must list every lost reason exactly once (`validation_error`) */
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
    lost_reasons_get: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Lost reason id */
                reason_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Lost reason */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_LostReasonResponse"];
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
            /** @description Missing `records.read` and `settings.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Lost reason not found (`lost_reason_not_found`) */
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
    lost_reasons_update: {
        parameters: {
            query?: never;
            header?: {
                /** @description `updated_at` of the lost reason as last read (ISO-8601); when it changed since, the update is refused with 409 `version_conflict` */
                "If-Match"?: string | null;
            };
            path: {
                /** @description Lost reason id */
                reason_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateLostReasonRequest"];
            };
        };
        responses: {
            /** @description Lost reason renamed, activated or deactivated (they are never deleted); publishes `lost_reason.updated` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_LostReasonResponse"];
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
            /** @description Lost reason not found (`lost_reason_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description At least one lost reason must stay active (`last_lost_reason`), or the lost reason changed after the `If-Match` version (`version_conflict`) */
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
    workflow_stages_list: {
        parameters: {
            query?: {
                pipeline_id?: string;
                include_inactive?: boolean;
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
            /** @description Stages of every pipeline (or of `pipeline_id`) in order; inactive ones only with `include_inactive=true`, filtered by name with `search` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page_PipelineStageResponse"];
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
            /** @description Missing `records.read` and `settings.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Pipeline not found (`pipeline_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid parameters (`validation_error`) */
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
    workflow_stages_get: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Stage id */
                stage_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Stage */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PipelineStageResponse"];
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
            /** @description Missing `records.read` and `settings.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Stage not found (`pipeline_stage_not_found`) */
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
    workflow_stages_delete: {
        parameters: {
            query?: {
                target_stage_id?: string;
            };
            header?: never;
            path: {
                /** @description Stage id */
                stage_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Stage deleted; its records (including deleted ones) move to `target_stage_id`. Publishes `pipeline_stage.deleted` */
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
            /** @description Missing `settings.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Stage not found (`pipeline_stage_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The stage has records and no target was given (`stage_has_records`), or the pipeline would lose its last open, won or lost stage (`closing_stages_required`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid target stage (`validation_error`) */
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
    workflow_stages_update: {
        parameters: {
            query?: never;
            header?: {
                /** @description `updated_at` of the stage as last read (ISO-8601); when it changed since, the update is refused with 409 `version_conflict` */
                "If-Match"?: string | null;
            };
            path: {
                /** @description Stage id */
                stage_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateStageRequest"];
            };
        };
        responses: {
            /** @description Stage renamed or changed, including the fields required to enter (`entry_required_fields`) or leave it (`exit_required_fields`): editable fields declared by staged record providers. `rotting_days` (1 to 3650, `null` clears it, open stages only) marks records that stay longer in the stage as rotting. Publishes `pipeline_stage.updated` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PipelineStageResponse"];
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
            /** @description Stage not found (`pipeline_stage_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The kind of a stage with records cannot change (`stage_has_records`), or the pipeline would lose its last open, won or lost stage (`closing_stages_required`), or the stage changed after the `If-Match` version (`version_conflict`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid name, kind, probability or required fields (`validation_error`) */
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
    workflow_stages_activate: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Stage id */
                stage_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Stage active again; publishes `pipeline_stage.updated` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PipelineStageResponse"];
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
            /** @description Stage not found (`pipeline_stage_not_found`) */
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
    workflow_stages_deactivate: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Stage id */
                stage_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["StageRemovalRequest"];
            };
        };
        responses: {
            /** @description Stage deactivated; its records move to `target_stage_id` (another active stage of the same kind). Publishes `pipeline_stage.deactivated` and `record.stage_changed` for each moved record */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PipelineStageResponse"];
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
            /** @description Stage not found (`pipeline_stage_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The stage has records and no target was given (`stage_has_records`), or the pipeline would lose its last open, won or lost stage (`closing_stages_required`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid target stage (`validation_error`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    workflow_stage_guidance_get: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Pipeline stage id */
                stage_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Guidance and checklist of the stage (empty while never configured) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_StageGuidanceResponse"];
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
            /** @description Missing `records.read` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Stage not found (`pipeline_stage_not_found`) */
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
    workflow_stage_guidance_update: {
        parameters: {
            query?: never;
            header?: {
                /** @description `updated_at` of the guidance as read (ISO-8601); ignored while never saved */
                "If-Match"?: string | null;
            };
            path: {
                /** @description Pipeline stage id */
                stage_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["StageGuidanceRequest"];
            };
        };
        responses: {
            /** @description Guidance saved */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_StageGuidanceResponse"];
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
            /** @description Stage not found (`pipeline_stage_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The guidance changed since the `If-Match` version (`version_conflict`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Guidance too long, too many items, invalid or duplicated keys, invalid labels (`validation_error` with `details.fields`) */
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
    pipelines_list: {
        parameters: {
            query?: {
                page?: number;
                per_page?: number;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Pipelines of the organization in order with their stages (active and inactive) and the number of records in each stage. Pipelines restricted to teams (`team_ids`) only show up for members of those teams (sub-teams included) and for users with `settings.manage` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page_PipelineResponse"];
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
            /** @description Missing `records.read` and `settings.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid pagination (`validation_error`) */
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
    pipelines_create: {
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
                "application/json": components["schemas"]["CreatePipelineRequest"];
            };
        };
        responses: {
            /** @description Pipeline created at the end of the list with the given stages (or the default ones) and teams; the first pipeline of an organization becomes the default. Publishes `pipeline.created` */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PipelineResponse"];
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
            /** @description The stages lack an open, won or lost stage (`closing_stages_required`), or the organization reached the limit of workflow (`pipeline_limit_reached`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid name, teams or stages (`validation_error`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    pipelines_reorder: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ReorderRequest"];
            };
        };
        responses: {
            /** @description Pipelines reordered; publishes `pipeline.reordered` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_PipelineResponse"];
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
            /** @description The ids must list every pipeline exactly once (`validation_error`) */
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
    record_checklist_get: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Staged entity key */
                record_type: string;
                /** @description Record id */
                record_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Checklist of the current stage */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_RecordChecklistResponse"];
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
            /** @description Missing `records.read` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Record not found or not visible (`record_not_found`) */
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
    record_checklist_item_update: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Staged entity key */
                record_type: string;
                /** @description Record id */
                record_id: string;
                /** @description Checklist item id */
                item_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ToggleChecklistItemRequest"];
            };
        };
        responses: {
            /** @description Checklist of the current stage after the change */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_RecordChecklistResponse"];
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
            /** @description Missing `records.update` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Record not found or not editable (`record_not_found`) or item not in the record's pipeline (`checklist_item_not_found`) */
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
    workflow_required_fields: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Editable fields from staged entity providers */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_StageFieldChoice"];
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
    pipelines_get: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Pipeline id */
                pipeline_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Pipeline with its stages */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PipelineResponse"];
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
            /** @description Missing `records.read` and `settings.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Pipeline not found or restricted to other teams (`pipeline_not_found`) */
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
    pipelines_delete: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Pipeline id */
                pipeline_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["PipelineRemovalRequest"];
            };
        };
        responses: {
            /** @description Pipeline deleted with its stages; its records (including deleted ones) move to `target_pipeline_id`, each stage to `stage_mapping[stage_id]` or to the first active stage of the same kind. Publishes `pipeline.deleted` and `record.pipeline_changed`/`record.stage_changed` for each moved record */
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
            /** @description Missing `settings.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Pipeline not found (`pipeline_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The default pipeline cannot be deleted (`default_pipeline_required`), or the pipeline has records and no target was given (`pipeline_has_records`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid target pipeline or stage mapping (`validation_error`) */
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
    pipelines_update: {
        parameters: {
            query?: never;
            header?: {
                /** @description `updated_at` of the pipeline as last read (ISO-8601); when it changed since, the update is refused with 409 `version_conflict` */
                "If-Match"?: string | null;
            };
            path: {
                /** @description Pipeline id */
                pipeline_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdatePipelineRequest"];
            };
        };
        responses: {
            /** @description Pipeline renamed or restricted to teams (`team_ids`, empty for everyone); publishes `pipeline.updated` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PipelineResponse"];
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
            /** @description Pipeline not found (`pipeline_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The pipeline changed after the `If-Match` version (`version_conflict`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid name or teams (`validation_error`) */
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
    pipelines_activate: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Pipeline id */
                pipeline_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Pipeline active again; publishes `pipeline.activated` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PipelineResponse"];
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
            /** @description Pipeline not found (`pipeline_not_found`) */
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
    pipelines_deactivate: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Pipeline id */
                pipeline_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Pipeline deactivated: its records stay where they are, but no record can be created in or moved into it. Publishes `pipeline.deactivated` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PipelineResponse"];
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
            /** @description Pipeline not found (`pipeline_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The default pipeline cannot be deactivated (`default_pipeline_required`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
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
    pipelines_set_default: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Pipeline id */
                pipeline_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Pipeline is now the default one (Kanban and new records without a pipeline); publishes `pipeline.default_changed` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PipelineResponse"];
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
            /** @description Pipeline not found (`pipeline_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description An inactive pipeline cannot be the default one (`default_pipeline_required`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
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
    pipelines_create_stage: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Pipeline id */
                pipeline_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateStageRequest"];
            };
        };
        responses: {
            /** @description Stage created; open stages go after the last open stage, won and lost stages at the end. `entry_required_fields`/`exit_required_fields` list record fields that must be filled to enter or leave it. Publishes `pipeline_stage.created` */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PipelineStageResponse"];
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
            /** @description Pipeline not found (`pipeline_not_found`) */
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
            /** @description Invalid name, kind, probability or required fields (`validation_error`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    pipelines_reorder_stages: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Pipeline id */
                pipeline_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ReorderRequest"];
            };
        };
        responses: {
            /** @description Stages reordered; publishes `pipeline_stage.reordered` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PipelineResponse"];
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
            /** @description Pipeline not found (`pipeline_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The ids must list every stage of the pipeline exactly once (`validation_error`) */
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

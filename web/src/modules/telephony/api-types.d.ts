export interface paths {
    "/api/v1/telephony": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["calls_log"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/telephony/events": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["calls_events"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/telephony/outcomes": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["calls_outcomes_list"];
        put?: never;
        post: operations["calls_outcomes_create"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/telephony/outcomes/order": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put: operations["calls_outcomes_reorder"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/telephony/outcomes/{outcome_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch: operations["calls_outcomes_update"];
        trace?: never;
    };
    "/api/v1/telephony/providers": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["calls_providers"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/telephony/report": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["calls_report"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/telephony/sessions": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["calls_sessions_list"];
        put?: never;
        post: operations["calls_sessions_start"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/telephony/sessions/{session_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["calls_sessions_get"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/telephony/sessions/{session_id}/calls": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["calls_sessions_log"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/telephony/sessions/{session_id}/cursor": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["calls_sessions_cursor"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/telephony/sessions/{session_id}/finish": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["calls_sessions_finish"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/telephony/sessions/{session_id}/items": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["calls_sessions_items"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/telephony/sessions/{session_id}/skip": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["calls_sessions_skip"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/telephony/start": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["calls_start"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/telephony/{activity_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch: operations["calls_update"];
        trace?: never;
    };
    "/api/v1/telephony/{activity_id}/recording": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["calls_recording"];
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
        SessionCallResponse: {
            activity: components["schemas"]["ActivityResponse"];
            session: components["schemas"]["CallSessionResponse"];
        };
        CallSessionResponse: {
            completed_at?: string | null;
            counts?: null | components["schemas"]["CallSessionCountsResponse"];
            created_at: string;
            current?: null | components["schemas"]["CallSessionItemResponse"];
            id: string;
            name: string;
            /** Format: int32 */
            position: number;
            script?: string | null;
            status: components["schemas"]["CallSessionStatus"];
            /** Format: int32 */
            total: number;
            updated_at: string;
            view_id?: string | null;
        };
        /** @enum {string} */
        CallSessionStatus: "active" | "completed";
        CallSessionItemResponse: {
            activity_id?: string | null;
            contact_id: string;
            contact_name?: string | null;
            handled_at?: string | null;
            outcome_key?: string | null;
            /** Format: int32 */
            position: number;
            status: components["schemas"]["CallSessionItemStatus"];
        };
        /** @enum {string} */
        CallSessionItemStatus: "pending" | "logged" | "skipped";
        CallSessionCountsResponse: {
            /** Format: int32 */
            logged: number;
            /** Format: int32 */
            pending: number;
            /** Format: int32 */
            skipped: number;
        };
        ActivityResponse: {
            activity_type: components["schemas"]["ActivityTypeSummaryResponse"];
            activity_type_id: string;
            completed_at?: string | null;
            created_at: string;
            created_by?: string | null;
            custom_fields: Record<string, never>;
            description?: string | null;
            due_at?: string | null;
            /** Format: int32 */
            duration_minutes?: number | null;
            id: string;
            links: components["schemas"]["LinkedRecordResponse"][];
            location?: null | components["schemas"]["ActivityLocationResponse"];
            /**
             * Format: int32
             * @example 12
             */
            location_accuracy_m?: number | null;
            location_captured_at?: string | null;
            /**
             * Format: double
             * @example -23.55052
             */
            location_latitude?: number | null;
            /**
             * Format: double
             * @example -46.633308
             */
            location_longitude?: number | null;
            owner?: null | components["schemas"]["UserReferenceResponse"];
            owner_id?: string | null;
            priority: components["schemas"]["ActivityPriority"];
            record?: null | components["schemas"]["LinkedRecordResponse"];
            record_id?: string | null;
            record_type?: string | null;
            recurrence?: null | components["schemas"]["ActivityRecurrenceResponse"];
            /** Format: int32 */
            reminder_minutes_before?: number | null;
            status: components["schemas"]["ActivityStatus"];
            title: string;
            updated_at: string;
        };
        /** @enum {string} */
        ActivityStatus: "open" | "done" | "canceled";
        ActivityRecurrenceResponse: {
            by_weekday: components["schemas"]["RecurrenceWeekday"][];
            /** Format: int32 */
            count?: number | null;
            ended: boolean;
            frequency: components["schemas"]["RecurrenceFrequency"];
            /** Format: int32 */
            index: number;
            /** Format: int32 */
            interval: number;
            /** Format: int32 */
            month_day?: number | null;
            /** @example FREQ=MONTHLY;INTERVAL=1;BYMONTHDAY=31 */
            rrule: string;
            series_id: string;
            until?: string | null;
        };
        /** @enum {string} */
        RecurrenceFrequency: "daily" | "weekly" | "monthly" | "yearly";
        /** @enum {string} */
        RecurrenceWeekday: "mo" | "tu" | "we" | "th" | "fr" | "sa" | "su";
        /** @enum {string} */
        ActivityPriority: "low" | "normal" | "high" | "urgent";
        UserReferenceResponse: {
            display_name: string;
            id: string;
        };
        ActivityLocationResponse: {
            /**
             * Format: int32
             * @example 12
             */
            accuracy_m?: number | null;
            captured_at: string;
            /**
             * Format: double
             * @example -23.55052
             */
            latitude: number;
            /**
             * Format: double
             * @example -46.633308
             */
            longitude: number;
            /** @example https://www.openstreetmap.org/?mlat=-23.55052&mlon=-46.633308#map=17/-23.55052/-46.633308 */
            map_url: string;
        };
        LinkedRecordResponse: {
            record_id: string;
            record_type: string;
            title?: string | null;
        };
        ActivityTypeSummaryResponse: {
            color?: string | null;
            icon?: string | null;
            id: string;
            key: string;
            name: string;
        };
        CallReportResponse: {
            /** Format: int64 */
            calls: number;
            /** Format: int64 */
            connected: number;
            /** Format: int64 */
            duration_seconds: number;
            from: string;
            outcomes: components["schemas"]["CallOutcomeResponse"][];
            to: string;
            users: components["schemas"]["CallReportUserResponse"][];
        };
        CallReportUserResponse: {
            /** Format: int64 */
            calls: number;
            /** Format: int64 */
            connected: number;
            /** Format: int64 */
            duration_seconds: number;
            outcomes: components["schemas"]["CallReportOutcomeCountResponse"][];
            user?: null | components["schemas"]["UserReferenceResponse"];
        };
        CallReportOutcomeCountResponse: {
            /** Format: int64 */
            calls: number;
            outcome_key: string;
        };
        CallOutcomeResponse: {
            created_at: string;
            id: string;
            is_active: boolean;
            is_connected: boolean;
            is_system: boolean;
            /** @example answered */
            key: string;
            /** @example Answered */
            name: string;
            /** Format: int32 */
            position: number;
            updated_at: string;
        };
        CallRecordingResponse: {
            expires_at?: string | null;
            url: string;
        };
        StartedCallResponse: {
            provider: string;
            provider_call_id: string;
        };
        TelephonyProviderResponse: {
            /** @example voip.twilio */
            key: string;
            label: string;
        };
        ActivityCallResponse: {
            direction: components["schemas"]["CallDirection"];
            /** Format: int32 */
            duration_seconds?: number | null;
            has_recording: boolean;
            outcome: components["schemas"]["CallOutcomeSummaryResponse"];
            phone_number?: string | null;
            provider?: string | null;
            provider_call_id?: string | null;
            recording_url?: string | null;
            session_id?: string | null;
            updated_at: string;
        };
        CallOutcomeSummaryResponse: {
            is_connected: boolean;
            /** @example answered */
            key: string;
            /** @example Answered */
            name?: string | null;
        };
        /** @enum {string} */
        CallDirection: "inbound" | "outbound";
        Data_CallRecordingResponse: {
            data: {
                expires_at?: string | null;
                url: string;
            };
        };
        UpdateCallRequest: {
            direction?: null | components["schemas"]["CallDirection"];
            /** Format: int64 */
            duration_seconds?: number | null;
            outcome_key?: string | null;
            phone_number?: string | null;
            recording_url?: string | null;
        };
        Data_StartedCallResponse: {
            data: {
                provider: string;
                provider_call_id: string;
            };
        };
        StartCallRequest: {
            phone_number: string;
            provider: string;
            record_id?: string | null;
            record_type?: string | null;
        };
        SessionPositionRequest: {
            /** Format: int32 */
            position?: number | null;
        };
        Page_CallSessionItemResponse: {
            data: {
                activity_id?: string | null;
                contact_id: string;
                contact_name?: string | null;
                handled_at?: string | null;
                outcome_key?: string | null;
                /** Format: int32 */
                position: number;
                status: components["schemas"]["CallSessionItemStatus"];
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
        SessionCursorRequest: {
            /** Format: int32 */
            position: number;
        };
        Data_SessionCallResponse: {
            data: {
                activity: components["schemas"]["ActivityResponse"];
                session: components["schemas"]["CallSessionResponse"];
            };
        };
        SessionCallRequest: {
            custom_fields?: Record<string, never>;
            direction?: null | components["schemas"]["CallDirection"];
            /** Format: int64 */
            duration_seconds?: number | null;
            notes?: string | null;
            /** @example answered */
            outcome_key: string;
            phone_number?: string | null;
            /** Format: int32 */
            position?: number | null;
            provider?: string | null;
            provider_call_id?: string | null;
            recording_url?: string | null;
        };
        Data_CallSessionResponse: {
            data: {
                completed_at?: string | null;
                counts?: null | components["schemas"]["CallSessionCountsResponse"];
                created_at: string;
                current?: null | components["schemas"]["CallSessionItemResponse"];
                id: string;
                name: string;
                /** Format: int32 */
                position: number;
                script?: string | null;
                status: components["schemas"]["CallSessionStatus"];
                /** Format: int32 */
                total: number;
                updated_at: string;
                view_id?: string | null;
            };
        };
        StartCallSessionRequest: {
            name?: string | null;
            script?: string | null;
            view_id: string;
        };
        Page_CallSessionResponse: {
            data: {
                completed_at?: string | null;
                counts?: null | components["schemas"]["CallSessionCountsResponse"];
                created_at: string;
                current?: null | components["schemas"]["CallSessionItemResponse"];
                id: string;
                name: string;
                /** Format: int32 */
                position: number;
                script?: string | null;
                status: components["schemas"]["CallSessionStatus"];
                /** Format: int32 */
                total: number;
                updated_at: string;
                view_id?: string | null;
            }[];
            meta: components["schemas"]["PageMeta"];
        };
        Data_CallReportResponse: {
            data: {
                /** Format: int64 */
                calls: number;
                /** Format: int64 */
                connected: number;
                /** Format: int64 */
                duration_seconds: number;
                from: string;
                outcomes: components["schemas"]["CallOutcomeResponse"][];
                to: string;
                users: components["schemas"]["CallReportUserResponse"][];
            };
        };
        Data_Vec_TelephonyProviderResponse: {
            data: {
                /** @example voip.twilio */
                key: string;
                label: string;
            }[];
        };
        UpdateCallOutcomeRequest: {
            is_active?: boolean | null;
            is_connected?: boolean | null;
            name?: string | null;
        };
        ReorderCallOutcomesRequest: {
            ids: string[];
        };
        Data_CallOutcomeResponse: {
            data: {
                created_at: string;
                id: string;
                is_active: boolean;
                is_connected: boolean;
                is_system: boolean;
                /** @example answered */
                key: string;
                /** @example Answered */
                name: string;
                /** Format: int32 */
                position: number;
                updated_at: string;
            };
        };
        CreateCallOutcomeRequest: {
            is_connected?: boolean;
            /** @example callback_requested */
            key?: string | null;
            /** @example Requested callback */
            name: string;
        };
        Data_Vec_CallOutcomeResponse: {
            data: {
                created_at: string;
                id: string;
                is_active: boolean;
                is_connected: boolean;
                is_system: boolean;
                /** @example answered */
                key: string;
                /** @example Answered */
                name: string;
                /** Format: int32 */
                position: number;
                updated_at: string;
            }[];
        };
        CallEventRequest: {
            direction?: null | components["schemas"]["CallDirection"];
            /** Format: int64 */
            duration_seconds?: number | null;
            occurred_at?: string | null;
            outcome_key?: string | null;
            phone_number?: string | null;
            /** @example voip.twilio */
            provider: string;
            provider_call_id: string;
            record_id?: string | null;
            record_type?: string | null;
            recording_url?: string | null;
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
        Data_CallActivityResponse: {
            data: components["schemas"]["ActivityResponse"] & {
                call?: null | components["schemas"]["ActivityCallResponse"];
            };
        };
        CallActivityResponse: components["schemas"]["ActivityResponse"] & {
            call?: null | components["schemas"]["ActivityCallResponse"];
        };
        LogCallRequest: {
            custom_fields?: Record<string, never>;
            direction?: null | components["schemas"]["CallDirection"];
            /**
             * Format: int64
             * @example 95
             */
            duration_seconds?: number | null;
            notes?: string | null;
            occurred_at?: string | null;
            /** @example answered */
            outcome_key: string;
            /** @example +55 11 99999-0000 */
            phone_number?: string | null;
            provider?: string | null;
            provider_call_id?: string | null;
            record_id?: string | null;
            /** @example people.person */
            record_type?: string | null;
            recording_url?: string | null;
            title?: string | null;
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
    calls_log: {
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
                "application/json": components["schemas"]["LogCallRequest"];
            };
        };
        responses: {
            /** @description Call logged as a done activity of the `call` type with the call details in `call` (`activity.created` and `call.logged`). `direction` defaults to `outbound`; `outcome_key` is one of the active call outcomes of the organization; `duration_seconds` (0–86400) also fills `duration_minutes`, rounded up; `notes` become the description; `occurred_at` is the completion time (default now); `title` defaults to `<call type> · <outcome>`. `provider` + `provider_call_id` tie the call to a telephony module (unique) */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_CallActivityResponse"];
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
            /** @description Missing `tasks.create` (`permission_denied`, `insufficient_scope`) */
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
            /** @description Invalid fields, unknown or inactive outcome, unreadable record (`validation_error`) or no call activity type (`call_activity_type_missing`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    calls_events: {
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
                "application/json": components["schemas"]["CallEventRequest"];
            };
        };
        responses: {
            /** @description Call event of a telephony module (`provider` is `<module>.<provider>`, enabled for the organization). The first event of a `provider_call_id` logs the call as the acting user (`call.logged`; without `outcome_key` the first active outcome that is connected when `duration_seconds` > 0, or not connected otherwise); the next ones update its details (`call.updated`). Modules send it with `report_call_event` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_CallActivityResponse"];
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
            /** @description Missing `tasks.create` (or `tasks.update` for an existing call) (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown or disabled telephony provider (`telephony_provider_not_found`) */
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
            /** @description Invalid fields (`validation_error`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    calls_outcomes_list: {
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
            /** @description Call outcomes of the organization in their order (active only unless `include_inactive=true`). The system ones (`answered`, `voicemail`, `no_answer`, `wrong_number`) are created with the organization */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_CallOutcomeResponse"];
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
            /** @description Missing `tasks.read` or `settings.manage` (`permission_denied`, `insufficient_scope`) */
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
    calls_outcomes_create: {
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
                "application/json": components["schemas"]["CreateCallOutcomeRequest"];
            };
        };
        responses: {
            /** @description Call outcome created at the end of the list (`call_outcome.created`); `key` defaults to one derived from the name. `is_connected` marks outcomes where the person was reached (counted as connected in the report) */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_CallOutcomeResponse"];
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
            /** @description Another outcome uses the key (`call_outcome_key_taken`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid name or key, or the limit of 50 outcomes was reached (`validation_error`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    calls_outcomes_reorder: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ReorderCallOutcomesRequest"];
            };
        };
        responses: {
            /** @description Outcomes in the new order (`call_outcome.reordered`); `ids` lists every outcome, active or not, exactly once */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_CallOutcomeResponse"];
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
            /** @description The list does not match the outcomes of the organization (`validation_error`) */
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
    calls_outcomes_update: {
        parameters: {
            query?: never;
            header?: {
                /** @description `updated_at` of the outcome as last read (ISO-8601); when it changed since, the update is refused with 409 `version_conflict` */
                "If-Match"?: string | null;
            };
            path: {
                /** @description Call outcome id */
                outcome_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateCallOutcomeRequest"];
            };
        };
        responses: {
            /** @description Call outcome updated (`call_outcome.updated`); the key never changes. Inactive outcomes stay on the calls already logged but cannot be picked for new ones */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_CallOutcomeResponse"];
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
            /** @description Outcome not found (`call_outcome_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Deactivating the last active outcome (`last_active_call_outcome`) or the outcome changed after the `If-Match` version (`version_conflict`) */
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
    calls_providers: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Telephony providers of the modules enabled for the organization. Without any, the web app dials with `tel:` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_TelephonyProviderResponse"];
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
            /** @description Missing `tasks.read` (`permission_denied`, `insufficient_scope`) */
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
    calls_report: {
        parameters: {
            query?: {
                /** @example 2026-10-01 */
                from?: string;
                /** @example 2026-10-31 */
                to?: string;
                /** @example me */
                owner_id?: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Logged calls per owner and outcome in the period (`from` inclusive, `to` exclusive; dates `YYYY-MM-DD` in the organization time zone — `to` includes the whole day — or RFC 3339 instants; default the last 30 days, at most 366 days), only over the call tasks the user can read (`tasks.read` scope). `owner_id=me` keeps the user's calls. `connected` counts the outcomes marked as connected */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_CallReportResponse"];
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
            /** @description Missing `tasks.read` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid period or owner (`validation_error`) */
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
    calls_sessions_list: {
        parameters: {
            query?: {
                active?: boolean;
                page?: number;
                per_page?: number;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Call sessions of the user, newest first (`active=true` keeps the ones still in progress) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page_CallSessionResponse"];
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
            /** @description Missing `tasks.read` (`permission_denied`, `insufficient_scope`) */
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
    calls_sessions_start: {
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
                "application/json": components["schemas"]["StartCallSessionRequest"];
            };
        };
        responses: {
            /** @description Call session started from a saved view of people visible to the user (`call_session.started`): the people matching the view filter and sort under the user's read scope (at most 500) become the queue, kept for the session. `script` defaults to the `call_script` text in the view settings; `name` to the view name */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_CallSessionResponse"];
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
            /** @description Missing `tasks.create` or `people.read` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description View not found, not visible or not of people (`call_session_view_not_found`) */
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
            /** @description The view has no people (`call_session_empty`) or invalid fields (`validation_error`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    calls_sessions_get: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Call session id */
                session_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Session with counts and the `current` contact of the queue (null once completed) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_CallSessionResponse"];
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
            /** @description Missing `tasks.read` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Session not found or of another user (`call_session_not_found`) */
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
    calls_sessions_log: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Call session id */
                session_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SessionCallRequest"];
            };
        };
        responses: {
            /** @description Call logged for the contact at `position` (default the current one), in the same transaction that marks it as `logged` and moves the session to the next pending contact (after it, then from the start). When no pending contact remains the session is completed (`call_session.completed`) */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_SessionCallResponse"];
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
            /** @description Missing `tasks.create` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Session not found (`call_session_not_found`) or no contact at the position (`call_session_item_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The session is completed (`call_session_completed`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid fields, unknown outcome or the contact is no longer readable (`validation_error`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    calls_sessions_cursor: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Call session id */
                session_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SessionCursorRequest"];
            };
        };
        responses: {
            /** @description Session moved to the contact at `position` (go back or jump ahead); a contact already logged can be called again */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_CallSessionResponse"];
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
            /** @description Missing `tasks.create` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Session not found (`call_session_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The session is completed (`call_session_completed`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Position outside the queue (`validation_error`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    calls_sessions_finish: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Call session id */
                session_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Session completed before the end of the queue (`call_session.completed`) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_CallSessionResponse"];
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
            /** @description Missing `tasks.create` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Session not found (`call_session_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The session is already completed (`call_session_completed`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
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
    calls_sessions_items: {
        parameters: {
            query?: {
                page?: number;
                per_page?: number;
            };
            header?: never;
            path: {
                /** @description Call session id */
                session_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Queue of the session in order, with the status of each contact (`pending`, `logged`, `skipped`) and the last call logged for it */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page_CallSessionItemResponse"];
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
            /** @description Missing `tasks.read` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Session not found or of another user (`call_session_not_found`) */
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
    calls_sessions_skip: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Call session id */
                session_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SessionPositionRequest"];
            };
        };
        responses: {
            /** @description Contact at `position` (default the current one) marked as `skipped` when still pending, and the session moves to the next pending contact; completed when none remains */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_CallSessionResponse"];
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
            /** @description Missing `tasks.create` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Session not found (`call_session_not_found`) or no contact at the position (`call_session_item_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The session is completed (`call_session_completed`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
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
    calls_start: {
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
                "application/json": components["schemas"]["StartCallRequest"];
            };
        };
        responses: {
            /** @description Call started by the telephony provider (click to call). Nothing is logged yet: the provider reports the call with `POST /telephony/events` using the returned `provider_call_id`, or the user logs it with `POST /telephony` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_StartedCallResponse"];
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
            /** @description Missing `tasks.create` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown or disabled telephony provider (`telephony_provider_not_found`) */
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
            /** @description Missing phone number or unreadable record (`validation_error`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    calls_update: {
        parameters: {
            query?: never;
            header?: {
                /** @description `call.updated_at` as last read (ISO-8601); when it changed since, the update is refused with 409 `version_conflict` */
                "If-Match"?: string | null;
            };
            path: {
                /** @description Id of the call activity */
                activity_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateCallRequest"];
            };
        };
        responses: {
            /** @description Call details updated (`call.updated` with `changes`); only the sent fields change, `null` clears duration, phone number or recording */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_CallActivityResponse"];
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
            /** @description Missing `tasks.update` for this activity (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Activity not found (`activity_not_found`) or it is not a logged call (`call_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The call changed after the `If-Match` version (`version_conflict`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid fields or unknown outcome (`validation_error`) */
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
    calls_recording: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Id of the call activity */
                activity_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Address of the call recording: asked to the telephony provider of the call when it is enabled (it may expire at `expires_at`), otherwise the stored `recording_url` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_CallRecordingResponse"];
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
            /** @description Missing `tasks.read` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Activity not found (`activity_not_found`), not a call (`call_not_found`) or without recording (`call_recording_not_found`) */
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

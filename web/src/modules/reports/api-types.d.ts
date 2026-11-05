export interface paths {
    "/api/v1/reports": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["reports_list"];
        put?: never;
        /** @description Saves a custom report. `definition` is the same body of `POST /aggregates` (record type, filter, groups, metrics, period, comparison, sort and limit) and is validated by running it under the permissions of the caller, so unknown or hidden fields answer 422 with paths under `definition.`. Visibility follows the saved views: `organization` needs `settings.manage` and `team` needs membership of the team (or `settings.manage`). */
        post: operations["reports_create"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/reports/entities": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["reports_entities"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/reports/{report_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["reports_get"];
        put?: never;
        post?: never;
        delete: operations["reports_delete"];
        options?: never;
        head?: never;
        patch: operations["reports_update"];
        trace?: never;
    };
    "/api/v1/reports/{report_id}/export": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Runs the saved report like `POST /reports/{report_id}/run` and returns the result as a CSV (`;` separated, UTF-8 with BOM) or XLSX file: one column per group, then each metric (with previous value and variation when comparing) and a total row. */
        get: operations["reports_export"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/reports/{report_id}/run": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Runs the saved report under the read scope and field permissions of the caller, exactly like `POST /aggregates`. `preset` (or `from`/`to`, which imply `custom`) replaces the period of the report on its own period field; `compare_to_previous` replaces the comparison flag. */
        post: operations["reports_run"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/reports/{report_id}/schedules": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["reports_schedules_list"];
        put?: never;
        /** @description Schedules the report to be e-mailed `daily`, `weekly` (`weekday` 1 = Monday … 7 = Sunday) or `monthly` (`day_of_month`, the last day when the month is shorter) at `hour`:`minute` in the time zone of the organization. Every recipient must be an active member who can see the report; each one receives the result computed with their own permissions. `period_preset` replaces the period of the report on each delivery. */
        post: operations["reports_schedules_create"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/reports/{report_id}/schedules/{schedule_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        delete: operations["reports_schedules_delete"];
        options?: never;
        head?: never;
        patch: operations["reports_schedules_update"];
        trace?: never;
    };
    "/api/v1/reports/{report_id}/schedules/{schedule_id}/deliveries": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["reports_schedules_deliveries"];
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
        ReportEntityResponse: {
            labels: {
                [key: string]: string;
            };
            record_type: string;
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
        /** @enum {string} */
        DashboardPeriodPreset: "today" | "yesterday" | "last_7_days" | "last_30_days" | "week" | "last_week" | "month" | "last_month" | "quarter" | "last_quarter" | "year" | "last_year" | "custom";
        ReportDeliveryResponse: {
            failure_reason?: string | null;
            finished_at?: string | null;
            id: string;
            recipient?: null | components["schemas"]["UserReferenceResponse"];
            /** Format: int32 */
            row_count?: number | null;
            schedule_id: string;
            /** @example 2026-11-01T11:00:00Z */
            scheduled_for: string;
            status: components["schemas"]["ReportDeliveryStatus"];
            user_id: string;
        };
        /** @enum {string} */
        ReportDeliveryStatus: "sending" | "sent" | "failed";
        UserReferenceResponse: {
            display_name: string;
            id: string;
        };
        /** @enum {string} */
        ReportScheduleFrequency: "daily" | "weekly" | "monthly";
        ReportScheduleResponse: {
            /** @example 2026-10-20T13:45:00Z */
            created_at: string;
            created_by?: string | null;
            /** Format: int32 */
            day_of_month?: number | null;
            frequency: components["schemas"]["ReportScheduleFrequency"];
            /** Format: int32 */
            hour: number;
            id: string;
            is_active: boolean;
            last_run_at?: string | null;
            /** Format: int32 */
            minute: number;
            /** @example 2026-11-01T11:00:00Z */
            next_run_at: string;
            period_preset?: string | null;
            recipients: components["schemas"]["UserReferenceResponse"][];
            report_id: string;
            /** @example 2026-10-20T13:45:00Z */
            updated_at: string;
            /** Format: int32 */
            weekday?: number | null;
        };
        /** @enum {string} */
        ViewVisibility: "private" | "team" | "organization";
        /** @enum {string} */
        ReportVisualization: "table" | "bar" | "line" | "pie" | "number";
        ReportResponse: {
            can_edit: boolean;
            /** @example 2026-10-20T13:45:00Z */
            created_at: string;
            definition: components["schemas"]["AggregateQueryRequest"];
            description?: string | null;
            id: string;
            name: string;
            owner_id?: string | null;
            /** @example deals.deal */
            record_type: string;
            settings: Record<string, never>;
            team_id?: string | null;
            /** @example 2026-10-20T13:45:00Z */
            updated_at: string;
            visibility: components["schemas"]["ViewVisibility"];
            visualization: components["schemas"]["ReportVisualization"];
        };
        AggregateQueryRequest: {
            /** @example exclude */
            archived?: string | null;
            compare_to_previous?: boolean;
            filter?: null | components["schemas"]["FilterNode"];
            group_by?: components["schemas"]["AggregateGroupBy"][];
            /**
             * Format: int32
             * @example 500
             */
            limit?: number | null;
            metrics?: components["schemas"]["AggregateMetric"][] | null;
            period?: null | components["schemas"]["AggregatePeriod"];
            /** @example deals.deal */
            record_type: string;
            sort?: components["schemas"]["AggregateSort"][];
        };
        AggregateSort: {
            direction?: null | components["schemas"]["SortDirection"];
            /** @example sum:amount */
            key: string;
        };
        /** @enum {string} */
        SortDirection: "asc" | "desc";
        AggregatePeriod: {
            /** @example closed_at */
            field: string;
            /** @example 2026-01-01 */
            from?: string | null;
            /** @example month */
            preset?: string | null;
            /** @example 2026-03-31 */
            to?: string | null;
        };
        AggregateMetric: {
            /** @example amount */
            field?: string | null;
            fn: components["schemas"]["AggregateFunction"];
            /** @example closed_at */
            to?: string | null;
            unit?: null | components["schemas"]["AggregateDurationUnit"];
        };
        /** @enum {string} */
        AggregateDurationUnit: "days" | "hours";
        /** @enum {string} */
        AggregateFunction: "count" | "sum" | "avg" | "min" | "max";
        AggregateGroupBy: {
            /** @example closed_at */
            field: string;
            granularity?: null | components["schemas"]["TimeGranularity"];
        };
        /** @enum {string} */
        TimeGranularity: "day" | "week" | "month" | "quarter" | "year";
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
        Page_ReportDeliveryResponse: {
            data: {
                failure_reason?: string | null;
                finished_at?: string | null;
                id: string;
                recipient?: null | components["schemas"]["UserReferenceResponse"];
                /** Format: int32 */
                row_count?: number | null;
                schedule_id: string;
                /** @example 2026-11-01T11:00:00Z */
                scheduled_for: string;
                status: components["schemas"]["ReportDeliveryStatus"];
                user_id: string;
            }[];
            meta: components["schemas"]["PageMeta"];
        };
        UpdateScheduleRequest: {
            /** Format: int32 */
            day_of_month?: number | null;
            frequency?: null | components["schemas"]["ReportScheduleFrequency"];
            /** Format: int32 */
            hour?: number | null;
            is_active?: boolean | null;
            /** Format: int32 */
            minute?: number | null;
            period_preset?: string | null;
            recipients?: string[] | null;
            /** Format: int32 */
            weekday?: number | null;
        };
        Data_ReportScheduleResponse: {
            data: {
                /** @example 2026-10-20T13:45:00Z */
                created_at: string;
                created_by?: string | null;
                /** Format: int32 */
                day_of_month?: number | null;
                frequency: components["schemas"]["ReportScheduleFrequency"];
                /** Format: int32 */
                hour: number;
                id: string;
                is_active: boolean;
                last_run_at?: string | null;
                /** Format: int32 */
                minute: number;
                /** @example 2026-11-01T11:00:00Z */
                next_run_at: string;
                period_preset?: string | null;
                recipients: components["schemas"]["UserReferenceResponse"][];
                report_id: string;
                /** @example 2026-10-20T13:45:00Z */
                updated_at: string;
                /** Format: int32 */
                weekday?: number | null;
            };
        };
        CreateScheduleRequest: {
            /**
             * Format: int32
             * @example 1
             */
            day_of_month?: number | null;
            frequency: components["schemas"]["ReportScheduleFrequency"];
            /**
             * Format: int32
             * @example 8
             */
            hour: number;
            is_active?: boolean;
            /**
             * Format: int32
             * @example 0
             */
            minute?: number;
            /** @example last_month */
            period_preset?: string | null;
            recipients: string[];
            /**
             * Format: int32
             * @example 1
             */
            weekday?: number | null;
        };
        Page_ReportScheduleResponse: {
            data: {
                /** @example 2026-10-20T13:45:00Z */
                created_at: string;
                created_by?: string | null;
                /** Format: int32 */
                day_of_month?: number | null;
                frequency: components["schemas"]["ReportScheduleFrequency"];
                /** Format: int32 */
                hour: number;
                id: string;
                is_active: boolean;
                last_run_at?: string | null;
                /** Format: int32 */
                minute: number;
                /** @example 2026-11-01T11:00:00Z */
                next_run_at: string;
                period_preset?: string | null;
                recipients: components["schemas"]["UserReferenceResponse"][];
                report_id: string;
                /** @example 2026-10-20T13:45:00Z */
                updated_at: string;
                /** Format: int32 */
                weekday?: number | null;
            }[];
            meta: components["schemas"]["PageMeta"];
        };
        Data_AggregateResultResponse: {
            data: {
                groups: components["schemas"]["AggregateGroupColumnResponse"][];
                metrics: components["schemas"]["AggregateMetricColumnResponse"][];
                period?: null | components["schemas"]["AggregateRangeResponse"];
                previous_period?: null | components["schemas"]["AggregateRangeResponse"];
                /** @example deals.deal */
                record_type: string;
                rows: components["schemas"]["AggregateRowResponse"][];
                /** @example America/Sao_Paulo */
                timezone: string;
                totals: components["schemas"]["AggregateTotalsResponse"];
                truncated: boolean;
            };
        };
        AggregateTotalsResponse: {
            previous?: (string | null)[] | null;
            values: (string | null)[];
        };
        AggregateRowResponse: {
            groups: components["schemas"]["AggregateGroupValueResponse"][];
            previous?: (string | null)[] | null;
            /**
             * @example [
             *       "12",
             *       "45000.5"
             *     ]
             */
            values: (string | null)[];
        };
        AggregateGroupValueResponse: {
            /** @example Ana Souza */
            label?: string | null;
            /** @example 2026-10-01 */
            value?: string | null;
        };
        AggregateRangeResponse: {
            /** @example closed_at */
            field: string;
            /** @example 2026-10-01 */
            from: string;
            /** @example month */
            preset: string;
            /** @example 2026-10-31 */
            to: string;
        };
        AggregateMetricColumnResponse: {
            /** @example amount */
            field?: string | null;
            /** @example Amount */
            field_label?: string | null;
            field_type?: null | components["schemas"]["FieldType"];
            fn: components["schemas"]["AggregateFunction"];
            /** @example sum:amount */
            key: string;
            /** @example closed_at */
            to?: string | null;
            /** @example Closed at */
            to_label?: string | null;
            unit?: null | components["schemas"]["AggregateDurationUnit"];
        };
        FieldType: string;
        AggregateGroupColumnResponse: {
            field_type: components["schemas"]["FieldType"];
            granularity?: null | components["schemas"]["TimeGranularity"];
            /** @example closed_at */
            key: string;
            /** @example Closed at */
            label: string;
        };
        AggregateResultResponse: {
            groups: components["schemas"]["AggregateGroupColumnResponse"][];
            metrics: components["schemas"]["AggregateMetricColumnResponse"][];
            period?: null | components["schemas"]["AggregateRangeResponse"];
            previous_period?: null | components["schemas"]["AggregateRangeResponse"];
            /** @example deals.deal */
            record_type: string;
            rows: components["schemas"]["AggregateRowResponse"][];
            /** @example America/Sao_Paulo */
            timezone: string;
            totals: components["schemas"]["AggregateTotalsResponse"];
            truncated: boolean;
        };
        RunReportRequest: {
            compare_to_previous?: boolean | null;
            /** @example 2026-01-01 */
            from?: string | null;
            /** @example last_month */
            preset?: string | null;
            /** @example 2026-03-31 */
            to?: string | null;
        };
        UpdateReportRequest: {
            definition?: null | components["schemas"]["AggregateQueryRequest"];
            description?: string | null;
            name?: string | null;
            settings?: Record<string, never> | null;
            team_id?: string | null;
            visibility?: null | components["schemas"]["ViewVisibility"];
            visualization?: null | components["schemas"]["ReportVisualization"];
        };
        Data_Vec_ReportEntityResponse: {
            data: {
                labels: {
                    [key: string]: string;
                };
                record_type: string;
            }[];
        };
        Data_ReportResponse: {
            data: {
                can_edit: boolean;
                /** @example 2026-10-20T13:45:00Z */
                created_at: string;
                definition: components["schemas"]["AggregateQueryRequest"];
                description?: string | null;
                id: string;
                name: string;
                owner_id?: string | null;
                /** @example deals.deal */
                record_type: string;
                settings: Record<string, never>;
                team_id?: string | null;
                /** @example 2026-10-20T13:45:00Z */
                updated_at: string;
                visibility: components["schemas"]["ViewVisibility"];
                visualization: components["schemas"]["ReportVisualization"];
            };
        };
        CreateReportRequest: {
            definition: components["schemas"]["AggregateQueryRequest"];
            description?: string | null;
            /** @example Won amount per month by sales rep */
            name: string;
            settings?: Record<string, never> | null;
            team_id?: string | null;
            visibility?: null | components["schemas"]["ViewVisibility"];
            visualization?: null | components["schemas"]["ReportVisualization"];
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
        Page_ReportResponse: {
            data: {
                can_edit: boolean;
                /** @example 2026-10-20T13:45:00Z */
                created_at: string;
                definition: components["schemas"]["AggregateQueryRequest"];
                description?: string | null;
                id: string;
                name: string;
                owner_id?: string | null;
                /** @example deals.deal */
                record_type: string;
                settings: Record<string, never>;
                team_id?: string | null;
                /** @example 2026-10-20T13:45:00Z */
                updated_at: string;
                visibility: components["schemas"]["ViewVisibility"];
                visualization: components["schemas"]["ReportVisualization"];
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
    reports_list: {
        parameters: {
            query?: {
                /** @example deals.deal */
                record_type?: string;
                page?: number;
                per_page?: number;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Saved reports visible to the user (their own, the organization ones and those of their teams) over record types the user can read, ordered by name */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page_ReportResponse"];
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
            /** @description Missing the read permission of the requested record type (`permission_denied`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown record type (`record_type_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid query or pagination (`validation_error`) */
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
    reports_create: {
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
                "application/json": components["schemas"]["CreateReportRequest"];
            };
        };
        responses: {
            /** @description Report created */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_ReportResponse"];
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
            /** @description Missing the read permission of the record type, or sharing with an audience the user cannot reach (`permission_denied`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown record type (`record_type_not_found`) */
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
            /** @description Invalid name, visibility, team, visualization, settings or definition (`validation_error`, `field_not_accessible`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    reports_entities: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Readable entities offered by installed owners */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_ReportEntityResponse"];
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
    reports_get: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Report id */
                report_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description The saved report */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_ReportResponse"];
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
            /** @description Missing the read permission of the record type (`permission_denied`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The report does not exist or is not visible to the user (`report_not_found`) */
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
    reports_delete: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Report id */
                report_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Report deleted together with its schedules */
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
            /** @description Only the owner or someone with `settings.manage` can delete the report (`permission_denied`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The report does not exist or is not visible to the user (`report_not_found`) */
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
    reports_update: {
        parameters: {
            query?: never;
            header?: {
                /** @description `updated_at` of the report as last read; when it changed since, answers 409 `version_conflict` */
                "If-Match"?: string | null;
            };
            path: {
                /** @description Report id */
                report_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateReportRequest"];
            };
        };
        responses: {
            /** @description Report updated; omitted properties stay unchanged and a new `definition` is validated like on creation */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_ReportResponse"];
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
            /** @description Only the owner or someone with `settings.manage` can change the report (`permission_denied`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The report does not exist or is not visible to the user (`report_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The report changed since `If-Match` (`version_conflict`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid properties or definition (`validation_error`, `field_not_accessible`) */
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
    reports_export: {
        parameters: {
            query: {
                /** @example xlsx */
                format: string;
                /** @example last_month */
                preset?: string;
                /** @example 2026-01-01 */
                from?: string;
                /** @example 2026-03-31 */
                to?: string;
                compare_to_previous?: boolean;
            };
            header?: never;
            path: {
                /** @description Report id */
                report_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description The report result as an attachment */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/octet-stream": unknown;
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
            /** @description Missing the read permission of the record type (`permission_denied`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The report does not exist or is not visible to the user (`report_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown format or invalid override (`validation_error`, `report_period_required`, `field_not_accessible`) */
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
    reports_run: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Report id */
                report_id: string;
            };
            cookie?: never;
        };
        /** @description Period and comparison overrides; send `{}` to run the report as saved */
        requestBody: {
            content: {
                "application/json": components["schemas"]["RunReportRequest"];
            };
        };
        responses: {
            /** @description Aggregated rows, totals and the resolved periods */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_AggregateResultResponse"];
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
            /** @description Missing the read permission of the record type (`permission_denied`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The report does not exist or is not visible to the user (`report_not_found`) */
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
            /** @description Invalid override, a period override on a report without period (`report_period_required`), or a field hidden for the caller (`field_not_accessible`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    reports_schedules_list: {
        parameters: {
            query?: {
                page?: number;
                per_page?: number;
            };
            header?: never;
            path: {
                /** @description Report id */
                report_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description E-mail schedules of the report */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page_ReportScheduleResponse"];
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
            /** @description Only those who can edit the report see its schedules (`permission_denied`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The report does not exist or is not visible to the user (`report_not_found`) */
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
    reports_schedules_create: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Report id */
                report_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateScheduleRequest"];
            };
        };
        responses: {
            /** @description Schedule created with its next run */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_ReportScheduleResponse"];
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
            /** @description Only the owner of the report or someone with `settings.manage` can schedule it (`permission_denied`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The report does not exist or is not visible to the user (`report_not_found`) */
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
            /** @description Invalid frequency, time, period or recipients (`validation_error`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    reports_schedules_delete: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Report id */
                report_id: string;
                /** @description Schedule id */
                schedule_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Schedule deleted */
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
            /** @description Only the owner of the report or someone with `settings.manage` can delete it (`permission_denied`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown report or schedule (`report_not_found`, `report_schedule_not_found`) */
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
    reports_schedules_update: {
        parameters: {
            query?: never;
            header?: {
                /** @description `updated_at` of the schedule as last read; when it changed since, answers 409 `version_conflict` */
                "If-Match"?: string | null;
            };
            path: {
                /** @description Report id */
                report_id: string;
                /** @description Schedule id */
                schedule_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateScheduleRequest"];
            };
        };
        responses: {
            /** @description Schedule updated; changing the timing or reactivating it recomputes the next run */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_ReportScheduleResponse"];
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
            /** @description Only the owner of the report or someone with `settings.manage` can change it (`permission_denied`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown report or schedule (`report_not_found`, `report_schedule_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The schedule changed since `If-Match` (`version_conflict`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid frequency, time, period or recipients (`validation_error`) */
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
    reports_schedules_deliveries: {
        parameters: {
            query?: {
                page?: number;
                per_page?: number;
            };
            header?: never;
            path: {
                /** @description Report id */
                report_id: string;
                /** @description Schedule id */
                schedule_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Deliveries of the schedule, most recent first; `failed` ones carry the reason (recipient left the organization, lost access to the report or the record type, e-mail error) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page_ReportDeliveryResponse"];
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
            /** @description Only those who can edit the report see its deliveries (`permission_denied`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown report or schedule (`report_not_found`, `report_schedule_not_found`) */
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

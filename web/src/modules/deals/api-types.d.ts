export interface paths {
    "/api/v1/deal-member-roles": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["deal_member_roles_list"];
        put?: never;
        post: operations["deal_member_roles_create"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/deal-member-roles/order": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put: operations["deal_member_roles_reorder"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/deal-member-roles/{role_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        delete: operations["deal_member_roles_delete"];
        options?: never;
        head?: never;
        patch: operations["deal_member_roles_update"];
        trace?: never;
    };
    "/api/v1/deals": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["deals_list"];
        put?: never;
        post: operations["deals_create"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/deals/board": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["deals_board"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/deals/reports/conversion": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Conversion of a pipeline (default: the default active pipeline) for the deals created in the period, under the read scope of `deals.read` (`own` sees only their deals) and the field permissions of the role, in the time zone of the organization. `summary`: created, open, won and lost deals of the cohort, `conversion_rate` (won / created), `win_rate` (won / (won + lost)) and `average_cycle_seconds` (creation to closing of the won ones). `stages` (open stages in order): `reached` counts the deals that got to the stage or beyond (won deals reached every stage), `advanced` those that got to the next stage (the last one advances to won) and `passage_rate` = advanced / reached; `lost` counts the lost deals whose last open stage was this one (deals without stage history fall into `unattributed_lost`). Times come from the stage history (`deal_stage_history`, fed by `deal.created`/`deal.stage_changed` and backfilled from the event log): `average_seconds` over the visits that entered the stage in the period and already left it, `open_visits`/`open_average_seconds` for the deals still there. `lost_reasons` groups the lost deals of the cohort by reason. With `compare_to_previous=true`, `previous` is the same snapshot for the period right before. Amounts are decimal strings (null when the role cannot see `amount`). */
        get: operations["deals_reports_conversion"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/deals/reports/performance": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Sales performance grouped by `dimension` (`owner`, default; `team` of the owner, where an owner in two teams counts in both; `source`; `pipeline`), optionally limited to one pipeline, under the read scope of `deals.read` and `tasks.read` (`own` sees only their numbers) and the field permissions of the role. Per row and in `totals`: `created` (deals created in the period), `won`/`won_amount` and `lost` (closed in the period), `conversion_rate` (won / (won + lost)), `average_cycle_seconds` (creation to closing of the deals won in the period) and `activities_done` (tasks completed in the period, only for `owner`/`team` and with `tasks.read`; with `pipeline_id`, only tasks linked to deals of that pipeline). `key` is the user, team, source or pipeline id (null groups the deals without one) and `label` its name. Totals are computed apart, never by summing the rows. With `compare_to_previous=true`, every row and the totals carry `previous` for the period right before. */
        get: operations["deals_reports_performance"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/deals/{deal_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["deals_get"];
        put?: never;
        post?: never;
        delete: operations["deals_delete"];
        options?: never;
        head?: never;
        patch: operations["deals_update"];
        trace?: never;
    };
    "/api/v1/deals/{deal_id}/archive": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["deals_archive"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/deals/{deal_id}/lost": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["deals_lose"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/deals/{deal_id}/members": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Team of a deal (permission `deals.read` on the deal, including the pipeline restriction by team): the members with role and revenue credit, and the owner with the credit left to them. The credit of the members adds up to at most 100%; the owner receives the rest (`remaining_percent`), plus their own line when they are also a member. Members see the deal even with the `own` scope of `deals.read` (read only; editing follows the owner rule). */
        get: operations["deal_members_list"];
        put?: never;
        /** @description Adds a user to the team of a deal (permission `deals.update` on the deal) and publishes `deal.member_added`. `user_id` is an active member of the organization (the owner may be added too); `role_id` is an active deal member role (optional); `credit_percent` (0–100, 2 decimals, default 0) is taken from the share of the owner, and the members together cannot pass 100%. At most 50 members per deal. */
        post: operations["deal_members_create"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/deals/{deal_id}/members/{member_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** @description Removes a member from the team (permission `deals.update` on the deal) and publishes `deal.member_removed`; their credit returns to the owner. */
        delete: operations["deal_members_delete"];
        options?: never;
        head?: never;
        /** @description Changes the role or the credit of a member (permission `deals.update` on the deal) and publishes `deal.member_updated` with the changes; omitted properties stay unchanged and `role_id: null` clears the role. */
        patch: operations["deal_members_update"];
        trace?: never;
    };
    "/api/v1/deals/{deal_id}/move": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["deals_move"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/deals/{deal_id}/move-pipeline": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["deals_move_pipeline"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/deals/{deal_id}/reopen": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["deals_reopen"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/deals/{deal_id}/unarchive": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["deals_unarchive"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/deals/{deal_id}/won": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["deals_win"];
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
        PerformanceMetricsResponse: {
            /** Format: int64 */
            activities_done?: number | null;
            /**
             * Format: int64
             * @example 1296000
             */
            average_cycle_seconds?: number | null;
            /**
             * Format: double
             * @example 0.4
             */
            conversion_rate?: number | null;
            /** Format: int64 */
            created: number;
            /** Format: int64 */
            lost: number;
            /** Format: int64 */
            won: number;
            /** @example 42000.00 */
            won_amount?: string | null;
        };
        PerformanceRowResponse: {
            current: components["schemas"]["PerformanceMetricsResponse"];
            key?: string | null;
            label?: string | null;
            previous?: null | components["schemas"]["PerformanceMetricsResponse"];
        };
        PerformanceReportResponse: {
            activities_visible: boolean;
            amounts_visible: boolean;
            /** @example BRL */
            currency: string;
            dimension: components["schemas"]["PerformanceDimension"];
            period: components["schemas"]["DashboardPeriodResponse"];
            previous_period?: null | components["schemas"]["DashboardPeriodResponse"];
            previous_totals?: null | components["schemas"]["PerformanceMetricsResponse"];
            rows: components["schemas"]["PerformanceRowResponse"][];
            /** @example America/Sao_Paulo */
            timezone: string;
            totals: components["schemas"]["PerformanceMetricsResponse"];
            truncated: boolean;
        };
        DashboardPeriodResponse: {
            ends_at: string;
            /** @example 2026-04-01 */
            from: string;
            preset: components["schemas"]["DashboardPeriodPreset"];
            starts_at: string;
            /** @example 2026-04-30 */
            to: string;
        };
        /** @enum {string} */
        DashboardPeriodPreset: "today" | "yesterday" | "last_7_days" | "last_30_days" | "week" | "last_week" | "month" | "last_month" | "quarter" | "last_quarter" | "year" | "last_year" | "custom";
        /** @enum {string} */
        PerformanceDimension: "owner" | "team" | "source" | "pipeline";
        LostReasonTotalResponse: {
            amount?: string | null;
            /** Format: int64 */
            count: number;
            id?: string | null;
            name?: string | null;
        };
        ReportStageResponse: {
            id: string;
            kind: components["schemas"]["ReportStageKind"];
            name: string;
            /** Format: int32 */
            position: number;
            /** Format: int32 */
            probability: number;
        };
        /** @enum {string} */
        ReportStageKind: "open" | "won" | "lost";
        StageFunnelResponse: {
            /** Format: int64 */
            advanced: number;
            /**
             * Format: int64
             * @example 259200
             */
            average_seconds?: number | null;
            /** Format: int64 */
            completed_visits: number;
            /** Format: int64 */
            lost: number;
            lost_amount?: string | null;
            /** Format: int64 */
            open_average_seconds?: number | null;
            /** Format: int64 */
            open_visits: number;
            /**
             * Format: double
             * @example 0.6
             */
            passage_rate?: number | null;
            /** Format: int64 */
            reached: number;
            reached_amount?: string | null;
            stage_id: string;
        };
        ConversionSummaryResponse: {
            /**
             * Format: int64
             * @example 1296000
             */
            average_cycle_seconds?: number | null;
            /**
             * Format: double
             * @example 0.25
             */
            conversion_rate?: number | null;
            /** Format: int64 */
            created: number;
            /** @example 150000.00 */
            created_amount?: string | null;
            /** Format: int64 */
            lost: number;
            lost_amount?: string | null;
            /** Format: int64 */
            open: number;
            /**
             * Format: double
             * @example 0.4
             */
            win_rate?: number | null;
            /** Format: int64 */
            won: number;
            /** @example 42000.00 */
            won_amount?: string | null;
        };
        ConversionSnapshotResponse: {
            lost_reasons: components["schemas"]["LostReasonTotalResponse"][];
            period: components["schemas"]["DashboardPeriodResponse"];
            stages: components["schemas"]["StageFunnelResponse"][];
            summary: components["schemas"]["ConversionSummaryResponse"];
            /** Format: int64 */
            unattributed_lost: number;
            unattributed_lost_amount?: string | null;
        };
        ConversionReportResponse: {
            amounts_visible: boolean;
            /** @example BRL */
            currency: string;
            current?: null | components["schemas"]["ConversionSnapshotResponse"];
            pipeline?: null | components["schemas"]["ReportPipelineResponse"];
            previous?: null | components["schemas"]["ConversionSnapshotResponse"];
            /** @example America/Sao_Paulo */
            timezone: string;
        };
        ReportPipelineResponse: {
            id: string;
            name: string;
            stages: components["schemas"]["ReportStageResponse"][];
        };
        DealMemberRoleResponse: {
            created_at: string;
            id: string;
            is_active: boolean;
            /** @example pre_sales */
            key: string;
            /** @example Pre-sales */
            name: string;
            /** Format: int32 */
            position: number;
            updated_at: string;
        };
        DealTeamResponse: {
            /** @example 30.00 */
            allocated_percent: string;
            deal_id: string;
            members: components["schemas"]["DealMemberResponse"][];
            owner?: null | components["schemas"]["DealTeamOwnerResponse"];
            /** @example 70.00 */
            remaining_percent: string;
        };
        DealTeamOwnerResponse: {
            /** @example 70.00 */
            credit_percent: string;
            display_name: string;
            id: string;
        };
        DealMemberResponse: {
            added_by?: string | null;
            created_at: string;
            /** @example 30.00 */
            credit_percent: string;
            deal_id: string;
            id: string;
            role?: null | components["schemas"]["DealMemberRoleSummaryResponse"];
            updated_at: string;
            user: components["schemas"]["UserReferenceResponse"];
            user_active: boolean;
        };
        UserReferenceResponse: {
            display_name: string;
            id: string;
        };
        DealMemberRoleSummaryResponse: {
            id: string;
            /** @example pre_sales */
            key: string;
            /** @example Pre-sales */
            name: string;
        };
        BoardResponse: {
            columns: components["schemas"]["BoardColumnResponse"][];
            /** Format: int64 */
            page: number;
            /** Format: int64 */
            per_column: number;
            pipeline: components["schemas"]["PipelineSummaryResponse"];
        };
        PipelineSummaryResponse: {
            id: string;
            is_active: boolean;
            is_default: boolean;
            /** @example Sales pipeline */
            name: string;
        };
        BoardColumnResponse: {
            /** Format: int64 */
            count: number;
            has_more: boolean;
            items: components["schemas"]["DealResponse"][];
            stage: components["schemas"]["PipelineStageResponse"];
            /** @example 45000.00 */
            total_amount: string;
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
        /** @enum {string} */
        StageKind: "open" | "won" | "lost";
        DealResponse: {
            /** @example 15000.50 */
            amount?: string | null;
            amount_base?: string | null;
            archived_at?: string | null;
            arr?: string | null;
            closed_at?: string | null;
            company?: null | components["schemas"]["DealNamedResponse"];
            company_id?: string | null;
            contact?: null | components["schemas"]["DealNamedResponse"];
            contact_id?: string | null;
            contract_end?: string | null;
            contract_start?: string | null;
            created_at: string;
            created_by?: string | null;
            /** @example BRL */
            currency: string;
            custom_fields: Record<string, never>;
            /**
             * Format: int64
             * @example 12
             */
            days_in_stage: number;
            exchange_rate?: string | null;
            /** @example 2026-12-31 */
            expected_close_date?: string | null;
            id: string;
            is_rotting: boolean;
            last_activity_at?: string | null;
            lost_reason?: null | components["schemas"]["DealNamedResponse"];
            lost_reason_id?: string | null;
            lost_reason_note?: string | null;
            mrr?: string | null;
            owner?: null | components["schemas"]["DealOwnerResponse"];
            owner_id?: string | null;
            pipeline_id: string;
            /** Format: int32 */
            probability?: number | null;
            renewal_of_deal_id?: string | null;
            rotting_at?: string | null;
            source?: string | null;
            stage: components["schemas"]["DealStageResponse"];
            stage_entered_at: string;
            stage_id: string;
            status: components["schemas"]["DealStatus"];
            tcv?: string | null;
            /** @example Acme onboarding */
            title: string;
            updated_at: string;
        } & {
            [key: string]: unknown;
        };
        /** @enum {string} */
        DealStatus: "open" | "won" | "lost";
        DealStageResponse: {
            id: string;
            kind: components["schemas"]["StageKind"];
            name: string;
            /** Format: int32 */
            probability: number;
            /**
             * Format: int32
             * @example 7
             */
            rotting_days?: number | null;
        };
        DealOwnerResponse: {
            display_name: string;
            id: string;
        };
        DealNamedResponse: {
            id: string;
            name: string;
        };
        ReopenDealRequest: {
            stage_id?: string | null;
        };
        MoveDealPipelineRequest: {
            fields?: Record<string, never> | null;
            lost_reason_id?: string | null;
            lost_reason_note?: string | null;
            pipeline_id: string;
            stage_id?: string | null;
        };
        MoveDealRequest: {
            fields?: Record<string, never> | null;
            lost_reason_id?: string | null;
            lost_reason_note?: string | null;
            stage_id: string;
        };
        UpdateDealMemberRequest: {
            credit_percent?: null | components["schemas"]["DecimalInput"];
            role_id?: string | null;
        };
        /** @example 15000.00 */
        DecimalInput: string | number;
        Data_DealMemberResponse: {
            data: {
                added_by?: string | null;
                created_at: string;
                /** @example 30.00 */
                credit_percent: string;
                deal_id: string;
                id: string;
                role?: null | components["schemas"]["DealMemberRoleSummaryResponse"];
                updated_at: string;
                user: components["schemas"]["UserReferenceResponse"];
                user_active: boolean;
            };
        };
        AddDealMemberRequest: {
            credit_percent?: null | components["schemas"]["DecimalInput"];
            role_id?: string | null;
            user_id: string;
        };
        Data_DealTeamResponse: {
            data: {
                /** @example 30.00 */
                allocated_percent: string;
                deal_id: string;
                members: components["schemas"]["DealMemberResponse"][];
                owner?: null | components["schemas"]["DealTeamOwnerResponse"];
                /** @example 70.00 */
                remaining_percent: string;
            };
        };
        LoseDealRequest: {
            lost_reason_id?: string | null;
            lost_reason_note?: string | null;
        };
        Data_PerformanceReportResponse: {
            data: {
                activities_visible: boolean;
                amounts_visible: boolean;
                /** @example BRL */
                currency: string;
                dimension: components["schemas"]["PerformanceDimension"];
                period: components["schemas"]["DashboardPeriodResponse"];
                previous_period?: null | components["schemas"]["DashboardPeriodResponse"];
                previous_totals?: null | components["schemas"]["PerformanceMetricsResponse"];
                rows: components["schemas"]["PerformanceRowResponse"][];
                /** @example America/Sao_Paulo */
                timezone: string;
                totals: components["schemas"]["PerformanceMetricsResponse"];
                truncated: boolean;
            };
        };
        Data_ConversionReportResponse: {
            data: {
                amounts_visible: boolean;
                /** @example BRL */
                currency: string;
                current?: null | components["schemas"]["ConversionSnapshotResponse"];
                pipeline?: null | components["schemas"]["ReportPipelineResponse"];
                previous?: null | components["schemas"]["ConversionSnapshotResponse"];
                /** @example America/Sao_Paulo */
                timezone: string;
            };
        };
        Data_BoardResponse: {
            data: {
                columns: components["schemas"]["BoardColumnResponse"][];
                /** Format: int64 */
                page: number;
                /** Format: int64 */
                per_column: number;
                pipeline: components["schemas"]["PipelineSummaryResponse"];
            };
        };
        Data_DealResponse: {
            data: {
                /** @example 15000.50 */
                amount?: string | null;
                amount_base?: string | null;
                archived_at?: string | null;
                arr?: string | null;
                closed_at?: string | null;
                company?: null | components["schemas"]["DealNamedResponse"];
                company_id?: string | null;
                contact?: null | components["schemas"]["DealNamedResponse"];
                contact_id?: string | null;
                contract_end?: string | null;
                contract_start?: string | null;
                created_at: string;
                created_by?: string | null;
                /** @example BRL */
                currency: string;
                custom_fields: Record<string, never>;
                /**
                 * Format: int64
                 * @example 12
                 */
                days_in_stage: number;
                exchange_rate?: string | null;
                /** @example 2026-12-31 */
                expected_close_date?: string | null;
                id: string;
                is_rotting: boolean;
                last_activity_at?: string | null;
                lost_reason?: null | components["schemas"]["DealNamedResponse"];
                lost_reason_id?: string | null;
                lost_reason_note?: string | null;
                mrr?: string | null;
                owner?: null | components["schemas"]["DealOwnerResponse"];
                owner_id?: string | null;
                pipeline_id: string;
                /** Format: int32 */
                probability?: number | null;
                renewal_of_deal_id?: string | null;
                rotting_at?: string | null;
                source?: string | null;
                stage: components["schemas"]["DealStageResponse"];
                stage_entered_at: string;
                stage_id: string;
                status: components["schemas"]["DealStatus"];
                tcv?: string | null;
                /** @example Acme onboarding */
                title: string;
                updated_at: string;
            } & {
                [key: string]: unknown;
            };
        };
        DealRequest: {
            amount?: null | components["schemas"]["DecimalInput"];
            company_id?: string | null;
            contact_id?: string | null;
            /** @example 2027-10-31 */
            contract_end?: string | null;
            /** @example 2026-11-01 */
            contract_start?: string | null;
            /** @example BRL */
            currency?: string | null;
            custom_fields?: Record<string, never> | null;
            /** @example 2026-12-31 */
            expected_close_date?: string | null;
            lost_reason_id?: string | null;
            lost_reason_note?: string | null;
            owner_id?: string | null;
            pipeline_id?: string | null;
            /** Format: int64 */
            probability?: number | null;
            source?: string | null;
            stage_id?: string | null;
            status?: null | components["schemas"]["DealStatus"];
            /** @example Acme onboarding */
            title?: string | null;
        } & {
            [key: string]: unknown;
        };
        Page_DealResponse: {
            data: ({
                /** @example 15000.50 */
                amount?: string | null;
                amount_base?: string | null;
                archived_at?: string | null;
                arr?: string | null;
                closed_at?: string | null;
                company?: null | components["schemas"]["DealNamedResponse"];
                company_id?: string | null;
                contact?: null | components["schemas"]["DealNamedResponse"];
                contact_id?: string | null;
                contract_end?: string | null;
                contract_start?: string | null;
                created_at: string;
                created_by?: string | null;
                /** @example BRL */
                currency: string;
                custom_fields: Record<string, never>;
                /**
                 * Format: int64
                 * @example 12
                 */
                days_in_stage: number;
                exchange_rate?: string | null;
                /** @example 2026-12-31 */
                expected_close_date?: string | null;
                id: string;
                is_rotting: boolean;
                last_activity_at?: string | null;
                lost_reason?: null | components["schemas"]["DealNamedResponse"];
                lost_reason_id?: string | null;
                lost_reason_note?: string | null;
                mrr?: string | null;
                owner?: null | components["schemas"]["DealOwnerResponse"];
                owner_id?: string | null;
                pipeline_id: string;
                /** Format: int32 */
                probability?: number | null;
                renewal_of_deal_id?: string | null;
                rotting_at?: string | null;
                source?: string | null;
                stage: components["schemas"]["DealStageResponse"];
                stage_entered_at: string;
                stage_id: string;
                status: components["schemas"]["DealStatus"];
                tcv?: string | null;
                /** @example Acme onboarding */
                title: string;
                updated_at: string;
            } & {
                [key: string]: unknown;
            })[];
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
        UpdateDealMemberRoleRequest: {
            is_active?: boolean | null;
            name?: string | null;
        };
        ReorderDealMemberRolesRequest: {
            ids: string[];
        };
        Data_DealMemberRoleResponse: {
            data: {
                created_at: string;
                id: string;
                is_active: boolean;
                /** @example pre_sales */
                key: string;
                /** @example Pre-sales */
                name: string;
                /** Format: int32 */
                position: number;
                updated_at: string;
            };
        };
        CreateDealMemberRoleRequest: {
            /** @example solution_architect */
            key?: string | null;
            /** @example Solutions architect */
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
        Data_Vec_DealMemberRoleResponse: {
            data: {
                created_at: string;
                id: string;
                is_active: boolean;
                /** @example pre_sales */
                key: string;
                /** @example Pre-sales */
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
    deal_member_roles_list: {
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
            /** @description Roles of deal team members in their order (active only unless `include_inactive=true`). `sales`, `pre_sales`, `specialist` and `manager` are created with the organization */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_DealMemberRoleResponse"];
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
            /** @description Missing `deals.read` (`permission_denied`, `insufficient_scope`) */
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
    deal_member_roles_create: {
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
                "application/json": components["schemas"]["CreateDealMemberRoleRequest"];
            };
        };
        responses: {
            /** @description Role created at the end of the list (`deal_member_role.created`); `key` defaults to one derived from the name */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_DealMemberRoleResponse"];
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
            /** @description Another role uses the key (`deal_member_role_key_taken`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid name or key, or the limit of 50 roles was reached (`validation_error`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    deal_member_roles_reorder: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ReorderDealMemberRolesRequest"];
            };
        };
        responses: {
            /** @description Roles in the new order (`deal_member_role.reordered`); `ids` lists every role, active or not, exactly once */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_DealMemberRoleResponse"];
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
            /** @description The list does not match the roles of the organization (`validation_error`) */
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
    deal_member_roles_delete: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Role id */
                role_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Role deleted (`deal_member_role.deleted`); the members that had it keep their credit and stay without role */
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
            /** @description Role not found (`deal_member_role_not_found`) */
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
    deal_member_roles_update: {
        parameters: {
            query?: never;
            header?: {
                /** @description `updated_at` of the role as last read (ISO-8601); when it changed since, the update is refused with 409 `version_conflict` */
                "If-Match"?: string | null;
            };
            path: {
                /** @description Role id */
                role_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateDealMemberRoleRequest"];
            };
        };
        responses: {
            /** @description Role updated (`deal_member_role.updated`); the key never changes. Inactive roles stay on the members that have them but cannot be picked again */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_DealMemberRoleResponse"];
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
            /** @description Role not found (`deal_member_role_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The role changed after the `If-Match` version (`version_conflict`) */
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
    deals_list: {
        parameters: {
            query?: {
                /** @example {"op":"and","conditions":[{"field":"name","operator":"contains","value":"ana"}]} */
                filter?: string;
                /** @example -created_at,name */
                sort?: string;
                search?: string;
                /** @example exclude */
                archived?: string;
                page?: number;
                per_page?: number;
                /** @example company,owner */
                include?: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Deals visible under the `deals.read` scope and in workflow open to the user's teams, filtered, searched and sorted through the record query engine. Besides `filter` as JSON, simple conditions go as `filter[field][operator]=value` (`filter[status][eq]=open`, `filter[amount][gte]=1000`, `filter[stage_id][in]=a,b`), combined with `and`. Default order: newest first. `stage`, `contact`, `company`, `owner` and `lost_reason` are always embedded; `include` accepts those names */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page_DealResponse"];
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
            /** @description Missing `deals.read` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid filter, sort, search, include or pagination (`validation_error` with `details.fields`) */
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
    deals_create: {
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
                "application/json": components["schemas"]["DealRequest"];
            };
        };
        responses: {
            /** @description Deal created in the given stage, or in the first open stage of the given (or default) pipeline; the owner defaults to the current user and the currency to the organization's. Publishes `deal.created` (and `deal.won`/`deal.lost` when created closed) */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_DealResponse"];
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
            /** @description Missing `deals.create` (`permission_denied`, `insufficient_scope`) */
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
            /** @description Invalid fields, including `amount` (a decimal string or number), `custom_fields.<key>`, a hidden or inactive pipeline (`validation_error` with `details.fields`), or fields required by the stage (`stage_requirements_missing`, or `stage_requirements_inaccessible` when a missing field is hidden or read-only for the role). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    deals_board: {
        parameters: {
            query?: {
                pipeline_id?: string;
                stage_id?: string;
                /** @example 20 */
                per_column?: number;
                /** @example 1 */
                page?: number;
                /** @example {"op":"and","conditions":[{"field":"name","operator":"contains","value":"ana"}]} */
                filter?: string;
                /** @example -created_at,name */
                sort?: string;
                search?: string;
                /** @example exclude */
                archived?: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Kanban of a pipeline (the default one when `pipeline_id` is omitted): one column per active stage with the count and the sum of `amount` of the deals matching the filter and the read scope, plus the cards of the requested page (`per_column` cards per column). Pass `stage_id` to page through a single column. Accepts the same `filter` (JSON or `filter[field][operator]=value`), `search` and `sort` as the deal listing; `total_amount` is a decimal string */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_BoardResponse"];
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
            /** @description Missing `deals.read` (`permission_denied`, `insufficient_scope`) */
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
            /** @description Invalid filter, sort, search, stage, `per_column` or `page` (`validation_error` with `details.fields`) */
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
    deals_reports_conversion: {
        parameters: {
            query?: {
                /** @example 0190a1b2-0000-7000-8000-000000000010 */
                pipeline_id?: string;
                /** @example quarter */
                period?: components["schemas"]["DashboardPeriodPreset"];
                /** @example 2026-01-01 */
                from?: string;
                /** @example 2026-03-31 */
                to?: string;
                compare_to_previous?: boolean;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Conversion report; `pipeline`, `current` and `previous` are null when the organization has no active pipeline */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_ConversionReportResponse"];
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
            /** @description Missing `deals.read` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown pipeline (`pipeline_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid pipeline id or period (`validation_error` with `details.fields`), or a field used by the report is hidden for the role (`field_not_accessible`) */
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
    deals_reports_performance: {
        parameters: {
            query?: {
                /** @example owner */
                dimension?: components["schemas"]["PerformanceDimension"];
                /** @example 0190a1b2-0000-7000-8000-000000000010 */
                pipeline_id?: string;
                /** @example month */
                period?: components["schemas"]["DashboardPeriodPreset"];
                /** @example 2026-01-01 */
                from?: string;
                /** @example 2026-01-31 */
                to?: string;
                compare_to_previous?: boolean;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Performance report */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PerformanceReportResponse"];
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
            /** @description Missing `deals.read` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown pipeline (`pipeline_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid dimension, pipeline id or period (`validation_error` with `details.fields`), or a field used by the report is hidden for the role (`field_not_accessible`) */
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
    deals_get: {
        parameters: {
            query?: {
                /** @example company,owner */
                include?: string;
            };
            header?: never;
            path: {
                /** @description Deal id */
                deal_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Deal with its custom fields and the stage, contact, company, owner and lost reason (always embedded; `include` accepts those names) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_DealResponse"];
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
            /** @description Missing `deals.read` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Deal not found, outside the read scope or in a pipeline restricted to other teams (`deal_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Unknown relationship in `include` (`validation_error`) */
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
    deals_delete: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Deal id */
                deal_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Deal deleted (soft delete); publishes `deal.deleted` */
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
            /** @description Missing `deals.delete` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Deal not found or outside the delete scope (`deal_not_found`) */
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
    deals_update: {
        parameters: {
            query?: never;
            header?: {
                /** @description `updated_at` of the deal as last read (ISO-8601); when it changed since, the update is refused with 409 `version_conflict` */
                "If-Match"?: string | null;
            };
            path: {
                /** @description Deal id */
                deal_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["DealRequest"];
            };
        };
        responses: {
            /** @description Deal updated; omitted fields stay unchanged and `null` clears them. A new `stage_id` or `status` moves the deal like `/move`, `/won`, `/lost` and `/reopen`. Publishes `deal.amount_changed`, `deal.owner_changed`, `deal.expected_close_date_changed` and the stage events that apply, plus `deal.updated` with the other changed fields (fields covered by a specific event are not repeated in it) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_DealResponse"];
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
            /** @description Missing `deals.update` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Deal not found or outside the update scope (`deal_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The deal changed after the `If-Match` version (`version_conflict`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid fields, including `amount`, `custom_fields.<key>`, a missing `lost_reason_id` when losing or an invalid `If-Match` (`validation_error` with `details.fields`), or fields required by the stage rules when `stage_id` changes (`stage_requirements_missing`, or `stage_requirements_inaccessible` when a missing field is hidden or read-only for the role) */
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
    deals_archive: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Deal id */
                deal_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Deal archived (hidden from listings unless `archived=include|only`); publishes `deal.archived` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_DealResponse"];
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
            /** @description Missing `deals.update` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Deal not found or outside the update scope (`deal_not_found`) */
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
    deals_lose: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Deal id */
                deal_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["LoseDealRequest"];
            };
        };
        responses: {
            /** @description Deal lost with an active lost reason: moved to the first lost stage of its pipeline with `closed_at`. Publishes `deal.stage_changed` and `deal.lost` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_DealResponse"];
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
            /** @description Missing `deals.update` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Deal not found or outside the update scope (`deal_not_found`) */
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
            /** @description Missing or inactive lost reason (`validation_error`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    deal_members_list: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Deal id */
                deal_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Team of the deal */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_DealTeamResponse"];
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
            /** @description Missing `deals.read` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Deal not found or not visible (`deal_not_found`) */
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
    deal_members_create: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Deal id */
                deal_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["AddDealMemberRequest"];
            };
        };
        responses: {
            /** @description Member added */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_DealMemberResponse"];
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
            /** @description Missing `deals.update` on the deal (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Deal not found or not visible (`deal_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The user is already a member (`deal_member_exists`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid user, role or credit (`validation_error`), credit above the available share (`deal_credit_exceeds_total`, with `allocated_percent` and `available_percent` in `details`) or member limit reached (`deal_members_limit_reached`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    deal_members_delete: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Deal id */
                deal_id: string;
                /** @description Member id */
                member_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Member removed */
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
            /** @description Missing `deals.update` on the deal (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Deal or member not found (`deal_not_found`, `deal_member_not_found`) */
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
    deal_members_update: {
        parameters: {
            query?: never;
            header?: {
                /** @description `updated_at` of the member as read (ISO-8601); the update fails with 409 `version_conflict` when it changed since */
                "If-Match"?: string | null;
            };
            path: {
                /** @description Deal id */
                deal_id: string;
                /** @description Member id */
                member_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateDealMemberRequest"];
            };
        };
        responses: {
            /** @description Member updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_DealMemberResponse"];
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
            /** @description Missing `deals.update` on the deal (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Deal or member not found (`deal_not_found`, `deal_member_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The member changed since the `If-Match` version (`version_conflict`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid role or credit (`validation_error`) or credit above the available share (`deal_credit_exceeds_total`) */
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
    deals_move: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Deal id */
                deal_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["MoveDealRequest"];
            };
        };
        responses: {
            /** @description Deal moved to the stage; a won stage wins it, a lost stage loses it (requires `lost_reason_id`) and an open stage reopens a closed deal. `fields` fills system or custom fields together with the move (for the stage rules). Publishes `deal.stage_changed`, `deal.pipeline_changed` when the stage belongs to another pipeline, and `deal.won`, `deal.lost` or `deal.reopened` when the status changes */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_DealResponse"];
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
            /** @description Missing `deals.update` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Deal not found, outside the update scope or in a pipeline hidden from the user's teams (`deal_not_found`) */
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
            /** @description Unknown or inactive stage, missing lost reason, or fields required to leave the current stage or enter the new one are empty (`stage_requirements_missing`, or `stage_requirements_inaccessible` when a missing field is hidden or read-only for the role, with `details.fields`); other invalid input answers `validation_error` with `details.fields`. `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    deals_move_pipeline: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Deal id */
                deal_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["MoveDealPipelineRequest"];
            };
        };
        responses: {
            /** @description Deal moved to another pipeline, into `stage_id` or, when omitted, the first active stage of the same kind as the current one. Publishes `deal.pipeline_changed` (old and new pipeline) and `deal.stage_changed` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_DealResponse"];
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
            /** @description Missing `deals.update` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Deal not found, outside the update scope or in a hidden pipeline (`deal_not_found`) */
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
            /** @description Unknown, hidden or inactive pipeline, stage outside the pipeline, missing lost reason or stage requirements (`stage_requirements_missing`, or `stage_requirements_inaccessible` when a missing field is hidden or read-only for the role); other invalid input answers `validation_error` with `details.fields`. `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    deals_reopen: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Deal id */
                deal_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ReopenDealRequest"];
            };
        };
        responses: {
            /** @description Closed deal reopened in the given open stage (or the first open stage of its pipeline); clears `closed_at` and the lost reason. Publishes `deal.stage_changed` and `deal.reopened` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_DealResponse"];
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
            /** @description Missing `deals.update` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Deal not found or outside the update scope (`deal_not_found`) */
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
            /** @description The stage is not an active open stage (`validation_error`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    deals_unarchive: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Deal id */
                deal_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Deal restored from the archive; publishes `deal.unarchived` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_DealResponse"];
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
            /** @description Missing `deals.update` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Deal not found or outside the update scope (`deal_not_found`) */
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
    deals_win: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Deal id */
                deal_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Deal won: moved to the first won stage of its pipeline with `closed_at`. Publishes `deal.stage_changed` and `deal.won` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_DealResponse"];
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
            /** @description Missing `deals.update` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Deal not found or outside the update scope (`deal_not_found`) */
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
            /** @description The pipeline has no active won stage (`validation_error`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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

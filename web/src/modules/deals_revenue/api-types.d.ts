export type paths = Record<string, never>;
export type webhooks = Record<string, never>;
export interface components {
    schemas: {
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
        /** @enum {string} */
        StageKind: "open" | "won" | "lost";
        DealOwnerResponse: {
            display_name: string;
            id: string;
        };
        DealNamedResponse: {
            id: string;
            name: string;
        };
    };
    responses: never;
    parameters: never;
    requestBodies: never;
    headers: never;
    pathItems: never;
}
export type $defs = Record<string, never>;
export type operations = Record<string, never>;

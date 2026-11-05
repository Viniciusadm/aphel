export interface paths {
    "/api/v1/assignment/assignments": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["assignment_assignments_list"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/assignment/rules": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["assignment_rules_list"];
        put?: never;
        post: operations["assignment_rules_create"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/assignment/rules/{rule_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["assignment_rules_get"];
        put?: never;
        post?: never;
        delete: operations["assignment_rules_delete"];
        options?: never;
        head?: never;
        patch: operations["assignment_rules_update"];
        trace?: never;
    };
    "/api/v1/assignment/rules/{rule_id}/simulate": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["assignment_rules_simulate"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/assignment/targets": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["assignment_targets"];
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
        LeadDistributionSimulationResponse: {
            recipients: components["schemas"]["LeadDistributionRecipientResponse"][];
            route_index?: number | null;
            selection: components["schemas"]["LeadDistributionSelection"];
            skipped: components["schemas"]["LeadDistributionSkippedResponse"][];
        };
        LeadDistributionSkippedResponse: {
            name?: string | null;
            reason: components["schemas"]["LeadDistributionSkipReason"];
            user_id: string;
        };
        /** @enum {string} */
        LeadDistributionSkipReason: "away" | "inactive";
        /** @enum {string} */
        LeadDistributionSelection: "members" | "route" | "fallback" | "unassigned";
        LeadDistributionRecipientResponse: {
            name: string;
            position: number;
            user_id: string;
        };
        /** @enum {string} */
        LeadDistributionFallback: "members" | "none";
        /** @enum {string} */
        LeadDistributionStrategy: "round_robin" | "weighted" | "least_loaded";
        LeadDistributionRouteResponse: {
            team_id?: string | null;
            user_ids: string[];
            values: string[];
        };
        LeadDistributionMemberResponse: {
            active: boolean;
            away: boolean;
            away_from?: string | null;
            away_now: boolean;
            away_until?: string | null;
            name?: string | null;
            user_id: string;
            /** Format: int32 */
            weight: number;
        };
        LeadDistributionRuleResponse: {
            /** Format: int64 */
            assignments: number;
            can_manage: boolean;
            /** @example 2026-10-21T13:45:00Z */
            created_at: string;
            created_by?: string | null;
            description?: string | null;
            id: string;
            is_active: boolean;
            key: string;
            last_assigned_at?: string | null;
            last_user_id?: string | null;
            members: components["schemas"]["LeadDistributionMemberResponse"][];
            name: string;
            record_type: string;
            routing?: null | components["schemas"]["LeadDistributionRoutingResponse"];
            strategy: components["schemas"]["LeadDistributionStrategy"];
            /** @example 2026-10-21T13:45:00Z */
            updated_at: string;
        };
        LeadDistributionRoutingResponse: {
            fallback: components["schemas"]["LeadDistributionFallback"];
            field: string;
            routes: components["schemas"]["LeadDistributionRouteResponse"][];
        };
        Data_Vec_AssignmentTarget: {
            data: {
                label: string;
                labels: {
                    [key: string]: string;
                };
                record_type: string;
            }[];
        };
        Data_LeadDistributionSimulationResponse: {
            data: {
                recipients: components["schemas"]["LeadDistributionRecipientResponse"][];
                route_index?: number | null;
                selection: components["schemas"]["LeadDistributionSelection"];
                skipped: components["schemas"]["LeadDistributionSkippedResponse"][];
            };
        };
        LeadDistributionSimulateRequest: {
            /** @example 10 */
            count?: number | null;
            /**
             * @example {
             *       "source": "google"
             *     }
             */
            values?: {
                [key: string]: unknown;
            };
        };
        Data_LeadDistributionRuleResponse: {
            data: {
                /** Format: int64 */
                assignments: number;
                can_manage: boolean;
                /** @example 2026-10-21T13:45:00Z */
                created_at: string;
                created_by?: string | null;
                description?: string | null;
                id: string;
                is_active: boolean;
                key: string;
                last_assigned_at?: string | null;
                last_user_id?: string | null;
                members: components["schemas"]["LeadDistributionMemberResponse"][];
                name: string;
                record_type: string;
                routing?: null | components["schemas"]["LeadDistributionRoutingResponse"];
                strategy: components["schemas"]["LeadDistributionStrategy"];
                /** @example 2026-10-21T13:45:00Z */
                updated_at: string;
            };
        };
        LeadDistributionRuleRequest: {
            description?: string | null;
            is_active?: boolean | null;
            /** @example inbound_leads */
            key?: string | null;
            members?: components["schemas"]["LeadDistributionMemberRequest"][] | null;
            /** @example Website leads */
            name?: string | null;
            /** @example custom.ticket */
            record_type?: string | null;
            routing?: null | components["schemas"]["LeadDistributionRoutingRequest"];
            strategy?: null | components["schemas"]["LeadDistributionStrategy"];
        };
        LeadDistributionRoutingRequest: {
            fallback?: null | components["schemas"]["LeadDistributionFallback"];
            /** @example source */
            field: string;
            routes: components["schemas"]["LeadDistributionRouteRequest"][];
        };
        LeadDistributionRouteRequest: {
            team_id?: string | null;
            user_ids?: string[];
            /**
             * @example [
             *       "sp",
             *       "rj"
             *     ]
             */
            values: string[];
        };
        LeadDistributionMemberRequest: {
            away?: boolean;
            /** @example 2026-10-21T00:00:00Z */
            away_from?: string | null;
            /** @example 2026-10-28T00:00:00Z */
            away_until?: string | null;
            user_id: string;
            /**
             * Format: int32
             * @example 1
             */
            weight?: number | null;
        };
        Page_LeadDistributionRuleResponse: {
            data: {
                /** Format: int64 */
                assignments: number;
                can_manage: boolean;
                /** @example 2026-10-21T13:45:00Z */
                created_at: string;
                created_by?: string | null;
                description?: string | null;
                id: string;
                is_active: boolean;
                key: string;
                last_assigned_at?: string | null;
                last_user_id?: string | null;
                members: components["schemas"]["LeadDistributionMemberResponse"][];
                name: string;
                record_type: string;
                routing?: null | components["schemas"]["LeadDistributionRoutingResponse"];
                strategy: components["schemas"]["LeadDistributionStrategy"];
                /** @example 2026-10-21T13:45:00Z */
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
        ErrorEnvelope: {
            error: components["schemas"]["ErrorBody"];
        };
        ErrorBody: {
            /** @example validation_error */
            code: string;
            details?: Record<string, never> | null;
            message: string;
        };
        Page_LeadDistributionAssignmentResponse: {
            data: {
                /** @example 2026-10-21T13:45:00Z */
                created_at: string;
                id: string;
                record_id?: string | null;
                /** @example custom.ticket */
                record_type?: string | null;
                rule_id: string;
                rule_key: string;
                rule_name: string;
                source: components["schemas"]["LeadDistributionAssignmentSource"];
                user_id?: string | null;
                user_name?: string | null;
            }[];
            meta: components["schemas"]["PageMeta"];
        };
        /** @enum {string} */
        LeadDistributionAssignmentSource: "automation" | "incoming_webhook" | "import" | "booking" | "lead_form" | "plugin";
        LeadDistributionAssignmentResponse: {
            /** @example 2026-10-21T13:45:00Z */
            created_at: string;
            id: string;
            record_id?: string | null;
            /** @example custom.ticket */
            record_type?: string | null;
            rule_id: string;
            rule_key: string;
            rule_name: string;
            source: components["schemas"]["LeadDistributionAssignmentSource"];
            user_id?: string | null;
            user_name?: string | null;
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
    assignment_assignments_list: {
        parameters: {
            query?: {
                rule_id?: string;
                user_id?: string;
                /** @example custom.ticket */
                record_type?: string;
                source?: components["schemas"]["LeadDistributionAssignmentSource"];
                page?: number;
                per_page?: number;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Assignment history of the distribution rules, newest first: who received which record, from which rule and source (`automation`, `incoming_webhook`, `import`, `booking`, `lead_form`). `user_id` is null when the user was removed and `record_type`/`record_id` are null for picks without a record. Entries of a deleted rule are deleted with it */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page_LeadDistributionAssignmentResponse"];
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
            /** @description Missing `assignment.manage` (`permission_denied`, `insufficient_scope`) */
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
    assignment_rules_list: {
        parameters: {
            query?: {
                /** @example custom.ticket */
                record_type?: string;
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
            /** @description Lead distribution rules of the organization ordered by name, with members (name, active, away now), routing and usage (`assignments`, `last_user_id`, `last_assigned_at`). Needs `assignment.manage` or `automations.manage` (to pick a rule in an automation) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page_LeadDistributionRuleResponse"];
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
            /** @description Missing `assignment.manage` (`permission_denied`, `insufficient_scope`) */
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
    assignment_rules_create: {
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
                "application/json": components["schemas"]["LeadDistributionRuleRequest"];
            };
        };
        responses: {
            /** @description Rule created (`assignment_rule.created`). `record_type` is any registered entity with an owner; `strategy` is `round_robin` (whoever waited longest), `weighted` (smooth weighted rotation by `weight` 1–100) or `least_loaded` (fewest active records of the rule target; active states come from owner metadata). Members marked `away` (optionally between `away_from` and `away_until`) and users that are not active in the organization are skipped. `routing` maps values of a record field to a team and/or users (first matching route wins, values compared without case); without a match the rule falls back to the members (`fallback: members`) or leaves the record unassigned (`none`). `key` defaults to one derived from the name */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_LeadDistributionRuleResponse"];
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
            /** @description Missing `assignment.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Another rule already uses the key (`assignment_rule_key_taken`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid properties, unknown users, teams or field (`validation_error`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    assignment_rules_get: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Rule id */
                rule_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Rule with members, routing and usage */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_LeadDistributionRuleResponse"];
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
            /** @description Missing `assignment.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Rule not found (`assignment_rule_not_found`) */
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
    assignment_rules_delete: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Rule id */
                rule_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Rule deleted with its rotation state and assignment log (`assignment_rule.deleted`). Automations, incoming webhooks and imports that still point to it stop assigning (the automation step fails with `assignment_rule_not_found`) */
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
            /** @description Missing `assignment.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Rule not found (`assignment_rule_not_found`) */
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
    assignment_rules_update: {
        parameters: {
            query?: never;
            header?: {
                /** @description `updated_at` of the rule as read (ISO-8601, quotes optional); the update fails with 409 when it changed since */
                "If-Match"?: string | null;
            };
            path: {
                /** @description Rule id */
                rule_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["LeadDistributionRuleRequest"];
            };
        };
        responses: {
            /** @description Rule updated (`assignment_rule.updated`); omitted properties stay unchanged, `members` replaces the whole list and `routing: null` removes the routing. The rotation state is kept */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_LeadDistributionRuleResponse"];
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
            /** @description Missing `assignment.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Rule not found (`assignment_rule_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Key already used (`assignment_rule_key_taken`) or the rule changed since the `If-Match` version (`version_conflict`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid properties or malformed `If-Match` (`validation_error`) */
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
    assignment_rules_simulate: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Rule id */
                rule_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["LeadDistributionSimulateRequest"];
            };
        };
        responses: {
            /** @description Who would receive the next `count` (default 10, at most 50) records from the current rotation state, without writing anything. `values` are record field values used for the routing (`selection` tells whether the members, a route (`route_index`), the fallback or nobody would be used). `skipped` lists away and inactive users of the pool. An inactive rule simulates as if it were active */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_LeadDistributionSimulationResponse"];
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
            /** @description Missing `assignment.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Rule not found (`assignment_rule_not_found`) */
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
            /** @description Invalid count (`validation_error`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    assignment_targets: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Registered entities with assignment ownership support */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_AssignmentTarget"];
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

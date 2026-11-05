export interface paths {
    "/api/v1/validation-rules": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Validation rules of the organization, ordered by record type and position. Needs `settings.manage`. */
        get: operations["validation_rules_list"];
        put?: never;
        /** @description Creates a validation rule and publishes `validation_rule.created`. `expression` is a boolean formula (same language as formula fields) describing the **error condition**: when it evaluates to `true` over the proposed state of the record, the write is refused with 422 `validation_rule_failed` and `message` in `details.fields` (path of `field`, `custom_fields.<key>` for custom fields, or `record`). Applies to every write of the record type (forms, inline editing, API, imports, automations, bulk actions) on create, update or both (`applies_on`). `key` is generated from the name when left out. Needs `settings.manage`. */
        post: operations["validation_rules_create"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/validation-rules/order": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** @description Sets the order of every rule of a record type (`validation_rule.reordered`). Needs `settings.manage`. */
        put: operations["validation_rules_reorder"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/validation-rules/preview": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Checks a rule condition (syntax, types, boolean result) and, with `record_id`, evaluates it over the saved state of that record: `fails = true` means the rule would block a write leaving the record as it is. Needs `settings.manage`. */
        post: operations["validation_rules_preview"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/validation-rules/{rule_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["validation_rules_get"];
        put?: never;
        post?: never;
        delete: operations["validation_rules_delete"];
        options?: never;
        head?: never;
        /** @description Updates a validation rule (`validation_rule.updated`). `is_active = false` turns the rule off without removing it. The record type and the key never change. Fields left out keep their value; `field: null` clears the highlighted field. Needs `settings.manage`. */
        patch: operations["validation_rules_update"];
        trace?: never;
    };
}
export type webhooks = Record<string, never>;
export interface components {
    schemas: {
        ValidationRulePreviewResponse: {
            fails?: boolean | null;
            formula: components["schemas"]["FormulaPreviewResponse"];
        };
        FormulaPreviewResponse: {
            dependencies: components["schemas"]["FormulaDependenciesResponse"];
            errors: components["schemas"]["FormulaErrorResponse"][];
            evaluated: boolean;
            kind?: null | components["schemas"]["FormulaValueKind"];
            valid: boolean;
            value?: Record<string, never> | null;
        };
        /** @enum {string} */
        FormulaValueKind: "number" | "text" | "date" | "boolean";
        FormulaErrorResponse: {
            /** @example unknown_field */
            code: string;
            end: number;
            message: string;
            start: number;
        };
        FormulaDependenciesResponse: {
            aggregates: components["schemas"]["FormulaAggregateResponse"][];
            fields: string[];
            uses_today: boolean;
        };
        FormulaAggregateResponse: {
            /** @example total */
            field?: string | null;
            /** @example sum */
            function: string;
            /** @example items */
            relation: string;
        };
        ValidationRuleResponse: {
            applies_on: components["schemas"]["ValidationRuleTiming"];
            /** @example 2026-10-20T13:45:00Z */
            created_at: string;
            created_by?: string | null;
            /** @example amount > 10000 and is_empty(source) */
            expression: string;
            /** @example source */
            field?: string | null;
            id: string;
            is_active: boolean;
            /** @example discount_needs_reason */
            key: string;
            /** @example Provide the source for deals above 10k. */
            message: string;
            /** @example Discount requires justification */
            name: string;
            /** Format: int32 */
            position: number;
            /** @example deals.deal */
            record_type: string;
            /** @example 2026-10-20T13:45:00Z */
            updated_at: string;
            updated_by?: string | null;
        };
        /** @enum {string} */
        ValidationRuleTiming: "always" | "create" | "update";
        UpdateValidationRuleRequest: {
            applies_on?: null | components["schemas"]["ValidationRuleTiming"];
            expression?: string | null;
            field?: string | null;
            is_active?: boolean | null;
            message?: string | null;
            name?: string | null;
        };
        Data_ValidationRulePreviewResponse: {
            data: {
                fails?: boolean | null;
                formula: components["schemas"]["FormulaPreviewResponse"];
            };
        };
        ValidationRulePreviewRequest: {
            /** @example amount > 10000 and is_empty(source) */
            expression: string;
            record_id?: string | null;
            /** @example deals.deal */
            record_type: string;
        };
        ReorderValidationRulesRequest: {
            ids: string[];
            /** @example deals.deal */
            record_type: string;
        };
        Data_ValidationRuleResponse: {
            data: {
                applies_on: components["schemas"]["ValidationRuleTiming"];
                /** @example 2026-10-20T13:45:00Z */
                created_at: string;
                created_by?: string | null;
                /** @example amount > 10000 and is_empty(source) */
                expression: string;
                /** @example source */
                field?: string | null;
                id: string;
                is_active: boolean;
                /** @example discount_needs_reason */
                key: string;
                /** @example Provide the source for deals above 10k. */
                message: string;
                /** @example Discount requires justification */
                name: string;
                /** Format: int32 */
                position: number;
                /** @example deals.deal */
                record_type: string;
                /** @example 2026-10-20T13:45:00Z */
                updated_at: string;
                updated_by?: string | null;
            };
        };
        CreateValidationRuleRequest: {
            applies_on?: null | components["schemas"]["ValidationRuleTiming"];
            expression: string;
            field?: string | null;
            is_active?: boolean | null;
            /** @example discount_needs_reason */
            key?: string | null;
            message: string;
            name: string;
            /** @example deals.deal */
            record_type: string;
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
        Data_Vec_ValidationRuleResponse: {
            data: {
                applies_on: components["schemas"]["ValidationRuleTiming"];
                /** @example 2026-10-20T13:45:00Z */
                created_at: string;
                created_by?: string | null;
                /** @example amount > 10000 and is_empty(source) */
                expression: string;
                /** @example source */
                field?: string | null;
                id: string;
                is_active: boolean;
                /** @example discount_needs_reason */
                key: string;
                /** @example Provide the source for deals above 10k. */
                message: string;
                /** @example Discount requires justification */
                name: string;
                /** Format: int32 */
                position: number;
                /** @example deals.deal */
                record_type: string;
                /** @example 2026-10-20T13:45:00Z */
                updated_at: string;
                updated_by?: string | null;
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
    validation_rules_list: {
        parameters: {
            query?: {
                record_type?: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Validation rules */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_ValidationRuleResponse"];
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
    validation_rules_create: {
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
                "application/json": components["schemas"]["CreateValidationRuleRequest"];
            };
        };
        responses: {
            /** @description Rule created */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_ValidationRuleResponse"];
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
            /** @description Invalid name, key, message, field, timing or condition (`validation_error` with `details.fields`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    validation_rules_reorder: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ReorderValidationRulesRequest"];
            };
        };
        responses: {
            /** @description Rules of the record type in the new order */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_ValidationRuleResponse"];
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
            /** @description Unknown record type (`record_type_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description `ids` must list every rule of the record type once (`validation_error`) */
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
    validation_rules_preview: {
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
                "application/json": components["schemas"]["ValidationRulePreviewRequest"];
            };
        };
        responses: {
            /** @description Errors with character positions, dependencies and, with `record_id`, whether the record breaks the rule */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_ValidationRulePreviewResponse"];
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
            /** @description Unknown record type (`record_type_not_found`) or record not visible (`record_not_found`) */
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
    validation_rules_get: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Validation rule id */
                rule_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Validation rule */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_ValidationRuleResponse"];
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
            /** @description Rule not found (`validation_rule_not_found`) */
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
    validation_rules_delete: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Validation rule id */
                rule_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Rule removed (`validation_rule.deleted`) */
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
            /** @description Rule not found (`validation_rule_not_found`) */
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
    validation_rules_update: {
        parameters: {
            query?: never;
            header?: {
                /** @description `updated_at` of the rule as read (ISO-8601) */
                "If-Match"?: string | null;
            };
            path: {
                /** @description Validation rule id */
                rule_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateValidationRuleRequest"];
            };
        };
        responses: {
            /** @description Rule updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_ValidationRuleResponse"];
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
            /** @description Rule not found (`validation_rule_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The rule changed since the `If-Match` version (`version_conflict`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid name, message, field, timing or condition (`validation_error` with `details.fields`) */
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

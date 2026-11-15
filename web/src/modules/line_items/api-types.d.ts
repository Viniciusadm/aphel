export interface paths {
    "/api/v1/line-items/{record_type}/{record_id}/items": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Scoped line items. Parent entity permissions and monetary field access apply; mutations, audit and owner projections share one transaction. */
        get: operations["line_items_list"];
        put?: never;
        /** @description Scoped line items. Parent entity permissions and monetary field access apply; mutations, audit and owner projections share one transaction. */
        post: operations["line_items_create"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/line-items/{record_type}/{record_id}/items/apply-total": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Scoped line items. Parent entity permissions and monetary field access apply; mutations, audit and owner projections share one transaction. */
        post: operations["line_items_apply_total"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/line-items/{record_type}/{record_id}/items/order": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** @description Scoped line items. Parent entity permissions and monetary field access apply; mutations, audit and owner projections share one transaction. */
        put: operations["line_items_reorder"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/line-items/{record_type}/{record_id}/items/{item_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        delete: operations["line_items_delete"];
        options?: never;
        head?: never;
        /** @description Scoped line items. Parent entity permissions and monetary field access apply; mutations, audit and owner projections share one transaction. */
        patch: operations["line_items_update"];
        trace?: never;
    };
}
export type webhooks = Record<string, never>;
export interface components {
    schemas: {
        LineItemResponse: {
            /** @example 13500.00 */
            amount_base?: string | null;
            /** @example 2026-10-20T13:45:00Z */
            created_at: string;
            created_by?: string | null;
            /** @example 300.00 */
            discount_amount: string;
            discount_type: components["schemas"]["DiscountType"];
            /** @example 10.00 */
            discount_value: string;
            /** @example 5.4 */
            exchange_rate?: string | null;
            /** @example 3000.00 */
            gross: string;
            id: string;
            /** @example 225.00 */
            monthly_amount?: string | null;
            name: string;
            /** Format: int32 */
            position: number;
            product_id?: string | null;
            /** @example 2 */
            quantity: string;
            record_id: string;
            record_type: string;
            recurrence: components["schemas"]["Recurrence"];
            tax: string;
            tax_percent: string;
            /**
             * Format: int32
             * @example 12
             */
            term_months?: number | null;
            /** @example 2700.00 */
            total: string;
            unit?: string | null;
            /** @example 1500.00 */
            unit_price: string;
            /** @example 2026-10-20T13:45:00Z */
            updated_at: string;
        };
        /** @enum {string} */
        Recurrence: "none" | "monthly" | "quarterly" | "yearly";
        /** @enum {string} */
        DiscountType: "none" | "percent" | "amount";
        ReorderLineItemsRequest: {
            item_ids: string[];
        };
        Data_LineItemResponse: {
            data: {
                /** @example 13500.00 */
                amount_base?: string | null;
                /** @example 2026-10-20T13:45:00Z */
                created_at: string;
                created_by?: string | null;
                /** @example 300.00 */
                discount_amount: string;
                discount_type: components["schemas"]["DiscountType"];
                /** @example 10.00 */
                discount_value: string;
                /** @example 5.4 */
                exchange_rate?: string | null;
                /** @example 3000.00 */
                gross: string;
                id: string;
                /** @example 225.00 */
                monthly_amount?: string | null;
                name: string;
                /** Format: int32 */
                position: number;
                product_id?: string | null;
                /** @example 2 */
                quantity: string;
                record_id: string;
                record_type: string;
                recurrence: components["schemas"]["Recurrence"];
                tax: string;
                tax_percent: string;
                /**
                 * Format: int32
                 * @example 12
                 */
                term_months?: number | null;
                /** @example 2700.00 */
                total: string;
                unit?: string | null;
                /** @example 1500.00 */
                unit_price: string;
                /** @example 2026-10-20T13:45:00Z */
                updated_at: string;
            };
        };
        LineItemRequest: {
            discount_type?: null | components["schemas"]["DiscountType"];
            discount_value?: unknown;
            /** @example Onboarding */
            name?: string | null;
            product_id?: string | null;
            quantity?: unknown;
            recurrence?: null | components["schemas"]["Recurrence"];
            tax_percent?: unknown;
            /**
             * Format: int64
             * @example 12
             */
            term_months?: number | null;
            /** @example hour */
            unit?: string | null;
            unit_price?: unknown;
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
        Data_LineItemsResponse: {
            data: {
                amount_managed: boolean;
                amount_mode: components["schemas"]["AmountMode"];
                can_apply_total: boolean;
                /** @example BRL */
                currency: string;
                /** @example 300.00 */
                discount_total: string;
                items: components["schemas"]["LineItemResponse"][];
                /** @example 2700.00 */
                record_amount?: string | null;
                record_id: string;
                record_type: string;
                /** @example 3000.00 */
                subtotal: string;
                /** @example 2700.00 */
                total: string;
            };
        };
        /** @enum {string} */
        AmountMode: "manual" | "items";
        LineItemsResponse: {
            amount_managed: boolean;
            amount_mode: components["schemas"]["AmountMode"];
            can_apply_total: boolean;
            /** @example BRL */
            currency: string;
            /** @example 300.00 */
            discount_total: string;
            items: components["schemas"]["LineItemResponse"][];
            /** @example 2700.00 */
            record_amount?: string | null;
            record_id: string;
            record_type: string;
            /** @example 3000.00 */
            subtotal: string;
            /** @example 2700.00 */
            total: string;
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
    line_items_list: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity key */
                record_type: string;
                /** @description Record id */
                record_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Items and totals of the record */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_LineItemsResponse"];
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
            /** @description Missing the parent entity read permission (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Record not found, not visible or not attachable (`line_item_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description A declared monetary field is hidden for the role (`field_not_accessible`) */
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
    line_items_create: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Entity key */
                record_type: string;
                /** @description Record id */
                record_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["LineItemRequest"];
            };
        };
        responses: {
            /** @description Item added */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_LineItemResponse"];
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
            /** @description Missing the parent entity update permission (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Record not found, not visible or not attachable (`line_item_not_found`) */
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
            /** @description Invalid item (`validation_error` with fields such as `name`, `quantity`, `discount_value` or an unknown/inactive `product_id`), product in another currency (`product_currency_mismatch`), item limit reached (`line_items_limit_reached`) or `amount` not editable in the `items` mode (`field_not_accessible`, `field_read_only`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    line_items_apply_total: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Entity key */
                record_type: string;
                /** @description Record id */
                record_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Items and totals after the update */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_LineItemsResponse"];
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
            /** @description Missing the parent entity update permission (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Record not found, not visible or not attachable (`line_item_not_found`) */
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
            /** @description The parent has no items (`record_has_no_items`) or `amount` is not editable for the role (`field_not_accessible`, `field_read_only`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    line_items_reorder: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity key */
                record_type: string;
                /** @description Record id */
                record_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ReorderLineItemsRequest"];
            };
        };
        responses: {
            /** @description Items in the new order */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_LineItemsResponse"];
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
            /** @description Missing the parent entity update permission (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Record not found, not visible or not attachable (`line_item_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description `item_ids` is not a permutation of the items (`validation_error`) */
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
    line_items_delete: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Entity key */
                record_type: string;
                /** @description Record id */
                record_id: string;
                /** @description Item id */
                item_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Item removed; publishes `line_items.item_removed`. In the `items` amount mode the parent amount is recalculated (removing the last item sets it to `0.00`) */
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
            /** @description Missing the parent entity update permission (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Record or item not found (`line_item_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description `amount` not editable in the `items` mode (`field_not_accessible`, `field_read_only`) */
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
    line_items_update: {
        parameters: {
            query?: never;
            header?: {
                /** @description `updated_at` of the item as read (ISO-8601, quotes optional); the update fails with 409 when it changed since */
                "If-Match"?: string | null;
            };
            path: {
                /** @description Entity key */
                record_type: string;
                /** @description Record id */
                record_id: string;
                /** @description Item id */
                item_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["LineItemRequest"];
            };
        };
        responses: {
            /** @description Item updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_LineItemResponse"];
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
            /** @description Missing the parent entity update permission (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Record or item not found (`line_item_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The item changed since the `If-Match` version (`version_conflict`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid item or malformed `If-Match` (`validation_error`), product in another currency (`product_currency_mismatch`) or `amount` not editable in the `items` mode (`field_not_accessible`, `field_read_only`) */
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

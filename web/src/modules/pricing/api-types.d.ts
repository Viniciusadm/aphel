export interface paths {
    "/api/v1/pricing": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["price_lists_list"];
        put?: never;
        post: operations["price_lists_create"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/pricing/discount-limits": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["discount_limits_get"];
        put: operations["discount_limits_set"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/pricing/items/{product_id}/bundle": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["products_bundle_get"];
        put: operations["products_bundle_save"];
        post?: never;
        delete: operations["products_bundle_delete"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/pricing/{price_list_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["price_lists_get"];
        put?: never;
        post?: never;
        delete: operations["price_lists_delete"];
        options?: never;
        head?: never;
        patch: operations["price_lists_update"];
        trace?: never;
    };
    "/api/v1/pricing/{price_list_id}/items": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["price_lists_items_list"];
        put: operations["price_lists_items_replace"];
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
        ProductBundleResponse: {
            components: components["schemas"]["BundleComponentResponse"][];
            /** @example BRL */
            currency: string;
            pricing: components["schemas"]["BundlePricing"];
            product_id: string;
            /** @example 990.00 */
            unit_price: string;
            /** @example 2026-10-22T13:45:00Z */
            updated_at: string;
        };
        /** @enum {string} */
        BundlePricing: "fixed" | "sum";
        BundleComponentResponse: {
            code?: string | null;
            name: string;
            product_id: string;
            /** @example 2 */
            quantity: string;
            /** @example 240.00 */
            total: string;
            /** @example 120.00 */
            unit_price: string;
        };
        ReplacePriceListItemsRequest: {
            items: components["schemas"]["PriceListItemRequest"][];
        };
        PriceListItemRequest: {
            min_quantity?: null | components["schemas"]["DecimalInput"];
            product_id: string;
            unit_price: components["schemas"]["DecimalInput"];
        };
        /** @example 15000.00 */
        DecimalInput: string | number;
        Data_Vec_PriceListItemResponse: {
            data: {
                id: string;
                /** @example 1 */
                min_quantity: string;
                product_code?: string | null;
                /** @example BRL */
                product_currency: string;
                product_id: string;
                product_name: string;
                /** @example 1500.00 */
                product_price: string;
                /** @example 1350.00 */
                unit_price: string;
            }[];
        };
        Data_ProductBundleResponse: {
            data: {
                components: components["schemas"]["BundleComponentResponse"][];
                /** @example BRL */
                currency: string;
                pricing: components["schemas"]["BundlePricing"];
                product_id: string;
                /** @example 990.00 */
                unit_price: string;
                /** @example 2026-10-22T13:45:00Z */
                updated_at: string;
            };
        };
        ProductBundleRequest: {
            components: components["schemas"]["BundleComponentRequest"][];
            pricing: components["schemas"]["BundlePricing"];
        };
        BundleComponentRequest: {
            product_id: string;
            quantity: components["schemas"]["DecimalInput"];
        };
        Data_Option_ProductBundleResponse: {
            data: null | {
                components: components["schemas"]["BundleComponentResponse"][];
                /** @example BRL */
                currency: string;
                pricing: components["schemas"]["BundlePricing"];
                product_id: string;
                /** @example 990.00 */
                unit_price: string;
                /** @example 2026-10-22T13:45:00Z */
                updated_at: string;
            };
        };
        SetDiscountLimitsRequest: {
            limits: components["schemas"]["DiscountLimitRequest"][];
        };
        DiscountLimitRequest: {
            max_discount_percent?: null | components["schemas"]["DecimalInput"];
            role_id: string;
        };
        Data_Vec_DiscountLimitResponse: {
            data: {
                /** @example 15 */
                max_discount_percent?: string | null;
                role_id: string;
                /** @example sales */
                role_key: string;
                /** @example Sales rep */
                role_name: string;
            }[];
        };
        Data_PriceListResponse: {
            data: {
                applies_to: components["schemas"]["PriceListAppliesTo"];
                company_ids: string[];
                /** @example 2026-10-22T13:45:00Z */
                created_at: string;
                created_by?: string | null;
                /** @example BRL */
                currency: string;
                description?: string | null;
                id: string;
                is_active: boolean;
                is_default: boolean;
                /** Format: int64 */
                item_count: number;
                /** @example wholesale */
                key: string;
                /** @example Wholesale */
                name: string;
                segment_id?: string | null;
                /** @example 2026-10-22T13:45:00Z */
                updated_at: string;
                /** @example 2026-01-01 */
                valid_from?: string | null;
                /** @example 2026-12-31 */
                valid_to?: string | null;
            };
        };
        /** @enum {string} */
        PriceListAppliesTo: "all" | "segment" | "organizations";
        PriceListResponse: {
            applies_to: components["schemas"]["PriceListAppliesTo"];
            company_ids: string[];
            /** @example 2026-10-22T13:45:00Z */
            created_at: string;
            created_by?: string | null;
            /** @example BRL */
            currency: string;
            description?: string | null;
            id: string;
            is_active: boolean;
            is_default: boolean;
            /** Format: int64 */
            item_count: number;
            /** @example wholesale */
            key: string;
            /** @example Wholesale */
            name: string;
            segment_id?: string | null;
            /** @example 2026-10-22T13:45:00Z */
            updated_at: string;
            /** @example 2026-01-01 */
            valid_from?: string | null;
            /** @example 2026-12-31 */
            valid_to?: string | null;
        };
        PriceListRequest: {
            applies_to?: null | components["schemas"]["PriceListAppliesTo"];
            company_ids?: string[] | null;
            /** @example BRL */
            currency?: string | null;
            description?: string | null;
            is_active?: boolean | null;
            is_default?: boolean | null;
            /** @example wholesale */
            key?: string | null;
            /** @example Wholesale */
            name?: string | null;
            segment_id?: string | null;
            /** @example 2026-01-01 */
            valid_from?: string | null;
            /** @example 2026-12-31 */
            valid_to?: string | null;
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
        Page_PriceListResponse: {
            data: {
                applies_to: components["schemas"]["PriceListAppliesTo"];
                company_ids: string[];
                /** @example 2026-10-22T13:45:00Z */
                created_at: string;
                created_by?: string | null;
                /** @example BRL */
                currency: string;
                description?: string | null;
                id: string;
                is_active: boolean;
                is_default: boolean;
                /** Format: int64 */
                item_count: number;
                /** @example wholesale */
                key: string;
                /** @example Wholesale */
                name: string;
                segment_id?: string | null;
                /** @example 2026-10-22T13:45:00Z */
                updated_at: string;
                /** @example 2026-01-01 */
                valid_from?: string | null;
                /** @example 2026-12-31 */
                valid_to?: string | null;
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
    price_lists_list: {
        parameters: {
            query?: {
                /** @example BRL */
                currency?: string;
                active?: boolean;
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
            /** @description Price lists of the organization, the default of each currency first, then by name. `currency`, `active` and `search` (name or key) narrow the page */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page_PriceListResponse"];
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
            /** @description Missing `catalog_items.read` with organization scope (`permission_denied`, `insufficient_scope`) */
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
    price_lists_create: {
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
                "application/json": components["schemas"]["PriceListRequest"];
            };
        };
        responses: {
            /** @description Price list created (`price_list.created`). `currency` defaults to the organization currency and must be enabled; `applies_to` is `all` (default), `segment` (with a company `segment_id`) or `organizations` (with `company_ids`); `valid_from`/`valid_to` (inclusive, optional) bound when it is in effect. Marking `is_default` (only for `all`) replaces the previous default of the currency. `key` is generated from the name when omitted */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PriceListResponse"];
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
            /** @description Missing `catalog_items.update` with organization scope (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Another price list uses the key (`price_list_key_taken`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid fields (`validation_error` with `details.fields`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    discount_limits_get: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Every role of the organization with its maximum discount in percent (`null` = no limit). A quote whose discount passes the limit of the role of its owner goes through the `quote_discount_limit` approval policy before it can be sent */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_DiscountLimitResponse"];
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
    discount_limits_set: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SetDiscountLimitsRequest"];
            };
        };
        responses: {
            /** @description Limits replaced (`discount_limits.updated` with the changes): roles left out or sent with `max_discount_percent: null` have no limit. Percent between 0 and 100 with up to 2 decimal places */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_DiscountLimitResponse"];
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
            /** @description Unknown role or invalid percent (`validation_error` with `details.fields`) */
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
    products_bundle_get: {
        parameters: {
            query?: {
                price_list_id?: string;
            };
            header?: never;
            path: {
                /** @description Product id */
                product_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Bundle of the product expanded into its components (`data: null` when the product is not a bundle). `fixed` uses the price of the product itself; `sum` adds component price × quantity. With `price_list_id`, prices come from the list in effect today (falling back to the catalog price) and `currency` is the currency of the list */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Option_ProductBundleResponse"];
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
            /** @description Missing `catalog_items.read` with organization scope (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Product or price list not found (`product_not_found`, `price_list_not_found`) */
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
    products_bundle_save: {
        parameters: {
            query?: never;
            header?: {
                /** @description `updated_at` of the bundle as last read (ISO-8601); when it changed since, the update is refused with 409 `version_conflict` */
                "If-Match"?: string | null;
            };
            path: {
                /** @description Product id */
                product_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ProductBundleRequest"];
            };
        };
        responses: {
            /** @description Product turned into a bundle, or its bundle replaced (`product_bundle.updated`). 1 to 50 components, each a different product with a positive quantity; bundles cannot contain bundles (one level only), and with `sum` every component must use the currency of the bundle */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_ProductBundleResponse"];
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
            /** @description Missing `catalog_items.update` with organization scope (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Product not found (`product_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The product is a component of another bundle (`product_bundle_nested`) or the bundle changed after the `If-Match` version (`version_conflict`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid pricing or components (`validation_error` with `details.fields`) */
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
    products_bundle_delete: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Product id */
                product_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description The product is no longer a bundle (`product_bundle.removed`); the product itself is kept */
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
            /** @description Missing `catalog_items.update` with organization scope (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Product not found or not a bundle (`product_not_found`, `product_bundle_not_found`) */
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
    price_lists_get: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Price list id */
                price_list_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Price list */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PriceListResponse"];
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
            /** @description Missing `catalog_items.read` with organization scope (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Price list not found (`price_list_not_found`) */
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
    price_lists_delete: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Price list id */
                price_list_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Price list and its items removed (`price_list.deleted`). Quotes that used it keep their prices */
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
            /** @description Missing `catalog_items.update` with organization scope (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Price list not found (`price_list_not_found`) */
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
    price_lists_update: {
        parameters: {
            query?: never;
            header?: {
                /** @description `updated_at` of the price list as last read (ISO-8601); when it changed since, the update is refused with 409 `version_conflict` */
                "If-Match"?: string | null;
            };
            path: {
                /** @description Price list id */
                price_list_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["PriceListRequest"];
            };
        };
        responses: {
            /** @description Price list updated (`price_list.updated` with the changes); only the fields sent change */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PriceListResponse"];
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
            /** @description Missing `catalog_items.update` with organization scope (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Price list not found (`price_list_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Another price list uses the key (`price_list_key_taken`) or the list changed after the `If-Match` version (`version_conflict`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid fields (`validation_error` with `details.fields`) */
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
    price_lists_items_list: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Price list id */
                price_list_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Every price of the list, with the product name, code and catalog price for reference. A product can have several tiers (`min_quantity`) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_PriceListItemResponse"];
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
            /** @description Missing `catalog_items.read` with organization scope (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Price list not found (`price_list_not_found`) */
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
    price_lists_items_replace: {
        parameters: {
            query?: never;
            header?: {
                /** @description `updated_at` of the price list as last read; replacing the items bumps it */
                "If-Match"?: string | null;
            };
            path: {
                /** @description Price list id */
                price_list_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ReplacePriceListItemsRequest"];
            };
        };
        responses: {
            /** @description Items replaced in bulk by the list sent (`price_list.items_replaced`; at most 5000). `min_quantity` defaults to 1 and is unique per product: a quantity uses the tier with the largest `min_quantity` not above it, or the smallest tier when it is below all of them */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_PriceListItemResponse"];
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
            /** @description Missing `catalog_items.update` with organization scope (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Price list not found (`price_list_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The list changed after the `If-Match` version (`version_conflict`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid items (`validation_error` with `details.fields` as `items.<index>.<field>`) */
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

export interface paths {
    "/api/v1/deals/{deal_id}/quotes": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Current version of every quote of a deal, newest number first (permissions `quotes.read` and read access to the deal). */
        get: operations["quotes_for_deal"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/public/quotes/{token}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Quote behind a signed public link (no session): organization, number, version, items, totals, validity, terms and notes. `state` is `open` (can be accepted or rejected), `accepted`, `rejected`, `expired` or `superseded` (a newer version replaced it). The first view of an open quote publishes `quote.viewed`. */
        get: operations["quotes_public_get"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/public/quotes/{token}/accept": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Accepts an open quote (no session, no `X-Requested-With` needed) with the name and e-mail of who accepts; the IP address, user agent and time are recorded. Publishes `quote.accepted` (and `deal.quote_accepted`) and queues the synchronization of the deal: its items are replaced by the quote lines and its amount becomes the quote total (or the sum of the items when the amount follows the items); when configured, the deal moves to the acceptance stage of its pipeline (a failure there is recorded and published as `quote.deal_sync_failed`, the acceptance is kept). */
        post: operations["quotes_public_accept"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/public/quotes/{token}/reject": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Rejects an open quote with an optional reason (no session). Publishes `quote.rejected` (and `deal.quote_rejected`). */
        post: operations["quotes_public_reject"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/quote-settings": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Quote settings of the organization: default validity in days, default terms and, per pipeline, the stage the deal moves to when a quote is accepted (`settings.manage` or `quotes.read`). */
        get: operations["quote_settings_get"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        /** @description Updates the quote settings (`settings.manage`) and publishes `quote_settings.updated`. `accept_stages` replaces the whole map `{ "<pipeline_id>": "<stage_id>" }` (a `null` stage removes the pipeline). */
        patch: operations["quote_settings_update"];
        trace?: never;
    };
    "/api/v1/quotes": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Lists quotes (permission `quotes.read`, scoped by owner). Only the current version of each quote is listed unless `include_versions=true`. Filters accept the JSON AST in `filter` or `filter[field][operator]=value` (`filter[status][eq]=sent`, `filter[deal_id][eq]=<id>`, `filter[total][gte]=1000`, `filter[exceeds_discount_limit][eq]=true`). `sort=field,-other`; `search` (or `q`) matches the title. */
        get: operations["quotes_list"];
        put?: never;
        /** @description Creates a quote for a deal (permissions `quotes.create` and read access to the deal) with the next number of the organization, version 1 and status `draft`, and publishes `quote.created`. The currency, contact and company come from the deal; the owner defaults to the caller; `valid_until` and `terms` default to the quote settings; the price list defaults to the best active list for the currency and company. `from_line_items: true` copies the deal items; otherwise send `items`. Items with `product_id` and no `unit_price` take the price of the chosen price list (or the product price); bundles enter as one line with their components. Totals are computed on the server: line = quantity × unit price − line discount; the total discount applies to the sum of the lines after their discounts and is prorated among them before the tax; tax = line tax % over the prorated line. */
        post: operations["quotes_create"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/quotes/{quote_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Returns a quote version with its items, totals, discount limit of the owner, versions, public link (after sending) and custom fields (permission `quotes.read`). */
        get: operations["quotes_get"];
        put?: never;
        post?: never;
        delete: operations["quotes_delete"];
        options?: never;
        head?: never;
        /** @description Partially updates the current version of a quote in `draft` or `approved` (permission `quotes.update`) and publishes `quote.updated` with the changes. `items`, when sent, replaces every line. `reprice: true` (or changing `price_list_id`) prices the product lines again. Changing an `approved` quote takes it back to `draft` and cancels its open approval requests. Send `If-Match` with the `updated_at` you read. */
        patch: operations["quotes_update"];
        trace?: never;
    };
    "/api/v1/quotes/{quote_id}/accept": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Records the acceptance of the current version of a quote by the seller on behalf of the customer (permission `quotes.update`), for an acceptance received outside the public link (signed paper, phone, e-mail). Works on `sent`, `approved` and `draft` quotes; a `draft` or `approved` quote passes the approval checkpoint of `quote_send` first (`approval_required`), like sending. `name` defaults to the quote contact and `email` to the contact e-mail. The status becomes `accepted`, the hook `QuoteAccepting` runs, `quote.accepted` is published with `manual: true` and `recorded_by` (also `deal.quote_accepted`) and the deal is synchronized exactly as in the public acceptance (job `quote.deal_sync`). */
        post: operations["quotes_accept"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/quotes/{quote_id}/link": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Signed public link of a sent quote version (permission `quotes.read`). The link carries no internal id and only shows that version. */
        get: operations["quotes_link"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/quotes/{quote_id}/pdf": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description PDF document of a quote version (permission `quotes.read`), generated on the fly: organization, number, version, status, contact, company, owner, validity, items, totals, terms and notes. Totals hidden from the caller by field permissions are left out. */
        get: operations["quotes_pdf"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/quotes/{quote_id}/send": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Queues the quote e-mail from the caller's mailbox (permissions `quotes.send` and `emails.send`), linked to the deal, with the signed public link in the body (`{{quote_link}}` marks where; otherwise it is appended). Without `to`, the deal contact receives it. A `draft` or `approved` quote passes the approval checkpoint of `quote_send` first: a quote that needs approval cannot be sent before it (`approval_required`), and the granted approval is consumed. A `sent` quote can be sent again. Queue acceptance changes the status to `sent` and publishes `quote.sent` (also `deal.quote_sent` on the deal timeline); the delivery job sends the e-mail afterwards. */
        post: operations["quotes_send"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/quotes/{quote_id}/submit": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Sends a `draft` quote for approval (permission `quotes.update`) through the approval policies of the moment `quote_send` (such as the default discount limit policy, based on `exceeds_discount_limit`). When no policy applies, or an approval is already granted, the quote becomes `approved`; otherwise `pending_approval` until the approvers decide (`approval.approved` → `approved`, `approval.rejected`/`approval.cancelled` → `draft`). Publishes `quote.submitted` (and `quote.approved` when approved at once). */
        post: operations["quotes_submit"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/quotes/{quote_id}/versions": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Every version of the quote number, newest first (permission `quotes.read`). */
        get: operations["quotes_versions"];
        put?: never;
        /** @description Creates a new version from the current one (permissions `quotes.create` and `quotes.update`): same number, `version` + 1, status `draft`, copy of the header, items and custom fields. The previous version becomes read-only, its public link shows it was replaced and its open approval requests are cancelled. Publishes `quote.version_created`. */
        post: operations["quotes_create_version"];
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
        PublicQuoteResponse: {
            accepted_at?: string | null;
            accepted_name?: string | null;
            company_name?: string | null;
            contact_name?: string | null;
            currency: string;
            discount_total: string;
            discount_type: components["schemas"]["QuoteDiscountType"];
            discount_value: string;
            items: components["schemas"]["PublicQuoteItemResponse"][];
            items_discount: string;
            notes?: string | null;
            /** Format: int32 */
            number: number;
            organization: components["schemas"]["PublicQuoteOrganization"];
            owner_email?: string | null;
            owner_name?: string | null;
            rejected_at?: string | null;
            rejection_reason?: string | null;
            sent_at?: string | null;
            state: components["schemas"]["PublicQuoteState"];
            subtotal: string;
            tax_total: string;
            terms?: string | null;
            title: string;
            total: string;
            total_discount: string;
            valid_until?: string | null;
            /** Format: int32 */
            version: number;
        };
        /** @enum {string} */
        PublicQuoteState: "open" | "accepted" | "rejected" | "expired" | "superseded";
        PublicQuoteOrganization: {
            name: string;
            primary_color?: string | null;
        };
        PublicQuoteItemResponse: {
            components: components["schemas"]["QuoteItemComponentResponse"][];
            description?: string | null;
            discount_amount: string;
            discount_type: components["schemas"]["QuoteDiscountType"];
            discount_value: string;
            name: string;
            quantity: string;
            tax_amount: string;
            tax_percent: string;
            total: string;
            unit?: string | null;
            unit_price: string;
        };
        /** @enum {string} */
        QuoteDiscountType: "none" | "percent" | "amount";
        QuoteItemComponentResponse: {
            code?: string | null;
            name: string;
            product_id?: string | null;
            /** @example 1 */
            quantity: string;
            /** @example 100.00 */
            unit_price: string;
        };
        QuoteSettingsResponse: {
            accept_stages: Record<string, never>;
            default_terms?: string | null;
            /** Format: int32 */
            default_valid_days?: number | null;
            updated_at?: string | null;
        };
        EmailAddressInput: {
            /** @example maria@customer.com */
            address: string;
            name?: string | null;
        };
        QuoteItemRequest: {
            description?: string | null;
            discount_type?: null | components["schemas"]["QuoteDiscountType"];
            discount_value?: null | components["schemas"]["DecimalInput"];
            /** @example Onboarding */
            name?: string | null;
            product_id?: string | null;
            quantity?: null | components["schemas"]["DecimalInput"];
            recurrence?: string | null;
            tax_percent?: null | components["schemas"]["DecimalInput"];
            /**
             * Format: int64
             * @example 12
             */
            term_months?: number | null;
            unit?: string | null;
            unit_price?: null | components["schemas"]["DecimalInput"];
        };
        /** @example 15000.00 */
        DecimalInput: string | number;
        QuoteDetailResponse: {
            acceptance?: null | components["schemas"]["QuoteAcceptanceResponse"];
            accepted_at?: string | null;
            company_id?: string | null;
            contact_id?: string | null;
            created_at: string;
            created_by?: string | null;
            currency: string;
            custom_fields: Record<string, never>;
            deal_id: string;
            deal_sync_error?: string | null;
            deal_synced_at?: string | null;
            discount_limit?: string | null;
            discount_total: string;
            discount_type: components["schemas"]["QuoteDiscountType"];
            discount_value: string;
            exceeds_discount_limit: boolean;
            expired_at?: string | null;
            id: string;
            is_current: boolean;
            items: components["schemas"]["QuoteItemResponse"][];
            items_discount: string;
            /** @example 12.5 */
            max_discount_percent: string;
            notes?: string | null;
            /** Format: int32 */
            number: number;
            owner_id?: string | null;
            price_list_id?: string | null;
            public_url?: string | null;
            rejected_at?: string | null;
            rejection_reason?: string | null;
            sent_at?: string | null;
            sent_by?: string | null;
            status: components["schemas"]["QuoteStatus"];
            subtotal: string;
            tax_total: string;
            terms?: string | null;
            title: string;
            total: string;
            total_discount: string;
            updated_at: string;
            valid_until?: string | null;
            /** Format: int32 */
            version: number;
            versions: components["schemas"]["QuoteVersionResponse"][];
            viewed_at?: string | null;
        };
        QuoteVersionResponse: {
            created_at: string;
            id: string;
            is_current: boolean;
            status: components["schemas"]["QuoteStatus"];
            total: string;
            /** Format: int32 */
            version: number;
        };
        /** @enum {string} */
        QuoteStatus: "draft" | "pending_approval" | "approved" | "sent" | "accepted" | "rejected" | "expired";
        QuoteItemResponse: {
            components: components["schemas"]["QuoteItemComponentResponse"][];
            description?: string | null;
            /** @example 300.00 */
            discount_amount: string;
            discount_type: components["schemas"]["QuoteDiscountType"];
            /** @example 10.00 */
            discount_value: string;
            /** @example 3000.00 */
            gross: string;
            id: string;
            name: string;
            /** Format: int32 */
            position: number;
            product_id?: string | null;
            /** @example 2 */
            quantity: string;
            recurrence: string;
            /** @example 135.00 */
            tax_amount: string;
            /** @example 5 */
            tax_percent: string;
            /** Format: int32 */
            term_months?: number | null;
            /** @example 2835.00 */
            total: string;
            /** @example 0.00 */
            total_discount_share: string;
            unit?: string | null;
            /** @example 1500.00 */
            unit_price: string;
        };
        QuoteAcceptanceResponse: {
            accepted_at?: string | null;
            email: string;
            ip?: string | null;
            name: string;
            recorded_by?: string | null;
            user_agent?: string | null;
        };
        QuoteResponse: {
            accepted_at?: string | null;
            company_id?: string | null;
            contact_id?: string | null;
            created_at: string;
            created_by?: string | null;
            /** @example BRL */
            currency: string;
            custom_fields: Record<string, never>;
            deal_id: string;
            /** @example 300.00 */
            discount_total: string;
            exceeds_discount_limit: boolean;
            expired_at?: string | null;
            id: string;
            is_current: boolean;
            /**
             * Format: int32
             * @example 42
             */
            number: number;
            owner_id?: string | null;
            price_list_id?: string | null;
            rejected_at?: string | null;
            sent_at?: string | null;
            status: components["schemas"]["QuoteStatus"];
            /** @example 3000.00 */
            subtotal: string;
            /** @example 135.00 */
            tax_total: string;
            title: string;
            /** @example 2835.00 */
            total: string;
            updated_at: string;
            /** @example 2026-11-30 */
            valid_until?: string | null;
            /**
             * Format: int32
             * @example 1
             */
            version: number;
            viewed_at?: string | null;
        };
        Data_Vec_QuoteVersionResponse: {
            data: {
                created_at: string;
                id: string;
                is_current: boolean;
                status: components["schemas"]["QuoteStatus"];
                total: string;
                /** Format: int32 */
                version: number;
            }[];
        };
        SubmitQuoteRequest: {
            comment?: string | null;
        };
        SendQuoteRequest: {
            body_html?: string | null;
            cc?: components["schemas"]["EmailAddressInput"][];
            mailbox_id?: string | null;
            subject?: string | null;
            to?: components["schemas"]["EmailAddressInput"][];
        };
        Data_QuoteLinkResponse: {
            data: {
                url: string;
            };
        };
        QuoteLinkResponse: {
            url: string;
        };
        AcceptQuoteManuallyRequest: {
            email?: string | null;
            name?: string | null;
            note?: string | null;
        };
        Data_QuoteDetailResponse: {
            data: {
                acceptance?: null | components["schemas"]["QuoteAcceptanceResponse"];
                accepted_at?: string | null;
                company_id?: string | null;
                contact_id?: string | null;
                created_at: string;
                created_by?: string | null;
                currency: string;
                custom_fields: Record<string, never>;
                deal_id: string;
                deal_sync_error?: string | null;
                deal_synced_at?: string | null;
                discount_limit?: string | null;
                discount_total: string;
                discount_type: components["schemas"]["QuoteDiscountType"];
                discount_value: string;
                exceeds_discount_limit: boolean;
                expired_at?: string | null;
                id: string;
                is_current: boolean;
                items: components["schemas"]["QuoteItemResponse"][];
                items_discount: string;
                /** @example 12.5 */
                max_discount_percent: string;
                notes?: string | null;
                /** Format: int32 */
                number: number;
                owner_id?: string | null;
                price_list_id?: string | null;
                public_url?: string | null;
                rejected_at?: string | null;
                rejection_reason?: string | null;
                sent_at?: string | null;
                sent_by?: string | null;
                status: components["schemas"]["QuoteStatus"];
                subtotal: string;
                tax_total: string;
                terms?: string | null;
                title: string;
                total: string;
                total_discount: string;
                updated_at: string;
                valid_until?: string | null;
                /** Format: int32 */
                version: number;
                versions: components["schemas"]["QuoteVersionResponse"][];
                viewed_at?: string | null;
            };
        };
        QuoteRequest: {
            custom_fields?: Record<string, never> | null;
            deal_id?: string | null;
            discount_type?: null | components["schemas"]["QuoteDiscountType"];
            discount_value?: null | components["schemas"]["DecimalInput"];
            from_line_items?: boolean | null;
            items?: components["schemas"]["QuoteItemRequest"][] | null;
            notes?: string | null;
            owner_id?: string | null;
            price_list_id?: string | null;
            reprice?: boolean | null;
            terms?: string | null;
            /** @example Onboarding proposal */
            title?: string | null;
            /** @example 2026-11-30 */
            valid_until?: string | null;
        };
        Page_QuoteResponse: {
            data: {
                accepted_at?: string | null;
                company_id?: string | null;
                contact_id?: string | null;
                created_at: string;
                created_by?: string | null;
                /** @example BRL */
                currency: string;
                custom_fields: Record<string, never>;
                deal_id: string;
                /** @example 300.00 */
                discount_total: string;
                exceeds_discount_limit: boolean;
                expired_at?: string | null;
                id: string;
                is_current: boolean;
                /**
                 * Format: int32
                 * @example 42
                 */
                number: number;
                owner_id?: string | null;
                price_list_id?: string | null;
                rejected_at?: string | null;
                sent_at?: string | null;
                status: components["schemas"]["QuoteStatus"];
                /** @example 3000.00 */
                subtotal: string;
                /** @example 135.00 */
                tax_total: string;
                title: string;
                /** @example 2835.00 */
                total: string;
                updated_at: string;
                /** @example 2026-11-30 */
                valid_until?: string | null;
                /**
                 * Format: int32
                 * @example 1
                 */
                version: number;
                viewed_at?: string | null;
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
        QuoteSettingsRequest: {
            accept_stages?: Record<string, never> | null;
            default_terms?: string | null;
            /**
             * Format: int32
             * @example 15
             */
            default_valid_days?: number | null;
        };
        Data_QuoteSettingsResponse: {
            data: {
                accept_stages: Record<string, never>;
                default_terms?: string | null;
                /** Format: int32 */
                default_valid_days?: number | null;
                updated_at?: string | null;
            };
        };
        PublicRejectQuoteRequest: {
            reason?: string | null;
        };
        PublicAcceptQuoteRequest: {
            /** @example maria@customer.com */
            email: string;
            /** @example Maria Souza */
            name: string;
        };
        Data_PublicQuoteResponse: {
            data: {
                accepted_at?: string | null;
                accepted_name?: string | null;
                company_name?: string | null;
                contact_name?: string | null;
                currency: string;
                discount_total: string;
                discount_type: components["schemas"]["QuoteDiscountType"];
                discount_value: string;
                items: components["schemas"]["PublicQuoteItemResponse"][];
                items_discount: string;
                notes?: string | null;
                /** Format: int32 */
                number: number;
                organization: components["schemas"]["PublicQuoteOrganization"];
                owner_email?: string | null;
                owner_name?: string | null;
                rejected_at?: string | null;
                rejection_reason?: string | null;
                sent_at?: string | null;
                state: components["schemas"]["PublicQuoteState"];
                subtotal: string;
                tax_total: string;
                terms?: string | null;
                title: string;
                total: string;
                total_discount: string;
                valid_until?: string | null;
                /** Format: int32 */
                version: number;
            };
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
        Data_Vec_QuoteResponse: {
            data: {
                accepted_at?: string | null;
                company_id?: string | null;
                contact_id?: string | null;
                created_at: string;
                created_by?: string | null;
                /** @example BRL */
                currency: string;
                custom_fields: Record<string, never>;
                deal_id: string;
                /** @example 300.00 */
                discount_total: string;
                exceeds_discount_limit: boolean;
                expired_at?: string | null;
                id: string;
                is_current: boolean;
                /**
                 * Format: int32
                 * @example 42
                 */
                number: number;
                owner_id?: string | null;
                price_list_id?: string | null;
                rejected_at?: string | null;
                sent_at?: string | null;
                status: components["schemas"]["QuoteStatus"];
                /** @example 3000.00 */
                subtotal: string;
                /** @example 135.00 */
                tax_total: string;
                title: string;
                /** @example 2835.00 */
                total: string;
                updated_at: string;
                /** @example 2026-11-30 */
                valid_until?: string | null;
                /**
                 * Format: int32
                 * @example 1
                 */
                version: number;
                viewed_at?: string | null;
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
    quotes_for_deal: {
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
            /** @description Quotes of the deal */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_QuoteResponse"];
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
            /** @description Missing `quotes.read` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Deal not found (`deal_not_found`) */
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
    quotes_public_get: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Signed public token */
                token: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Quote */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PublicQuoteResponse"];
                };
            };
            /** @description Invalid token (`quote_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Too many requests from this address (`rate_limited`) */
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
    quotes_public_accept: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Signed public token */
                token: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["PublicAcceptQuoteRequest"];
            };
        };
        responses: {
            /** @description Accepted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PublicQuoteResponse"];
                };
            };
            /** @description Invalid token (`quote_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Already answered (`quote_closed`), expired (`quote_expired`) or replaced by a newer version (`quote_superseded`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid name or e-mail (`validation_error`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Too many attempts (`rate_limited`) */
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
    quotes_public_reject: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Signed public token */
                token: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["PublicRejectQuoteRequest"];
            };
        };
        responses: {
            /** @description Rejected */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PublicQuoteResponse"];
                };
            };
            /** @description Invalid token (`quote_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Already answered (`quote_closed`), expired (`quote_expired`) or replaced (`quote_superseded`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Reason too long (`validation_error`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Too many attempts (`rate_limited`) */
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
    quote_settings_get: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Settings */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_QuoteSettingsResponse"];
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
            /** @description Missing permission (`permission_denied`, `insufficient_scope`) */
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
    quote_settings_update: {
        parameters: {
            query?: never;
            header?: {
                /** @description `updated_at` of the settings as read */
                "If-Match"?: string | null;
            };
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["QuoteSettingsRequest"];
            };
        };
        responses: {
            /** @description Settings updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_QuoteSettingsResponse"];
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
            /** @description Changed since `If-Match` (`version_conflict`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid settings (`validation_error`) */
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
    quotes_list: {
        parameters: {
            query?: {
                /** @example {"op":"and","conditions":[{"field":"name","operator":"contains","value":"ana"}]} */
                filter?: string;
                /** @example -created_at,name */
                sort?: string;
                search?: string;
                /** @example exclude */
                archived?: string;
                /** @example acme */
                q?: string;
                include_versions?: boolean;
                page?: number;
                per_page?: number;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Page of quotes */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page_QuoteResponse"];
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
            /** @description Missing `quotes.read` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid filter, sort, search or pagination (`validation_error`, `field_not_accessible`) */
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
    quotes_create: {
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
                "application/json": components["schemas"]["QuoteRequest"];
            };
        };
        responses: {
            /** @description Quote created */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_QuoteDetailResponse"];
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
            /** @description Missing `quotes.create` (`permission_denied`, `insufficient_scope`) */
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
            /** @description A pending approval locks the write (`approval_pending`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid fields or items (`validation_error` with `details.fields` such as `items.0.unit_price`), unknown price list (`quote_price_list_not_found`), too many items (`quote_items_limit_reached`), fields the role cannot write (`field_not_accessible`, `field_read_only`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    quotes_get: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Quote id (one version) */
                quote_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description The quote */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_QuoteDetailResponse"];
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
            /** @description Missing `quotes.read` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Quote not found (`quote_not_found`) */
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
    quotes_delete: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Quote id (any version) */
                quote_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Quote deleted with all its versions (soft delete); publishes `quote.deleted` for each version and invalidates the public link */
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
            /** @description Missing `quotes.delete` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Quote not found (`quote_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description A version was accepted (`quote_already_accepted`) or a link blocks the deletion (`record_in_use`) */
            409: {
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
    quotes_update: {
        parameters: {
            query?: never;
            header?: {
                /** @description `updated_at` of the quote as read; 409 when it changed since */
                "If-Match"?: string | null;
            };
            path: {
                /** @description Quote id */
                quote_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["QuoteRequest"];
            };
        };
        responses: {
            /** @description Quote updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_QuoteDetailResponse"];
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
            /** @description Missing `quotes.update` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Quote not found (`quote_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Old version (`quote_not_current`), sent or closed (`quote_not_editable`), waiting for approval (`approval_pending`) or changed since `If-Match` (`version_conflict`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid fields or items (`validation_error`, `quote_price_list_not_found`, `quote_items_limit_reached`, `field_not_accessible`, `field_read_only`) */
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
    quotes_accept: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Quote id */
                quote_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["AcceptQuoteManuallyRequest"];
            };
        };
        responses: {
            /** @description Quote after the acceptance */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_QuoteDetailResponse"];
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
            /** @description Missing `quotes.update` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Quote not found (`quote_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Needs approval (`approval_required`), waiting for approval (`approval_pending`), expired (`quote_expired`), old version (`quote_not_current`) or already answered (`quote_invalid_transition`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid name, e-mail or note (`validation_error`) or no items (`quote_has_no_items`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    quotes_link: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Quote id */
                quote_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Public link */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_QuoteLinkResponse"];
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
            /** @description Missing `quotes.read` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Quote not found (`quote_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The quote was not sent yet (`quote_not_sent`) */
            409: {
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
    quotes_pdf: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Quote id */
                quote_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description PDF served as an attachment */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/pdf": unknown;
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
            /** @description Missing `quotes.read` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Quote not found (`quote_not_found`) */
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
    quotes_send: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Quote id */
                quote_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SendQuoteRequest"];
            };
        };
        responses: {
            /** @description Quote e-mail queued */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_QuoteDetailResponse"];
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
            /** @description Missing `quotes.send` or `emails.send` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Quote not found (`quote_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Needs approval (`approval_required`), waiting for approval (`approval_pending`), expired (`quote_expired`), old version (`quote_not_current`) or closed (`quote_invalid_transition`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description No recipient or invalid e-mail (`validation_error`), no items (`quote_has_no_items`) or no mailbox to send from. `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    quotes_submit: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Quote id */
                quote_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SubmitQuoteRequest"];
            };
        };
        responses: {
            /** @description Quote after the submission */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_QuoteDetailResponse"];
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
            /** @description Missing `quotes.update` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Quote not found (`quote_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Not a current draft (`quote_invalid_transition`, `quote_not_current`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description No items (`quote_has_no_items`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    quotes_versions: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Quote id (any version) */
                quote_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Versions */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_QuoteVersionResponse"];
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
            /** @description Missing `quotes.read` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Quote not found (`quote_not_found`) */
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
    quotes_create_version: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Id of the current version */
                quote_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description New version */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_QuoteDetailResponse"];
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
            /** @description Missing permission (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Quote not found (`quote_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Not the current version (`quote_not_current`) or already accepted (`quote_already_accepted`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
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
}

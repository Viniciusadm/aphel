export interface paths {
    "/api/v1/organizations": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Lists the organizations visible to the user (permission `organizations.read`, limited by its record scope). Filters accept the JSON AST in `filter` or simple conditions as `filter[field][operator]=value` (`filter[industry][eq]=Varejo`, `filter[owner_id][in]=a,b`, `filter[domain]=acme.com.br` means `eq`), combined with `and`. `sort=field,-other` (up to 3); without `sort`, newest first. `search` (or its alias `q`) matches name, domain, email and phone; phone terms are normalized to E.164. `archived=exclude|include|only` (default `exclude`). The owner is always embedded; Enabled `include` contributions can enrich each company. */
        get: operations["companies_list"];
        put?: never;
        /** @description Creates a company (permission `organizations.create`) and publishes `company.created`. Email is lowercased, phone normalized to E.164 and the domain taken from the website when missing; the owner defaults to the current user. Custom fields go in `custom_fields`. `possible_duplicates` (only in this response, empty when none) lists visible organizations that the duplicate rules match (a warning, never an error); those pairs also enter the duplicate queue. */
        post: operations["companies_create"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/organizations/{company_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Returns a company with its custom fields and owner (permission `organizations.read`). The id of a company merged into another one answers the company that absorbed it (compare `id`). Enabled `include` contributions can enrich this response. */
        get: operations["companies_get"];
        put?: never;
        post?: never;
        delete: operations["companies_delete"];
        options?: never;
        head?: never;
        /** @description Partially updates a company (permission `organizations.update` on this company) and publishes `company.updated` with the changes. Omitted properties stay unchanged and `null` clears an optional one; `custom_fields` only touches the keys sent. Send `If-Match` with the `updated_at` you read to avoid overwriting someone else's change. */
        patch: operations["companies_update"];
        trace?: never;
    };
    "/api/v1/organizations/{company_id}/archive": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["companies_archive"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/organizations/{company_id}/group-totals": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Totals of a company and all its subsidiaries, at any level (permission `organizations.read`). Only records the caller can read count: `companies_count` (readable organizations of the group, the company included), `contacts_count` (people whose company is in the group; null without `people.read`), `open_deals_count`/`open_deals_amount` and `won_deals_count`/`won_amount` (deals whose company is in the group, amounts summed over `amount_base` in the base `currency` of the organization; null without `deals.read`, amounts null when `amount_base` is hidden). Archived people and deals are left out. */
        get: operations["companies_group_totals"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/organizations/{company_id}/hierarchy": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Company group of a company (permission `organizations.read`): `root_id` is the top company of the group and `nodes` lists the group depth-first (siblings by name), each with `parent_id` and `depth` (0 = root). Companies the caller cannot read are left out, except when they sit on the path between the root and a readable company: then they appear with `accessible = false` and no data, so the tree keeps its shape. `contacts_count`, `open_deals_count` and `open_deals_amount` (sum of `amount_base`, in the base `currency` of the organization) count only records the caller can read; they are null without `people.read`/`deals.read` (or when `amount_base` is hidden). `subsidiaries` counts the direct subsidiaries listed. Up to 1000 organizations (`truncated` tells when there are more). `max_depth` is the deepest group allowed when setting `parent_company_id`. */
        get: operations["companies_hierarchy"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/organizations/{company_id}/unarchive": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["companies_unarchive"];
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
        CompanyGroupTotalsResponse: {
            /** Format: int64 */
            companies_count: number;
            company_id: string;
            /** Format: int64 */
            contacts_count?: number | null;
            /** @example BRL */
            currency?: string | null;
            /** @example 15000.00 */
            open_deals_amount?: string | null;
            /** Format: int64 */
            open_deals_count?: number | null;
            /** @example 42000.00 */
            won_amount?: string | null;
            /** Format: int64 */
            won_deals_count?: number | null;
        };
        CompanyHierarchyResponse: {
            company_id: string;
            /** @example BRL */
            currency?: string | null;
            /** @example 10 */
            max_depth: number;
            nodes: components["schemas"]["CompanyHierarchyNodeResponse"][];
            root_id: string;
            truncated: boolean;
        };
        CompanyHierarchyNodeResponse: {
            accessible: boolean;
            archived: boolean;
            /** Format: int64 */
            contacts_count?: number | null;
            /**
             * Format: int32
             * @example 0
             */
            depth: number;
            id: string;
            /** @example Acme Holding */
            name?: string | null;
            /** @example 15000.00 */
            open_deals_amount?: string | null;
            /** Format: int64 */
            open_deals_count?: number | null;
            owner?: null | components["schemas"]["OwnerResponse"];
            parent_id?: string | null;
            /** Format: int32 */
            subsidiaries: number;
        };
        OwnerResponse: {
            display_name: string;
            id: string;
        };
        UserReferenceResponse: {
            display_name: string;
            id: string;
        };
        Data_CompanyHierarchyResponse: {
            data: {
                company_id: string;
                /** @example BRL */
                currency?: string | null;
                /** @example 10 */
                max_depth: number;
                nodes: components["schemas"]["CompanyHierarchyNodeResponse"][];
                root_id: string;
                truncated: boolean;
            };
        };
        Data_CompanyGroupTotalsResponse: {
            data: {
                /** Format: int64 */
                companies_count: number;
                company_id: string;
                /** Format: int64 */
                contacts_count?: number | null;
                /** @example BRL */
                currency?: string | null;
                /** @example 15000.00 */
                open_deals_amount?: string | null;
                /** Format: int64 */
                open_deals_count?: number | null;
                /** @example 42000.00 */
                won_amount?: string | null;
                /** Format: int64 */
                won_deals_count?: number | null;
            };
        };
        Data_CompanyResponse: {
            data: {
                address_city?: string | null;
                address_complement?: string | null;
                address_country?: string | null;
                address_district?: string | null;
                address_number?: string | null;
                address_postal_code?: string | null;
                address_state?: string | null;
                address_street?: string | null;
                /** @example 2026-10-20T13:45:00Z */
                archived_at?: string | null;
                /** @example 2026-10-20T13:45:00Z */
                created_at: string;
                created_by?: string | null;
                custom_fields: Record<string, never>;
                description?: string | null;
                domain?: string | null;
                email?: string | null;
                id: string;
                industry?: string | null;
                /** Format: int64 */
                interaction_count: number;
                /** @example 2026-10-20T13:45:00Z */
                last_contacted_at?: string | null;
                last_contacted_by?: string | null;
                /** @example 2026-10-20T13:45:00Z */
                last_email_received_at?: string | null;
                /** @example 2026-10-20T13:45:00Z */
                last_email_sent_at?: string | null;
                name: string;
                owner?: null | components["schemas"]["OwnerResponse"];
                owner_id?: string | null;
                parent_company_id?: string | null;
                phone?: string | null;
                possible_duplicates?: components["schemas"]["DuplicateMatchResponse"][] | null;
                size_range?: string | null;
                source?: string | null;
                /** @example 2026-10-20T13:45:00Z */
                updated_at: string;
                website?: string | null;
            } & {
                [key: string]: unknown;
            };
        };
        DuplicateMatchResponse: {
            record_id: string;
            /**
             * @example [
             *       "email"
             *     ]
             */
            rules: string[];
            title: string;
        };
        CompanyResponse: {
            address_city?: string | null;
            address_complement?: string | null;
            address_country?: string | null;
            address_district?: string | null;
            address_number?: string | null;
            address_postal_code?: string | null;
            address_state?: string | null;
            address_street?: string | null;
            /** @example 2026-10-20T13:45:00Z */
            archived_at?: string | null;
            /** @example 2026-10-20T13:45:00Z */
            created_at: string;
            created_by?: string | null;
            custom_fields: Record<string, never>;
            description?: string | null;
            domain?: string | null;
            email?: string | null;
            id: string;
            industry?: string | null;
            /** Format: int64 */
            interaction_count: number;
            /** @example 2026-10-20T13:45:00Z */
            last_contacted_at?: string | null;
            last_contacted_by?: string | null;
            /** @example 2026-10-20T13:45:00Z */
            last_email_received_at?: string | null;
            /** @example 2026-10-20T13:45:00Z */
            last_email_sent_at?: string | null;
            name: string;
            owner?: null | components["schemas"]["OwnerResponse"];
            owner_id?: string | null;
            parent_company_id?: string | null;
            phone?: string | null;
            possible_duplicates?: components["schemas"]["DuplicateMatchResponse"][] | null;
            size_range?: string | null;
            source?: string | null;
            /** @example 2026-10-20T13:45:00Z */
            updated_at: string;
            website?: string | null;
        } & {
            [key: string]: unknown;
        };
        CompanyRequest: {
            address_city?: string | null;
            address_complement?: string | null;
            /** @example BR */
            address_country?: string | null;
            address_district?: string | null;
            address_number?: string | null;
            address_postal_code?: string | null;
            address_state?: string | null;
            address_street?: string | null;
            custom_fields?: Record<string, never> | null;
            description?: string | null;
            /** @example acme.com.br */
            domain?: string | null;
            email?: string | null;
            industry?: string | null;
            /** @example Acme Inc. */
            name?: string | null;
            owner_id?: string | null;
            parent_company_id?: string | null;
            /** @example (11) 3333-4444 */
            phone?: string | null;
            /** @example size_11_50 */
            size_range?: string | null;
            source?: string | null;
            /** @example https://www.acme.com.br */
            website?: string | null;
        } & {
            [key: string]: unknown;
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
        Page_CompanyResponse: {
            data: ({
                address_city?: string | null;
                address_complement?: string | null;
                address_country?: string | null;
                address_district?: string | null;
                address_number?: string | null;
                address_postal_code?: string | null;
                address_state?: string | null;
                address_street?: string | null;
                /** @example 2026-10-20T13:45:00Z */
                archived_at?: string | null;
                /** @example 2026-10-20T13:45:00Z */
                created_at: string;
                created_by?: string | null;
                custom_fields: Record<string, never>;
                description?: string | null;
                domain?: string | null;
                email?: string | null;
                id: string;
                industry?: string | null;
                /** Format: int64 */
                interaction_count: number;
                /** @example 2026-10-20T13:45:00Z */
                last_contacted_at?: string | null;
                last_contacted_by?: string | null;
                /** @example 2026-10-20T13:45:00Z */
                last_email_received_at?: string | null;
                /** @example 2026-10-20T13:45:00Z */
                last_email_sent_at?: string | null;
                name: string;
                owner?: null | components["schemas"]["OwnerResponse"];
                owner_id?: string | null;
                parent_company_id?: string | null;
                phone?: string | null;
                possible_duplicates?: components["schemas"]["DuplicateMatchResponse"][] | null;
                size_range?: string | null;
                source?: string | null;
                /** @example 2026-10-20T13:45:00Z */
                updated_at: string;
                website?: string | null;
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
    };
    responses: never;
    parameters: never;
    requestBodies: never;
    headers: never;
    pathItems: never;
}
export type $defs = Record<string, never>;
export interface operations {
    companies_list: {
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
                /** @example company,owner */
                include?: string;
                page?: number;
                per_page?: number;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Page of organizations with their custom fields */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page_CompanyResponse"];
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
            /** @description Missing `organizations.read` (`permission_denied`, `insufficient_scope`) */
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
        };
    };
    companies_create: {
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
                "application/json": components["schemas"]["CompanyRequest"];
            };
        };
        responses: {
            /** @description Company created */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_CompanyResponse"];
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
            /** @description Missing `organizations.create` (`permission_denied`, `insufficient_scope`) */
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
            /** @description Invalid system or custom fields (`validation_error`, fields such as `name` or `custom_fields.<key>`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    companies_get: {
        parameters: {
            query?: {
                /** @example company,owner */
                include?: string;
            };
            header?: never;
            path: {
                /** @description Company id */
                company_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description The company */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_CompanyResponse"];
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
            /** @description Missing `organizations.read` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Company not found or not visible (`company_not_found`) */
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
    companies_delete: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Company id */
                company_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Company moved to the trash (soft delete); publishes `company.deleted`. Restore it through `POST /trash/company/{id}/restore` */
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
            /** @description Missing `organizations.delete` for this company (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Company not found or not visible (`company_not_found`) */
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
    companies_update: {
        parameters: {
            query?: never;
            header?: {
                /** @description `updated_at` of the company as read (ISO-8601, quotes optional); the update fails with 409 when it changed since */
                "If-Match"?: string | null;
            };
            path: {
                /** @description Company id */
                company_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CompanyRequest"];
            };
        };
        responses: {
            /** @description Company updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_CompanyResponse"];
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
            /** @description Missing `organizations.update` for this company (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Company not found or not visible (`company_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The company changed since the `If-Match` version (`version_conflict`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid system or custom fields or malformed `If-Match` (`validation_error`) */
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
    companies_archive: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Company id */
                company_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Company archived (hidden from listings unless `archived=include|only`); publishes `company.archived` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_CompanyResponse"];
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
            /** @description Missing `organizations.update` for this company (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Company not found or not visible (`company_not_found`) */
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
    companies_group_totals: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Company id */
                company_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Totals of the group */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_CompanyGroupTotalsResponse"];
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
            /** @description Missing `organizations.read` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Company not found or not visible (`company_not_found`) */
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
    companies_hierarchy: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Company id */
                company_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Company group */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_CompanyHierarchyResponse"];
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
            /** @description Missing `organizations.read` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Company not found or not visible (`company_not_found`) */
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
    companies_unarchive: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Company id */
                company_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Company restored from the archive; publishes `company.unarchived` */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_CompanyResponse"];
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
            /** @description Missing `organizations.update` for this company (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Company not found or not visible (`company_not_found`) */
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
}

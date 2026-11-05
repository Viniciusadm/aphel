export interface paths {
    "/api/v1/booking/availability": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["booking_availability_get"];
        put: operations["booking_availability_save"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/booking/bookings": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["booking_bookings_list"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/booking/bookings/{booking_id}/cancel": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["booking_bookings_cancel"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/booking/contact-fields": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["booking_contact_fields"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/booking/distribution-rules": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["booking_distribution_rules"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/booking/meeting-types": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["booking_meeting_types_list"];
        put?: never;
        post: operations["booking_meeting_types_create"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/booking/meeting-types/{meeting_type_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["booking_meeting_types_get"];
        put?: never;
        post?: never;
        delete: operations["booking_meeting_types_delete"];
        options?: never;
        head?: never;
        patch: operations["booking_meeting_types_update"];
        trace?: never;
    };
    "/api/v1/public/book/manage/{token}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Booking behind a signed management link (no session): meeting, time, host, location and status; `can_change` tells whether it can still be rescheduled or canceled. */
        get: operations["booking_public_booking"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/public/book/manage/{token}/cancel": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Cancels the booking (no session): cancels the meeting activity, sends the cancellation `.ics` (`METHOD:CANCEL`) to the guest and the host and publishes `meeting.canceled`. */
        post: operations["booking_public_cancel"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/public/book/manage/{token}/reschedule": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Moves the booking to another free slot of the same host (no session): updates the meeting activity, sends the updated `.ics` to the guest and the host and publishes `meeting.rescheduled`. */
        post: operations["booking_public_reschedule"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/public/book/manage/{token}/slots": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Free slots of the same host to reschedule the booking (its own time counts as free). Same rules as the slots of the link. */
        get: operations["booking_public_booking_slots"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/public/book/{slug}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Public data of an active booking link (no session): name, duration, kind of location, host name (null for team links), organization name, host time zone and the questions of the booking form. */
        get: operations["booking_public_meeting_type"];
        put?: never;
        /** @description Books a free slot (no session, no `X-Requested-With` needed). Finds the contact by e-mail or creates it (owned by the host, with the mapped answers), creates a `meeting` activity linked to it and assigned to the host, sends the confirmation with an `.ics` invitation to the guest and the host, and publishes `meeting.booked`. Team links pick the host among the free members by the rotation of the distribution rule. `website` is a honeypot that must stay empty when the link has spam protection. The answer carries `manage_token`, used to reschedule or cancel. */
        post: operations["booking_public_book"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/public/book/{slug}/slots": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Free start times of the link between `from` and `to` (RFC 3339, at most 42 days apart), no session. Only free slots are returned, never anything about the host's other commitments: open tasks with time and duration, other bookings, the connected Google/Microsoft calendars (busy times only), buffers, minimum notice, daily limit and horizon are applied before. For team links the slots of all eligible members are merged. */
        get: operations["booking_public_slots"];
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
        PublicBookingResponse: {
            can_change: boolean;
            cancel_reason?: string | null;
            /** Format: int32 */
            duration_minutes: number;
            ends_at: string;
            guest_email: string;
            guest_name: string;
            host_name: string;
            location_kind: components["schemas"]["MeetingLocationKind"];
            location_value?: string | null;
            manage_token: string;
            meeting_type_name: string;
            organization_name: string;
            slug: string;
            starts_at: string;
            status: components["schemas"]["MeetingBookingStatus"];
            /** @example America/Sao_Paulo */
            timezone: string;
        };
        /** @enum {string} */
        MeetingBookingStatus: "confirmed" | "canceled";
        /** @enum {string} */
        MeetingLocationKind: "none" | "video" | "address" | "phone";
        PublicSlotsResponse: {
            slots: components["schemas"]["PublicSlot"][];
        };
        PublicSlot: {
            ends_at: string;
            starts_at: string;
        };
        PublicMeetingTypeResponse: {
            description?: string | null;
            /** Format: int32 */
            duration_minutes: number;
            host_name?: string | null;
            location_kind: components["schemas"]["MeetingLocationKind"];
            name: string;
            organization_name: string;
            questions: components["schemas"]["PublicMeetingQuestion"][];
            slug: string;
            spam_protection: boolean;
            /** @example America/Sao_Paulo */
            timezone: string;
        };
        PublicMeetingQuestion: {
            key: string;
            kind: components["schemas"]["MeetingQuestionKind"];
            label: string;
            required: boolean;
        };
        /** @enum {string} */
        MeetingQuestionKind: "text" | "long_text";
        MeetingBookingResponse: {
            activity_id?: string | null;
            answers: Record<string, never>;
            cancel_reason?: string | null;
            canceled_at?: string | null;
            contact_id?: string | null;
            created_at: string;
            ends_at: string;
            guest_email: string;
            guest_name: string;
            guest_timezone?: string | null;
            host: components["schemas"]["BookingReference"];
            id: string;
            meeting_type: components["schemas"]["BookingReference"];
            starts_at: string;
            status: components["schemas"]["MeetingBookingStatus"];
        };
        BookingReference: {
            id: string;
            name: string;
        };
        MeetingTypeResponse: {
            /** @example /book/intro-call-ana */
            booking_path: string;
            created_at: string;
            description?: string | null;
            distribution_rule?: null | components["schemas"]["BookingReference"];
            /** Format: int32 */
            duration_minutes: number;
            id: string;
            is_active: boolean;
            /** @example personal */
            kind: string;
            location_kind: components["schemas"]["MeetingLocationKind"];
            location_value?: string | null;
            name: string;
            owner: components["schemas"]["BookingReference"];
            questions: components["schemas"]["MeetingQuestionResponse"][];
            /** Format: int32 */
            slot_interval_minutes: number;
            slug: string;
            spam_protection: boolean;
            updated_at: string;
        };
        MeetingQuestionResponse: {
            contact_field?: string | null;
            key: string;
            kind: components["schemas"]["MeetingQuestionKind"];
            label: string;
            required: boolean;
        };
        MeetingQuestionRequest: {
            /** @example job_title */
            contact_field?: string | null;
            /** @example organizations.organization */
            key?: string | null;
            kind?: null | components["schemas"]["MeetingQuestionKind"];
            /** @example Company */
            label: string;
            required?: boolean;
        };
        BookingContactFieldResponse: {
            is_custom: boolean;
            /** @example job_title */
            key: string;
            /** @example Job title */
            label: string;
        };
        BookingAvailabilityResponse: {
            /** Format: int32 */
            buffer_after_minutes: number;
            /** Format: int32 */
            buffer_before_minutes: number;
            configured: boolean;
            /** Format: int32 */
            horizon_days: number;
            /** Format: int32 */
            max_per_day?: number | null;
            /** Format: int32 */
            min_notice_minutes: number;
            /** @example America/Sao_Paulo */
            timezone: string;
            updated_at?: string | null;
            user_id: string;
            weekly_hours: components["schemas"]["BookingWeeklyInterval"][];
        };
        BookingWeeklyInterval: {
            /** @example 18:00 */
            end: string;
            /** @example 09:00 */
            start: string;
            /**
             * Format: int32
             * @example 1
             */
            weekday: number;
        };
        PublicBookingRequest: {
            answers?: Record<string, never>;
            /** @example maria@customer.com */
            email: string;
            /** @example pt-BR */
            locale?: string | null;
            /** @example Maria Souza */
            name: string;
            /** @example 2026-10-27T13:00:00Z */
            starts_at: string;
            /** @example America/Sao_Paulo */
            timezone?: string | null;
            website?: string | null;
        };
        Data_PublicMeetingTypeResponse: {
            data: {
                description?: string | null;
                /** Format: int32 */
                duration_minutes: number;
                host_name?: string | null;
                location_kind: components["schemas"]["MeetingLocationKind"];
                name: string;
                organization_name: string;
                questions: components["schemas"]["PublicMeetingQuestion"][];
                slug: string;
                spam_protection: boolean;
                /** @example America/Sao_Paulo */
                timezone: string;
            };
        };
        Data_PublicSlotsResponse: {
            data: {
                slots: components["schemas"]["PublicSlot"][];
            };
        };
        PublicRescheduleRequest: {
            /** @example 2026-10-28T14:00:00Z */
            starts_at: string;
        };
        PublicCancelRequest: {
            reason?: string | null;
        };
        Data_PublicBookingResponse: {
            data: {
                can_change: boolean;
                cancel_reason?: string | null;
                /** Format: int32 */
                duration_minutes: number;
                ends_at: string;
                guest_email: string;
                guest_name: string;
                host_name: string;
                location_kind: components["schemas"]["MeetingLocationKind"];
                location_value?: string | null;
                manage_token: string;
                meeting_type_name: string;
                organization_name: string;
                slug: string;
                starts_at: string;
                status: components["schemas"]["MeetingBookingStatus"];
                /** @example America/Sao_Paulo */
                timezone: string;
            };
        };
        Data_MeetingTypeResponse: {
            data: {
                /** @example /book/intro-call-ana */
                booking_path: string;
                created_at: string;
                description?: string | null;
                distribution_rule?: null | components["schemas"]["BookingReference"];
                /** Format: int32 */
                duration_minutes: number;
                id: string;
                is_active: boolean;
                /** @example personal */
                kind: string;
                location_kind: components["schemas"]["MeetingLocationKind"];
                location_value?: string | null;
                name: string;
                owner: components["schemas"]["BookingReference"];
                questions: components["schemas"]["MeetingQuestionResponse"][];
                /** Format: int32 */
                slot_interval_minutes: number;
                slug: string;
                spam_protection: boolean;
                updated_at: string;
            };
        };
        MeetingTypeRequest: {
            description?: string | null;
            distribution_rule_id?: string | null;
            /**
             * Format: int32
             * @example 30
             */
            duration_minutes?: number | null;
            is_active?: boolean | null;
            location_kind?: null | components["schemas"]["MeetingLocationKind"];
            /** @example https://meet.example.com/ana */
            location_value?: string | null;
            /** @example Intro call */
            name?: string | null;
            questions?: components["schemas"]["MeetingQuestionRequest"][] | null;
            /**
             * Format: int32
             * @example 30
             */
            slot_interval_minutes?: number | null;
            /** @example intro-call-ana */
            slug?: string | null;
            spam_protection?: boolean | null;
        };
        Page_MeetingTypeResponse: {
            data: {
                /** @example /book/intro-call-ana */
                booking_path: string;
                created_at: string;
                description?: string | null;
                distribution_rule?: null | components["schemas"]["BookingReference"];
                /** Format: int32 */
                duration_minutes: number;
                id: string;
                is_active: boolean;
                /** @example personal */
                kind: string;
                location_kind: components["schemas"]["MeetingLocationKind"];
                location_value?: string | null;
                name: string;
                owner: components["schemas"]["BookingReference"];
                questions: components["schemas"]["MeetingQuestionResponse"][];
                /** Format: int32 */
                slot_interval_minutes: number;
                slug: string;
                spam_protection: boolean;
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
        Data_Vec_BookingReference: {
            data: {
                id: string;
                name: string;
            }[];
        };
        Data_Vec_BookingContactFieldResponse: {
            data: {
                is_custom: boolean;
                /** @example job_title */
                key: string;
                /** @example Job title */
                label: string;
            }[];
        };
        Data_MeetingBookingResponse: {
            data: {
                activity_id?: string | null;
                answers: Record<string, never>;
                cancel_reason?: string | null;
                canceled_at?: string | null;
                contact_id?: string | null;
                created_at: string;
                ends_at: string;
                guest_email: string;
                guest_name: string;
                guest_timezone?: string | null;
                host: components["schemas"]["BookingReference"];
                id: string;
                meeting_type: components["schemas"]["BookingReference"];
                starts_at: string;
                status: components["schemas"]["MeetingBookingStatus"];
            };
        };
        MeetingBookingCancelRequest: {
            /** @example Booking conflict */
            reason?: string | null;
        };
        Page_MeetingBookingResponse: {
            data: {
                activity_id?: string | null;
                answers: Record<string, never>;
                cancel_reason?: string | null;
                canceled_at?: string | null;
                contact_id?: string | null;
                created_at: string;
                ends_at: string;
                guest_email: string;
                guest_name: string;
                guest_timezone?: string | null;
                host: components["schemas"]["BookingReference"];
                id: string;
                meeting_type: components["schemas"]["BookingReference"];
                starts_at: string;
                status: components["schemas"]["MeetingBookingStatus"];
            }[];
            meta: components["schemas"]["PageMeta"];
        };
        BookingAvailabilityRequest: {
            /**
             * Format: int32
             * @example 15
             */
            buffer_after_minutes?: number | null;
            /**
             * Format: int32
             * @example 0
             */
            buffer_before_minutes?: number | null;
            /**
             * Format: int32
             * @example 60
             */
            horizon_days?: number | null;
            /**
             * Format: int32
             * @example 6
             */
            max_per_day?: number | null;
            /**
             * Format: int32
             * @example 240
             */
            min_notice_minutes?: number | null;
            /** @example America/Sao_Paulo */
            timezone?: string | null;
            weekly_hours?: components["schemas"]["BookingWeeklyInterval"][] | null;
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
        Data_BookingAvailabilityResponse: {
            data: {
                /** Format: int32 */
                buffer_after_minutes: number;
                /** Format: int32 */
                buffer_before_minutes: number;
                configured: boolean;
                /** Format: int32 */
                horizon_days: number;
                /** Format: int32 */
                max_per_day?: number | null;
                /** Format: int32 */
                min_notice_minutes: number;
                /** @example America/Sao_Paulo */
                timezone: string;
                updated_at?: string | null;
                user_id: string;
                weekly_hours: components["schemas"]["BookingWeeklyInterval"][];
            };
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
    booking_availability_get: {
        parameters: {
            query?: {
                user_id?: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Weekly hours, time zone, minimum notice, daily limit, buffers and booking horizon used by the booking links of the user (yours, or `user_id` when your `booking.manage` scope covers that member). Defaults (Monday to Friday 09:00–18:00, 4 h notice, 60 days) with `configured: false` until saved */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_BookingAvailabilityResponse"];
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
            /** @description Missing `booking.manage` or user outside your scope (`permission_denied`, `insufficient_scope`) */
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
    booking_availability_save: {
        parameters: {
            query?: {
                user_id?: string;
            };
            header?: {
                /** @description `updated_at` of the availability as read; fails with 409 when it changed since */
                "If-Match"?: string | null;
            };
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["BookingAvailabilityRequest"];
            };
        };
        responses: {
            /** @description Availability saved (`booking_availability.updated`); omitted properties stay unchanged and `weekly_hours` replaces the whole week (`weekday` 1 = Monday … 7 = Sunday, `HH:MM`, intervals of a day cannot overlap) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_BookingAvailabilityResponse"];
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
            /** @description Missing `booking.manage` or user outside your scope (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Changed since the `If-Match` version (`version_conflict`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid time zone, hours or limits (`validation_error`) */
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
    booking_bookings_list: {
        parameters: {
            query?: {
                status?: components["schemas"]["MeetingBookingStatus"];
                meeting_type_id?: string;
                upcoming?: boolean;
                page?: number;
                per_page?: number;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Meetings booked through booking links whose host is within your `booking.manage` scope; `upcoming=true` lists the ones not finished yet in chronological order, otherwise newest first */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page_MeetingBookingResponse"];
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
            /** @description Missing `booking.manage` (`permission_denied`, `insufficient_scope`) */
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
    booking_bookings_cancel: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Booking id */
                booking_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["MeetingBookingCancelRequest"];
            };
        };
        responses: {
            /** @description Booking canceled by the host (`meeting.canceled`): the meeting activity is canceled and the guest receives the cancellation with an `.ics` (`METHOD:CANCEL`) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_MeetingBookingResponse"];
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
            /** @description Missing `booking.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Not found or outside your scope (`booking_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Already canceled or in the past (`booking_not_changeable`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
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
    booking_contact_fields: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Contact fields that can receive the answer of a booking question (phone, mobile, job title, notes, city, state, country and active text-like custom fields). Answers fill the contact only when the booking creates it */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_BookingContactFieldResponse"];
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
            /** @description Missing `booking.manage` (`permission_denied`, `insufficient_scope`) */
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
    booking_distribution_rules: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Active lead distribution rules that a team booking link can use to pick the host */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_Vec_BookingReference"];
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
            /** @description Team links need `booking.manage` for the team or the organization (`permission_denied`, `insufficient_scope`) */
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
    booking_meeting_types_list: {
        parameters: {
            query?: {
                page?: number;
                per_page?: number;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Meeting types (booking links) whose owner is within your `booking.manage` scope, ordered by name */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page_MeetingTypeResponse"];
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
            /** @description Missing `booking.manage` (`permission_denied`, `insufficient_scope`) */
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
    booking_meeting_types_create: {
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
                "application/json": components["schemas"]["MeetingTypeRequest"];
            };
        };
        responses: {
            /** @description Meeting type created (`meeting_type.created`) and owned by you; its public page is `/book/{slug}` (`slug` derived from the name when omitted). `location_kind` is `none`, `video`, `address` or `phone` (value shown to the guest only after booking). `questions` (up to 10) may map the answer to a contact field (see `/booking/contact-fields`). `distribution_rule_id` turns it into a team link: the host is picked among the free members of the rule by its rotation. `spam_protection` enables the honeypot */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_MeetingTypeResponse"];
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
            /** @description Missing `booking.manage`, or a team link without team/organization scope (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Another link already uses the slug (`meeting_type_slug_taken`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid properties, questions or rule (`validation_error`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
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
    booking_meeting_types_get: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Meeting type id */
                meeting_type_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Meeting type */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_MeetingTypeResponse"];
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
            /** @description Missing `booking.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Not found or outside your scope (`meeting_type_not_found`) */
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
    booking_meeting_types_delete: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Meeting type id */
                meeting_type_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Meeting type and its booking history deleted (`meeting_type.deleted`); the meeting tasks stay in the CRM */
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
            /** @description Missing `booking.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Not found or outside your scope (`meeting_type_not_found`) */
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
    booking_meeting_types_update: {
        parameters: {
            query?: never;
            header?: {
                /** @description `updated_at` of the meeting type as read; fails with 409 when it changed since */
                "If-Match"?: string | null;
            };
            path: {
                /** @description Meeting type id */
                meeting_type_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["MeetingTypeRequest"];
            };
        };
        responses: {
            /** @description Meeting type updated (`meeting_type.updated`); omitted properties stay unchanged, `questions` replaces the list and `distribution_rule_id: null` turns a team link into a personal one. Existing bookings keep their time */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_MeetingTypeResponse"];
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
            /** @description Missing `booking.manage` (`permission_denied`, `insufficient_scope`) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Not found or outside your scope (`meeting_type_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Slug taken (`meeting_type_slug_taken`) or changed since the `If-Match` version (`version_conflict`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid properties (`validation_error`) */
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
    booking_public_booking: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Signed management token */
                token: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Booking */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PublicBookingResponse"];
                };
            };
            /** @description Invalid token (`booking_not_found`) */
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
    booking_public_cancel: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Signed management token */
                token: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["PublicCancelRequest"];
            };
        };
        responses: {
            /** @description Canceled */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PublicBookingResponse"];
                };
            };
            /** @description Invalid token (`booking_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Already canceled or past (`booking_not_changeable`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
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
            /** @description Too many changes from this address (`rate_limited`) */
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
    booking_public_reschedule: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Signed management token */
                token: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["PublicRescheduleRequest"];
            };
        };
        responses: {
            /** @description Rescheduled */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PublicBookingResponse"];
                };
            };
            /** @description Invalid token (`booking_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Slot no longer free (`booking_slot_unavailable`) or booking canceled/past (`booking_not_changeable`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid time (`validation_error`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Too many changes from this address (`rate_limited`) */
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
    booking_public_booking_slots: {
        parameters: {
            query: {
                /** @example 2026-10-26T00:00:00-03:00 */
                from: string;
                /** @example 2026-11-02T00:00:00-03:00 */
                to: string;
            };
            header?: never;
            path: {
                /** @description Signed management token */
                token: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Free slots */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PublicSlotsResponse"];
                };
            };
            /** @description Invalid token (`booking_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Canceled or past booking (`booking_not_changeable`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid window (`validation_error`) */
            422: {
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
    booking_public_meeting_type: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Slug of the booking link */
                slug: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Booking link */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PublicMeetingTypeResponse"];
                };
            };
            /** @description Unknown or inactive link (`meeting_type_not_found`) */
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
    booking_public_book: {
        parameters: {
            query?: never;
            header?: {
                /** @description Up to 255 printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored */
                "Idempotency-Key"?: string;
            };
            path: {
                /** @description Slug of the booking link */
                slug: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["PublicBookingRequest"];
            };
        };
        responses: {
            /** @description Booked */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PublicBookingResponse"];
                };
            };
            /** @description Unknown or inactive link (`meeting_type_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description The slot is no longer free (`booking_slot_unavailable`). The first request with this `Idempotency-Key` is still being processed (`idempotency_key_in_progress`) */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid data (`validation_error`) or rejected as spam (`booking_rejected`). `Idempotency-Key` already used with a different request (`idempotency_key_reused`) or longer than 255 characters (`validation_error`) */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Too many bookings from this address or for this link (`rate_limited`) */
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
    booking_public_slots: {
        parameters: {
            query: {
                /** @example 2026-10-26T00:00:00-03:00 */
                from: string;
                /** @example 2026-11-02T00:00:00-03:00 */
                to: string;
            };
            header?: never;
            path: {
                /** @description Slug of the booking link */
                slug: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Free slots */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Data_PublicSlotsResponse"];
                };
            };
            /** @description Unknown or inactive link (`meeting_type_not_found`) */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            /** @description Invalid window (`validation_error`) */
            422: {
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
}

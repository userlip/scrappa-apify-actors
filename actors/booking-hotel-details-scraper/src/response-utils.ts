import type { BookingHotelRequest } from './request-params.js';

export interface BookingHotelResponse {
    success?: boolean;
    data?: BookingHotelDetails;
    meta?: Record<string, unknown>;
    [key: string]: unknown;
}

export interface BookingHotelDetails {
    title?: string | null;
    canonical_url?: string | null;
    hotel_schema?: Record<string, unknown> | null;
    aggregate_rating?: Record<string, unknown> | null;
    json_ld?: unknown[] | Record<string, unknown> | null;
    parsed?: boolean;
    [key: string]: unknown;
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function firstString(...values: unknown[]): string | null {
    for (const value of values) {
        if (typeof value === 'string' && value.trim() !== '') {
            return value;
        }
    }

    return null;
}

function getNestedString(source: Record<string, unknown> | null | undefined, key: string): string | null {
    if (!source) {
        return null;
    }

    return firstString(source[key]);
}

export function getBookingHotelDetails(response: BookingHotelResponse): BookingHotelDetails {
    if (isRecord(response.data)) {
        return response.data as BookingHotelDetails;
    }

    return response as BookingHotelDetails;
}

export function buildBookingHotelDatasetItem(
    details: BookingHotelDetails,
    request: BookingHotelRequest,
): Record<string, unknown> {
    const hotelSchema = isRecord(details.hotel_schema) ? details.hotel_schema : null;
    const aggregateRating = isRecord(details.aggregate_rating)
        ? details.aggregate_rating
        : (isRecord(hotelSchema?.aggregateRating) ? hotelSchema.aggregateRating : null);

    return {
        ...details,
        title: firstString(details.title, getNestedString(hotelSchema, 'name')),
        canonical_url: firstString(details.canonical_url, details.url),
        hotel_schema: details.hotel_schema ?? null,
        aggregate_rating: aggregateRating,
        json_ld: details.json_ld ?? null,
        parsed: Boolean(details.parsed),
        request_index: request.index,
        request_input_type: request.inputType,
        request_url: request.params.url ?? null,
        request_country: request.params.country ?? null,
        request_slug: request.params.slug ?? null,
        request_success: true,
    };
}

export function buildBookingHotelErrorItem(
    request: BookingHotelRequest,
    error: unknown,
): Record<string, unknown> {
    return {
        request_index: request.index,
        request_input_type: request.inputType,
        request_url: request.params.url ?? null,
        request_country: request.params.country ?? null,
        request_slug: request.params.slug ?? null,
        request_success: false,
        error_message: error instanceof Error ? error.message : String(error),
    };
}

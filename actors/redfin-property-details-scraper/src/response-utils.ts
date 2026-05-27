import type { RedfinPropertyDetailsRequest } from './request-params.js';

export interface RedfinPropertyDetails {
    property_id?: number | string | null;
    address?: string | null;
    city?: string | null;
    state?: string | null;
    zip?: string | number | null;
    country?: string | null;
    price?: number | string | null;
    price_label?: string | null;
    beds?: number | string | null;
    baths?: number | string | null;
    sqft?: number | string | null;
    lot_size?: number | string | null;
    year_built?: number | string | null;
    property_type?: number | string | null;
    status?: number | string | null;
    status_label?: string | null;
    latitude?: number | string | null;
    longitude?: number | string | null;
    url?: string | null;
    photos?: unknown[];
    description?: string | null;
    [key: string]: unknown;
}

export interface RedfinPropertyDetailsResponse {
    data?: RedfinPropertyDetails | RedfinPropertyDetails[] | null;
    [key: string]: unknown;
}

function firstString(...values: unknown[]): string | null {
    for (const value of values) {
        if (typeof value === 'string' && value.trim() !== '') {
            return value;
        }
        if (typeof value === 'number' && Number.isFinite(value)) {
            return String(value);
        }
    }

    return null;
}

function firstNumber(...values: unknown[]): number | null {
    for (const value of values) {
        if (typeof value === 'number' && Number.isFinite(value)) {
            return value;
        }
        if (typeof value === 'string' && value.trim() !== '') {
            const number = Number(value);
            if (Number.isFinite(number)) {
                return number;
            }
        }
    }

    return null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === 'object' && value !== null && !Array.isArray(value);
}

export function getRedfinPropertyDetails(response: RedfinPropertyDetailsResponse): RedfinPropertyDetails | null {
    if (Array.isArray(response.data)) {
        return isRecord(response.data[0]) ? response.data[0] : null;
    }

    if (isRecord(response.data)) {
        return response.data;
    }

    if (isRecord(response) && isRecord(response.property)) {
        return response.property as RedfinPropertyDetails;
    }

    return null;
}

export function buildRedfinPropertyDetailsDatasetItem(
    property: RedfinPropertyDetails,
    request: RedfinPropertyDetailsRequest,
): Record<string, unknown> {
    return {
        ...property,
        property_id: firstNumber(property.property_id, request.params.property_id),
        address: firstString(property.address),
        city: firstString(property.city),
        state: firstString(property.state),
        zip: firstString(property.zip),
        country: firstString(property.country),
        price: firstNumber(property.price),
        price_label: firstString(property.price_label),
        beds: firstNumber(property.beds),
        baths: firstNumber(property.baths),
        sqft: firstNumber(property.sqft),
        lot_size: firstNumber(property.lot_size),
        year_built: firstNumber(property.year_built),
        property_type: firstNumber(property.property_type),
        status: firstNumber(property.status),
        status_label: firstString(property.status_label),
        latitude: firstNumber(property.latitude),
        longitude: firstNumber(property.longitude),
        url: firstString(property.url),
        description: firstString(property.description),
        photos: Array.isArray(property.photos) ? property.photos : [],
        request_property_index: request.index,
        request_property_id: request.params.property_id,
        request_input: request.input,
        request_source: request.source,
    };
}

export function buildRedfinPropertyErrorDatasetItem(
    request: RedfinPropertyDetailsRequest,
    message: string,
    statusCode: number | null,
): Record<string, unknown> {
    return {
        success: false,
        property_id: request.params.property_id,
        request_property_index: request.index,
        request_property_id: request.params.property_id,
        request_input: request.input,
        request_source: request.source,
        error: message,
        status_code: statusCode,
    };
}

export interface RedfinValuationItemInput {
    property_id?: unknown;
    listing_id?: unknown;
    url?: unknown;
}

export interface RedfinValuationInput extends RedfinValuationItemInput {
    property_ids?: unknown;
    properties?: unknown;
}

export interface RedfinValuationRequest {
    params: Record<string, unknown>;
    property_id: number;
    listing_id: number | null;
    url: string | null;
    index: number;
}

const MAX_VALUATIONS_PER_RUN = 50;

function cleanString(value: unknown, field: string, maxLength: number): string | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    if (typeof value !== 'string') {
        throw new Error(`${field} must be a string`);
    }

    const trimmed = value.trim();
    if (trimmed === '') {
        return undefined;
    }

    if (trimmed.length > maxLength) {
        throw new Error(`${field} must be ${maxLength} characters or fewer`);
    }

    return trimmed;
}

function cleanInteger(value: unknown, field: string): number | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    const normalized = typeof value === 'string' && /^\d+$/.test(value.trim())
        ? Number(value.trim())
        : value;

    if (typeof normalized !== 'number' || !Number.isInteger(normalized)) {
        throw new Error(`${field} must be an integer`);
    }

    if (!Number.isSafeInteger(normalized)) {
        throw new Error(`${field} must be a safe integer`);
    }

    if (normalized <= 0) {
        throw new Error(`${field} must be greater than 0`);
    }

    return normalized;
}

function cleanRequiredInteger(value: unknown, field: string): number {
    const cleaned = cleanInteger(value, field);
    if (cleaned === undefined) {
        throw new Error(`${field} is required`);
    }

    return cleaned;
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === 'object' && value !== null && !Array.isArray(value);
}

export function extractRedfinIdsFromUrl(value: unknown): { property_id?: number; listing_id?: number; url?: string } {
    const url = cleanString(value, 'url', 2000);
    if (url === undefined) {
        return {};
    }

    let propertyId: number | undefined;
    let listingId: number | undefined;

    let parsedUrl: URL;
    try {
        parsedUrl = new URL(url);
    } catch {
        return { url };
    }

    const hostname = parsedUrl.hostname.toLowerCase();
    if (hostname !== 'redfin.com' && !hostname.endsWith('.redfin.com')) {
        return { url };
    }

    const normalizedUrl = parsedUrl.toString();
    const homeMatch = normalizedUrl.match(/\/home\/(\d+)(?:[/?#]|$)/i);
    if (homeMatch) {
        propertyId = Number(homeMatch[1]);
    }

    const listingMatch = normalizedUrl.match(/[?&](?:listing_id|listingId|listingIdOverride|listing)=([0-9]+)/i)
        ?? normalizedUrl.match(/\/listing\/(\d+)(?:[/?#]|$)/i);
    if (listingMatch) {
        listingId = Number(listingMatch[1]);
    }

    return {
        property_id: propertyId,
        listing_id: listingId,
        url,
    };
}

function buildSingleRedfinValuationRequest(
    input: RedfinValuationItemInput,
    index: number,
    prefix = '',
): RedfinValuationRequest {
    const extracted = extractRedfinIdsFromUrl(input.url);
    const propertyId = cleanRequiredInteger(input.property_id ?? extracted.property_id, `${prefix}property_id`);
    const listingId = cleanInteger(input.listing_id ?? extracted.listing_id, `${prefix}listing_id`) ?? null;
    const url = extracted.url ?? null;

    const params: Record<string, unknown> = {
        property_id: propertyId,
    };
    if (listingId !== null) {
        params.listing_id = listingId;
    }

    return {
        params,
        property_id: propertyId,
        listing_id: listingId,
        url,
        index,
    };
}

function buildPropertyIdRequests(propertyIds: unknown[]): RedfinValuationRequest[] {
    return propertyIds.map((propertyId, index) => buildSingleRedfinValuationRequest(
        { property_id: propertyId },
        index,
        `property_ids[${index}].`,
    ));
}

function assertBatchSize(length: number, field: string): void {
    if (length === 0) {
        throw new Error(`${field} must include at least one property`);
    }
    if (length > MAX_VALUATIONS_PER_RUN) {
        throw new Error(`${field} cannot include more than ${MAX_VALUATIONS_PER_RUN} properties per run`);
    }
}

export function buildRedfinValuationRequests(input: RedfinValuationInput): RedfinValuationRequest[] {
    if (Array.isArray(input.properties)) {
        assertBatchSize(input.properties.length, 'properties');

        return input.properties.map((property, index) => {
            if (!isRecord(property)) {
                throw new Error(`properties[${index}] must be an object`);
            }

            return buildSingleRedfinValuationRequest(property, index, `properties[${index}].`);
        });
    }

    if (input.properties !== undefined) {
        throw new Error('properties must be an array of property objects');
    }

    if (Array.isArray(input.property_ids)) {
        assertBatchSize(input.property_ids.length, 'property_ids');
        return buildPropertyIdRequests(input.property_ids);
    }

    if (input.property_ids !== undefined) {
        throw new Error('property_ids must be an array of property IDs');
    }

    return [buildSingleRedfinValuationRequest(input, 0)];
}

export function describeRedfinValuationRequest(request: RedfinValuationRequest): string {
    return request.listing_id === null
        ? `property ${request.property_id}`
        : `property ${request.property_id} with listing ${request.listing_id}`;
}

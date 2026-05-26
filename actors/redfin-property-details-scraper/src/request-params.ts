export interface RedfinPropertyDetailsInput {
    property_id?: unknown;
    property_ids?: unknown;
    url?: unknown;
    urls?: unknown;
}

export interface RedfinPropertyDetailsRequest {
    params: {
        property_id: number;
    };
    index: number;
    input: string | number;
    source: 'property_id' | 'property_ids' | 'url' | 'urls';
}

const MAX_PROPERTIES_PER_RUN = 100;

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

function cleanPropertyId(value: unknown, field: string): number {
    const normalized = typeof value === 'string' && /^\d+$/.test(value.trim())
        ? Number(value.trim())
        : value;

    if (typeof normalized !== 'number' || !Number.isInteger(normalized)) {
        throw new Error(`${field} must be an integer`);
    }

    if (normalized <= 0) {
        throw new Error(`${field} must be greater than 0`);
    }

    if (!Number.isSafeInteger(normalized)) {
        throw new Error(`${field} is too large`);
    }

    return normalized;
}

export function extractRedfinPropertyIdFromUrl(value: unknown, field = 'url'): number {
    const url = cleanString(value, field, 2048);
    if (url === undefined) {
        throw new Error(`${field} is required`);
    }

    let parsed: URL;
    try {
        parsed = new URL(url);
    } catch {
        throw new Error(`${field} must be a valid Redfin URL containing /home/{property_id}`);
    }

    const hostname = parsed.hostname.toLowerCase();
    if (hostname !== 'redfin.com' && !hostname.endsWith('.redfin.com')) {
        throw new Error(`${field} must be a Redfin URL`);
    }

    const match = parsed.pathname.match(/\/home\/(\d+)(?:\/)?$/);
    if (!match) {
        throw new Error(`${field} must contain a /home/{property_id} path`);
    }

    return cleanPropertyId(match[1], field);
}

function addSingleRequest(
    requests: RedfinPropertyDetailsRequest[],
    seenPropertyIds: Set<number>,
    propertyId: number,
    input: string | number,
    source: RedfinPropertyDetailsRequest['source'],
): void {
    if (seenPropertyIds.has(propertyId)) {
        return;
    }

    seenPropertyIds.add(propertyId);
    requests.push({
        params: { property_id: propertyId },
        index: requests.length,
        input,
        source,
    });
}

function addPropertyIdInput(
    requests: RedfinPropertyDetailsRequest[],
    seenPropertyIds: Set<number>,
    value: unknown,
    source: 'property_id' | 'property_ids',
    field: string,
): void {
    const propertyId = cleanPropertyId(value, field);
    addSingleRequest(requests, seenPropertyIds, propertyId, typeof value === 'string' ? value.trim() : propertyId, source);
}

function addUrlInput(
    requests: RedfinPropertyDetailsRequest[],
    seenPropertyIds: Set<number>,
    value: unknown,
    source: 'url' | 'urls',
    field: string,
): void {
    const url = cleanString(value, field, 2048);
    if (url === undefined) {
        throw new Error(`${field} is required`);
    }

    const propertyId = extractRedfinPropertyIdFromUrl(url, field);
    addSingleRequest(requests, seenPropertyIds, propertyId, url, source);
}

export function buildRedfinPropertyDetailsRequests(input: RedfinPropertyDetailsInput): RedfinPropertyDetailsRequest[] {
    const requests: RedfinPropertyDetailsRequest[] = [];
    const seenPropertyIds = new Set<number>();

    if (input.property_id !== undefined && input.property_id !== null && input.property_id !== '') {
        addPropertyIdInput(requests, seenPropertyIds, input.property_id, 'property_id', 'property_id');
    }

    if (input.url !== undefined && input.url !== null && input.url !== '') {
        addUrlInput(requests, seenPropertyIds, input.url, 'url', 'url');
    }

    if (input.property_ids !== undefined) {
        if (!Array.isArray(input.property_ids)) {
            throw new Error('property_ids must be an array');
        }

        input.property_ids.forEach((propertyId, index) => {
            addPropertyIdInput(requests, seenPropertyIds, propertyId, 'property_ids', `property_ids[${index}]`);
        });
    }

    if (input.urls !== undefined) {
        if (!Array.isArray(input.urls)) {
            throw new Error('urls must be an array');
        }

        input.urls.forEach((url, index) => {
            addUrlInput(requests, seenPropertyIds, url, 'urls', `urls[${index}]`);
        });
    }

    if (requests.length === 0) {
        throw new Error('Provide at least one property_id, property_ids item, url, or urls item');
    }

    if (requests.length > MAX_PROPERTIES_PER_RUN) {
        throw new Error(`Input cannot include more than ${MAX_PROPERTIES_PER_RUN} unique properties per run`);
    }

    return requests;
}

export function describeRedfinPropertyDetailsRequest(request: RedfinPropertyDetailsRequest): string {
    return `property_id ${request.params.property_id} from ${request.source}`;
}

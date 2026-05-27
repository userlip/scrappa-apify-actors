export interface GooglePatentsDetailsInput {
    patent_id?: unknown;
    patent_ids?: unknown;
    url?: unknown;
    urls?: unknown;
    debug?: unknown;
}

export interface GooglePatentsDetailsRequest {
    inputPatentId: string;
    normalizedPatentId: string;
    params: Record<string, unknown>;
}

const FULL_PATENT_ID_PATTERN = /^patent\/([A-Z]{2}\d+[A-Z]?\d*)\/([a-z]{2})$/i;
const SHORT_PATENT_ID_PATTERN = /^[A-Z]{2}\d+[A-Z]?\d*$/i;
const GOOGLE_PATENTS_HOST_PATTERN = /(^|\.)patents\.google\.com$/i;

function cleanString(value: unknown, field: string): string | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    if (typeof value !== 'string') {
        throw new Error(`${field} must be a string`);
    }

    const trimmed = value.trim();
    return trimmed === '' ? undefined : trimmed;
}

function cleanStringArray(value: unknown, field: string): string[] {
    if (value === undefined || value === null || value === '') {
        return [];
    }

    if (!Array.isArray(value)) {
        throw new Error(`${field} must be an array of strings`);
    }

    return value
        .map((item, index) => cleanString(item, `${field}[${index}]`))
        .filter((item): item is string => item !== undefined);
}

function normalizeFullPatentId(rawPatentId: string): string {
    const fullMatch = FULL_PATENT_ID_PATTERN.exec(rawPatentId);
    if (fullMatch) {
        return `patent/${fullMatch[1].toUpperCase()}/${fullMatch[2].toLowerCase()}`;
    }

    if (!SHORT_PATENT_ID_PATTERN.test(rawPatentId)) {
        throw new Error(`Invalid patent ID format: ${rawPatentId}. Expected US9789384B1, EP3892147A1, WO2020123456A1, or a Google Patents URL.`);
    }

    return `patent/${rawPatentId.toUpperCase()}/en`;
}

export function normalizeGooglePatentIdentifier(rawValue: string): string {
    const trimmed = rawValue.trim();
    if (!trimmed) {
        throw new Error('Patent ID or URL cannot be empty');
    }

    if (/^https?:\/\//i.test(trimmed)) {
        const parsed = new URL(trimmed);
        if (!GOOGLE_PATENTS_HOST_PATTERN.test(parsed.hostname)) {
            throw new Error(`Invalid Google Patents URL: ${rawValue}`);
        }

        const match = /^\/patent\/([^/?#]+)(?:\/([a-z]{2}))?\/?$/i.exec(parsed.pathname);
        if (!match) {
            throw new Error(`Invalid Google Patents URL path: ${rawValue}`);
        }

        const [, publicationNumber, language = 'en'] = match;
        return normalizeFullPatentId(`patent/${publicationNumber}/${language}`);
    }

    return normalizeFullPatentId(trimmed);
}

export function collectGooglePatentsDetailsRequests(input: GooglePatentsDetailsInput): GooglePatentsDetailsRequest[] {
    const rawValues = [
        cleanString(input.patent_id, 'patent_id'),
        ...cleanStringArray(input.patent_ids, 'patent_ids'),
        cleanString(input.url, 'url'),
        ...cleanStringArray(input.urls, 'urls'),
    ].filter((value): value is string => value !== undefined);

    if (rawValues.length === 0) {
        throw new Error('At least one patent ID or Google Patents URL is required. Provide patent_id, patent_ids, url, or urls.');
    }

    const requests = new Map<string, GooglePatentsDetailsRequest>();
    for (const rawValue of rawValues) {
        const normalizedPatentId = normalizeGooglePatentIdentifier(rawValue);
        if (!requests.has(normalizedPatentId)) {
            requests.set(normalizedPatentId, {
                inputPatentId: rawValue,
                normalizedPatentId,
                params: {
                    patent_id: normalizedPatentId,
                },
            });
        }
    }

    return [...requests.values()];
}

export function describeGooglePatentsDetailsRequest(requests: GooglePatentsDetailsRequest[]): string {
    if (requests.length === 1) {
        return requests[0].normalizedPatentId;
    }

    return `${requests.length} patents`;
}

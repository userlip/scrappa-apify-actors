export interface TikTokSearchInput {
    keywords?: unknown;
    query?: unknown;
    region?: unknown;
    count?: unknown;
    cursor?: unknown;
    publish_time?: unknown;
    sort_type?: unknown;
}

export function formatTikTokSearchLookupForLog(input: TikTokSearchInput): string {
    const keywords = typeof input.keywords === 'string' ? input.keywords.trim() : '';
    const query = typeof input.query === 'string' ? input.query.trim() : '';

    return keywords || query || 'unknown TikTok search';
}

export function normalizeTikTokSearchKeywords(value: string): string {
    const keywords = value.trim().replace(/\s+/g, ' ');

    if (keywords === '') {
        return '';
    }

    if (keywords.length > 255) {
        throw new Error('TikTok search keywords must be 255 characters or fewer');
    }

    if (/[\r\n\t\f\v]/.test(value)) {
        throw new Error('TikTok search keywords cannot contain tabs, line breaks, or control whitespace');
    }

    return keywords;
}

function normalizeRegion(value: string): string {
    const region = value.trim().toUpperCase();

    if (region === '') {
        return '';
    }

    if (!/^[A-Z]{2,10}$/.test(region)) {
        throw new Error('region must be a 2 to 10 character country or region code');
    }

    return region;
}

function normalizeInteger(
    value: unknown,
    field: string,
    min: number,
    max: number,
    warn: (message: string) => void,
): number | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    if (typeof value !== 'number' || !Number.isInteger(value) || value < min || value > max) {
        warn(`${field} must be an integer between ${min} and ${max}, got ${String(value)}. Omitting ${field}.`);
        return undefined;
    }

    return value;
}

export function buildTikTokSearchParams(
    input: TikTokSearchInput,
    warn: (message: string) => void = console.warn,
): Record<string, unknown> {
    const params: Record<string, unknown> = {};

    if (typeof input.keywords === 'string') {
        const keywords = normalizeTikTokSearchKeywords(input.keywords);
        if (keywords !== '') {
            params.keywords = keywords;
        }
    } else if (input.keywords !== undefined && input.keywords !== null) {
        warn(`keywords must be a string, got ${typeof input.keywords}.`);
    }

    if (!params.keywords && typeof input.query === 'string') {
        const query = normalizeTikTokSearchKeywords(input.query);
        if (query !== '') {
            params.keywords = query;
        }
    } else if (!params.keywords && input.query !== undefined && input.query !== null) {
        warn(`query must be a string, got ${typeof input.query}.`);
    }

    if (!params.keywords) {
        throw new Error('TikTok search keywords are required');
    }

    if (typeof input.region === 'string') {
        const region = normalizeRegion(input.region);
        if (region !== '') {
            params.region = region;
        }
    } else if (input.region !== undefined && input.region !== null) {
        warn(`region must be a string, got ${typeof input.region}. Omitting region.`);
    }

    const count = normalizeInteger(input.count, 'count', 1, 50, warn);
    if (count !== undefined) {
        params.count = count;
    }

    if (typeof input.cursor === 'string') {
        const cursor = input.cursor.trim();
        if (cursor !== '') {
            params.cursor = cursor;
        }
    } else if (typeof input.cursor === 'number') {
        if (Number.isSafeInteger(input.cursor)) {
            params.cursor = String(input.cursor);
        } else {
            warn(`cursor must be a string or safe integer, got ${String(input.cursor)}. Starting from the first page.`);
        }
    } else if (input.cursor !== undefined && input.cursor !== null) {
        warn(`cursor must be a string or number, got ${typeof input.cursor}. Starting from the first page.`);
    }

    const publishTime = normalizeInteger(input.publish_time, 'publish_time', 0, 3650, warn);
    if (publishTime !== undefined) {
        params.publish_time = publishTime;
    }

    const sortType = normalizeInteger(input.sort_type, 'sort_type', 0, 10, warn);
    if (sortType !== undefined) {
        params.sort_type = sortType;
    }

    return params;
}

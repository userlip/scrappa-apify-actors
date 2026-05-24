export interface GoogleTrendsRelatedQueriesInput {
    query?: unknown;
    q?: unknown;
    geo?: unknown;
    time_range?: unknown;
    hl?: unknown;
    search_type?: unknown;
    include_autocomplete?: unknown;
}

const TIME_RANGE_VALUES = ['1h', '4h', '1d', '7d', '30d', '90d', '1y', '5y', 'all'] as const;
const SEARCH_TYPE_VALUES = ['web', 'images', 'news', 'youtube', 'shopping'] as const;

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

function cleanRequiredString(value: unknown, field: string, maxLength: number): string {
    const cleaned = cleanString(value, field, maxLength);
    if (cleaned === undefined) {
        throw new Error(`${field} is required`);
    }

    return cleaned;
}

function cleanGeo(value: unknown): string | undefined {
    const geo = cleanString(value, 'geo', 10);
    if (geo === undefined) {
        return undefined;
    }

    if (geo.toLowerCase() === 'worldwide') {
        return 'Worldwide';
    }

    return geo.toUpperCase();
}

function cleanLanguage(value: unknown): string | undefined {
    const hl = cleanString(value, 'hl', 2);
    if (hl === undefined) {
        return undefined;
    }

    if (!/^[a-z]{2}$/i.test(hl)) {
        throw new Error('hl must be a two-letter language code');
    }

    return hl.toLowerCase();
}

function cleanEnum<T extends readonly string[]>(
    value: unknown,
    field: string,
    values: T,
): T[number] | undefined {
    const cleaned = cleanString(value, field, 20);
    if (cleaned === undefined) {
        return undefined;
    }

    const normalized = cleaned.toLowerCase();
    if (!values.includes(normalized)) {
        throw new Error(`${field} must be one of: ${values.join(', ')}`);
    }

    return normalized as T[number];
}

function cleanBoolean(value: unknown, field: string): boolean {
    if (value === undefined || value === null || value === '') {
        return false;
    }

    if (typeof value === 'boolean') {
        return value;
    }

    throw new Error(`${field} must be a boolean`);
}

export function buildGoogleTrendsRelatedQueriesParams(input: GoogleTrendsRelatedQueriesInput): Record<string, unknown> {
    const query = input.query ?? input.q;
    const params: Record<string, unknown> = {
        q: cleanRequiredString(query, 'query', 100),
    };

    const geo = cleanGeo(input.geo);
    const timeRange = cleanEnum(input.time_range, 'time_range', TIME_RANGE_VALUES);
    const hl = cleanLanguage(input.hl);
    const searchType = cleanEnum(input.search_type, 'search_type', SEARCH_TYPE_VALUES);

    if (geo !== undefined) params.geo = geo;
    if (timeRange !== undefined) params.time_range = timeRange;
    if (hl !== undefined) params.hl = hl;
    if (searchType !== undefined) params.search_type = searchType;

    return params;
}

export function buildGoogleTrendsAutocompleteParams(params: Record<string, unknown>): Record<string, unknown> {
    const autocompleteParams: Record<string, unknown> = {
        q: params.q,
    };

    if (params.geo !== undefined) autocompleteParams.geo = params.geo;
    if (params.hl !== undefined) autocompleteParams.hl = params.hl;

    return autocompleteParams;
}

export function shouldIncludeAutocomplete(input: GoogleTrendsRelatedQueriesInput): boolean {
    return cleanBoolean(input.include_autocomplete, 'include_autocomplete');
}

export function describeGoogleTrendsRelatedQueriesRequest(params: Record<string, unknown>): string {
    const q = typeof params.q === 'string' ? params.q : 'unknown query';
    const filters = ['geo', 'time_range', 'hl', 'search_type']
        .filter((field) => params[field] !== undefined)
        .map((field) => `${field}=${String(params[field])}`);
    const filterSuffix = filters.length > 0 ? ` (${filters.join(', ')})` : '';

    return `"${q}"${filterSuffix}`;
}

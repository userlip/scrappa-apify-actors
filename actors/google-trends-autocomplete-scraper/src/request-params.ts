export interface GoogleTrendsAutocompleteInput {
    query?: unknown;
    q?: unknown;
    geo?: unknown;
    hl?: unknown;
}

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

function cleanQueryInput(input: GoogleTrendsAutocompleteInput): string {
    const query = cleanString(input.query, 'query', 100);
    if (query !== undefined) {
        return query;
    }

    return cleanRequiredString(input.q, 'query', 100);
}

export function buildGoogleTrendsAutocompleteParams(input: GoogleTrendsAutocompleteInput): Record<string, unknown> {
    const params: Record<string, unknown> = {
        q: cleanQueryInput(input),
        geo: 'US',
        hl: 'en',
    };

    const geo = cleanGeo(input.geo);
    const hl = cleanLanguage(input.hl);

    if (geo !== undefined) params.geo = geo;
    if (hl !== undefined) params.hl = hl;

    return params;
}

export function describeGoogleTrendsAutocompleteRequest(params: Record<string, unknown>): string {
    const q = typeof params.q === 'string' ? params.q : 'unknown query';
    const filters = ['geo', 'hl']
        .filter((field) => params[field] !== undefined)
        .map((field) => `${field}=${String(params[field])}`);
    const filterSuffix = filters.length > 0 ? ` (${filters.join(', ')})` : '';

    return `"${q}"${filterSuffix}`;
}

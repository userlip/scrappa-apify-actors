export interface GoogleFinanceSearchInput {
    q?: unknown;
    queries?: unknown;
    hl?: unknown;
    gl?: unknown;
}

export interface GoogleFinanceSearchRequest extends Record<string, unknown> {
    q: string;
    hl?: string;
    gl?: string;
}

const MAX_QUERIES_PER_RUN = 25;

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

function cleanLanguageCode(value: unknown): string | undefined {
    const hl = cleanString(value, 'hl', 10);
    return hl === undefined ? undefined : hl.toLowerCase();
}

function cleanCountryCode(value: unknown): string | undefined {
    const gl = cleanString(value, 'gl', 10);
    if (gl === undefined) {
        return undefined;
    }

    if (!/^[a-z]{2}$/i.test(gl)) {
        throw new Error('gl must be a two-letter country code');
    }

    return gl.toLowerCase();
}

function cleanQueries(input: GoogleFinanceSearchInput): string[] {
    if (input.queries !== undefined && input.queries !== null) {
        if (!Array.isArray(input.queries)) {
            throw new Error('queries must be an array of strings');
        }

        if (input.queries.length > MAX_QUERIES_PER_RUN) {
            throw new Error(`queries can include at most ${MAX_QUERIES_PER_RUN} items per run`);
        }

        const queries = input.queries.map((query, index) => cleanRequiredString(query, `queries[${index}]`, 255));
        if (queries.length === 0) {
            throw new Error('queries must include at least one query');
        }

        return queries;
    }

    return [cleanRequiredString(input.q, 'q', 255)];
}

export function buildGoogleFinanceSearchRequests(input: GoogleFinanceSearchInput): GoogleFinanceSearchRequest[] {
    const hl = cleanLanguageCode(input.hl);
    const gl = cleanCountryCode(input.gl);

    return cleanQueries(input).map((q) => {
        const params: GoogleFinanceSearchRequest = { q };
        if (hl !== undefined) params.hl = hl;
        if (gl !== undefined) params.gl = gl;
        return params;
    });
}

export function describeGoogleFinanceSearchRequest(params: GoogleFinanceSearchRequest): string {
    const filters = ['hl', 'gl']
        .filter((field) => params[field as keyof GoogleFinanceSearchRequest] !== undefined)
        .map((field) => `${field}=${String(params[field as keyof GoogleFinanceSearchRequest])}`);
    const filterSuffix = filters.length > 0 ? ` (${filters.join(', ')})` : '';

    return `"${params.q}"${filterSuffix}`;
}

export function getMaxQueriesPerRun(): number {
    return MAX_QUERIES_PER_RUN;
}

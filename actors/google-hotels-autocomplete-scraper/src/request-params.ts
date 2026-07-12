export interface GoogleHotelsAutocompleteInput {
    queries?: unknown;
    q?: unknown;
    gl?: unknown;
    hl?: unknown;
    currency?: unknown;
    type?: unknown;
}

export interface GoogleHotelsAutocompleteRequest {
    queries: string[];
    commonParams: Record<string, string>;
}

const MAX_QUERIES = 100;
const MAX_QUERY_LENGTH = 200;
const SUGGESTION_TYPES = new Set(['location', 'hotel', 'all']);

function cleanOptionalString(value: unknown, field: string, maxLength: number): string | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    if (typeof value !== 'string') {
        throw new Error(`${field} must be a string`);
    }

    const cleaned = value.trim();
    if (cleaned === '') {
        return undefined;
    }
    if (cleaned.length > maxLength) {
        throw new Error(`${field} must be ${maxLength} characters or fewer`);
    }

    return cleaned;
}

function cleanCode(value: unknown, field: string, length: number): string | undefined {
    const cleaned = cleanOptionalString(value, field, 20);
    if (cleaned === undefined) {
        return undefined;
    }
    if (!new RegExp(`^[A-Za-z]{${length}}$`).test(cleaned)) {
        throw new Error(`${field} must be a ${length}-letter code`);
    }

    return length === 3 ? cleaned.toUpperCase() : cleaned.toLowerCase();
}

function readQueries(value: unknown, field: string): unknown[] {
    if (value === undefined || value === null || value === '') {
        return [];
    }
    if (Array.isArray(value)) {
        return value;
    }
    if (typeof value === 'string') {
        return value.split(',');
    }

    throw new Error(`${field} must be an array of strings or a comma-separated string`);
}

function normalizeQueries(input: GoogleHotelsAutocompleteInput): string[] {
    const candidates = readQueries(input.queries, 'queries');
    const singularQuery = cleanOptionalString(input.q, 'q', MAX_QUERY_LENGTH);
    if (singularQuery !== undefined) {
        candidates.push(singularQuery);
    }
    const queries: string[] = [];
    const seen = new Set<string>();

    for (const [index, candidate] of candidates.entries()) {
        const query = cleanOptionalString(candidate, `queries[${index}]`, MAX_QUERY_LENGTH);
        if (query === undefined) {
            continue;
        }

        const key = query.toLocaleLowerCase('en-US');
        if (!seen.has(key)) {
            seen.add(key);
            queries.push(query);
        }
    }

    if (queries.length === 0) {
        throw new Error('At least one query is required in queries or q');
    }
    if (queries.length > MAX_QUERIES) {
        throw new Error(`A maximum of ${MAX_QUERIES} unique queries is allowed per run`);
    }

    return queries;
}

export function buildGoogleHotelsAutocompleteRequest(
    input: GoogleHotelsAutocompleteInput,
): GoogleHotelsAutocompleteRequest {
    const type = cleanOptionalString(input.type, 'type', 20)?.toLowerCase() ?? 'all';
    if (!SUGGESTION_TYPES.has(type)) {
        throw new Error('type must be one of: location, hotel, all');
    }

    const commonParams: Record<string, string> = {};
    const gl = cleanCode(input.gl, 'gl', 2);
    const hl = cleanCode(input.hl, 'hl', 2);
    const currency = cleanCode(input.currency, 'currency', 3);

    if (gl !== undefined) commonParams.gl = gl;
    if (hl !== undefined) commonParams.hl = hl;
    if (currency !== undefined) commonParams.currency = currency;
    commonParams.type = type;

    return { queries: normalizeQueries(input), commonParams };
}

export function paramsForQuery(
    query: string,
    commonParams: Record<string, string>,
): Record<string, string> {
    return { q: query, ...commonParams };
}

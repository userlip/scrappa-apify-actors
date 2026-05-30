export interface StartpageSearchQueryInput {
    query?: unknown;
    language?: unknown;
    page?: unknown;
    safe_search?: unknown;
}

export interface StartpageSearchInput {
    queries?: unknown;
    max_results_per_query?: unknown;
}

export interface StartpageSearchRequest {
    query: string;
    params: Record<string, unknown>;
}

export interface StartpageSearchPlan {
    requests: StartpageSearchRequest[];
    maxResultsPerQuery: number;
}

const LANGUAGES = [
    'english',
    'deutsch',
    'french',
    'spanish',
    'italian',
    'portuguese',
    'dutch',
    'russian',
    'chinese',
    'japanese',
    'arabic',
    'all',
] as const;

const DEFAULT_MAX_RESULTS_PER_QUERY = 20;
const MAX_QUERIES_PER_RUN = 100;
const MAX_RESULTS_PER_QUERY = 100;

function cleanString(value: unknown, field: string, maxLength: number): string | undefined {
    if (value === undefined || value === null) {
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

function cleanInteger(value: unknown, field: string, min: number, max: number): number | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    if (typeof value !== 'number' || !Number.isInteger(value)) {
        throw new Error(`${field} must be an integer`);
    }

    if (value < min || value > max) {
        throw new Error(`${field} must be between ${min} and ${max}`);
    }

    return value;
}

function cleanBoolean(value: unknown, field: string): boolean | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    if (typeof value !== 'boolean') {
        throw new Error(`${field} must be a boolean`);
    }

    return value;
}

function cleanLanguage(value: unknown): string | undefined {
    const cleaned = cleanString(value, 'language', 50);
    if (cleaned === undefined) {
        return undefined;
    }

    const normalized = cleaned.toLowerCase();
    if (!LANGUAGES.includes(normalized as typeof LANGUAGES[number])) {
        throw new Error(`language must be one of: ${LANGUAGES.join(', ')}`);
    }

    return normalized;
}

function cleanQueries(value: unknown): StartpageSearchQueryInput[] {
    if (!Array.isArray(value)) {
        throw new Error('queries must be an array');
    }

    if (value.length === 0) {
        throw new Error('queries must include at least one search query');
    }

    if (value.length > MAX_QUERIES_PER_RUN) {
        throw new Error(`queries cannot include more than ${MAX_QUERIES_PER_RUN} items`);
    }

    return value.map((item, index) => {
        if (!item || typeof item !== 'object' || Array.isArray(item)) {
            throw new Error(`queries[${index}] must be an object`);
        }

        return item as StartpageSearchQueryInput;
    });
}

export function buildStartpageSearchPlan(input: StartpageSearchInput): StartpageSearchPlan {
    const rawQueries = cleanQueries(input.queries);
    const maxResultsPerQuery = cleanInteger(
        input.max_results_per_query,
        'max_results_per_query',
        1,
        MAX_RESULTS_PER_QUERY,
    ) ?? DEFAULT_MAX_RESULTS_PER_QUERY;

    const requests = rawQueries.map((item, index) => {
        const query = cleanRequiredString(item.query, `queries[${index}].query`, 500);
        const language = cleanLanguage(item.language);
        const page = cleanInteger(item.page, `queries[${index}].page`, 0, 10);
        const safeSearch = cleanBoolean(item.safe_search, `queries[${index}].safe_search`);

        const params: Record<string, unknown> = { query };
        if (language !== undefined) params.language = language;
        if (page !== undefined) params.page = page;
        if (safeSearch !== undefined) params.safe_search = safeSearch ? 1 : 0;

        return { query, params };
    });

    return { requests, maxResultsPerQuery };
}

export function describeStartpageSearchPlan(plan: StartpageSearchPlan): string {
    const sampleQueries = plan.requests.slice(0, 3).map((request) => `"${request.query}"`);
    const suffix = plan.requests.length > sampleQueries.length
        ? ` and ${plan.requests.length - sampleQueries.length} more`
        : '';

    return `${plan.requests.length} query request(s): ${sampleQueries.join(', ')}${suffix}`;
}

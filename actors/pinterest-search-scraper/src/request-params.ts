export interface PinterestSearchInput {
    query?: unknown;
    queries?: unknown;
    limit?: unknown;
    bookmark?: unknown;
}

export interface PinterestSearchRequest {
    query: string;
    params: Record<string, unknown>;
}

export interface PinterestSearchPlan {
    requests: PinterestSearchRequest[];
    limit: number;
    bookmark?: string;
}

const DEFAULT_LIMIT = 50;
const MAX_LIMIT = 250;
const MAX_QUERIES_PER_RUN = 100;

function decodeInputString(value: string): string {
    if (!value.includes('%')) {
        return value;
    }

    try {
        return decodeURIComponent(value);
    } catch {
        return value;
    }
}

function cleanString(value: unknown, field: string, maxLength: number): string | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    if (typeof value !== 'string') {
        throw new Error(`${field} must be a string`);
    }

    const trimmed = decodeInputString(value).trim();
    if (trimmed === '') {
        return undefined;
    }

    if (trimmed.length > maxLength) {
        throw new Error(`${field} must be ${maxLength} characters or fewer`);
    }

    return trimmed;
}

function cleanInteger(value: unknown, field: string, min: number, max: number): number | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    const normalized = typeof value === 'string' && value.trim() !== '' && /^\d+$/.test(value.trim())
        ? Number(value.trim())
        : value;

    if (typeof normalized !== 'number' || !Number.isInteger(normalized)) {
        throw new Error(`${field} must be an integer`);
    }

    if (normalized < min || normalized > max) {
        throw new Error(`${field} must be between ${min} and ${max}`);
    }

    return normalized;
}

function cleanQueries(input: PinterestSearchInput): string[] {
    const queries: string[] = [];
    const singleQuery = cleanString(input.query, 'query', 200);

    if (singleQuery !== undefined) {
        queries.push(singleQuery);
    }

    if (input.queries !== undefined && input.queries !== null && input.queries !== '') {
        if (!Array.isArray(input.queries)) {
            throw new Error('queries must be an array of strings');
        }

        for (const [index, value] of input.queries.entries()) {
            const query = cleanString(value, `queries[${index}]`, 200);
            if (query !== undefined) {
                queries.push(query);
            }
        }
    }

    const uniqueQueries = Array.from(new Set(queries));
    if (uniqueQueries.length === 0) {
        throw new Error('Provide at least one Pinterest search query using queries or query');
    }

    if (uniqueQueries.length > MAX_QUERIES_PER_RUN) {
        throw new Error(`queries cannot contain more than ${MAX_QUERIES_PER_RUN} values per run`);
    }

    return uniqueQueries;
}

export function buildPinterestSearchPlan(input: PinterestSearchInput): PinterestSearchPlan {
    const queries = cleanQueries(input);
    const limit = cleanInteger(input.limit, 'limit', 1, MAX_LIMIT) ?? DEFAULT_LIMIT;
    const bookmark = cleanString(input.bookmark, 'bookmark', 2000);

    const requests = queries.map((query) => {
        const params: Record<string, unknown> = {
            query,
            limit,
        };

        if (bookmark !== undefined) {
            params.bookmark = bookmark;
        }

        return { query, params };
    });

    return { requests, limit, bookmark };
}

export function describePinterestSearchRequest(plan: PinterestSearchPlan): string {
    const queryLabel = plan.requests.length === 1
        ? `"${plan.requests[0].query}"`
        : `${plan.requests.length} queries`;
    const bookmarkLabel = plan.bookmark ? ', with bookmark' : '';

    return `${queryLabel} (${plan.limit} pins/query${bookmarkLabel})`;
}

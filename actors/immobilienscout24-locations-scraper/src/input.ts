export interface LocationsInput {
    queries?: unknown;
    query?: unknown;
    limit?: unknown;
}

export interface LocationsRequest {
    query: string;
    limit: number;
}

const DEFAULT_LIMIT = 10;
const MAX_QUERIES = 100;

export function buildLocationRequests(input: LocationsInput): LocationsRequest[] {
    const rawQueries = getRawQueries(input);
    const limit = parseLimit(input.limit);
    const seen = new Set<string>();
    const queries: string[] = [];

    for (const rawQuery of rawQueries) {
        if (typeof rawQuery !== 'string') {
            throw new Error('Each query must be a string');
        }

        const query = rawQuery.trim();
        if (!query) {
            throw new Error('Queries must not be empty');
        }
        if (query.length > 120) {
            throw new Error('Each query must be at most 120 characters');
        }

        const key = query.toLocaleLowerCase('de-DE');
        if (!seen.has(key)) {
            seen.add(key);
            queries.push(query);
        }
    }

    if (queries.length === 0) {
        throw new Error('Provide at least one location in queries or query');
    }
    if (queries.length > MAX_QUERIES) {
        throw new Error(`A run supports at most ${MAX_QUERIES} unique queries`);
    }

    return queries.map((query) => ({ query, limit }));
}

function getRawQueries(input: LocationsInput): unknown[] {
    if (input.queries !== undefined) {
        if (!Array.isArray(input.queries)) {
            throw new Error('queries must be an array of strings');
        }
        return input.queries;
    }

    return input.query === undefined ? [] : [input.query];
}

function parseLimit(value: unknown): number {
    if (value === undefined) {
        return DEFAULT_LIMIT;
    }
    if (typeof value !== 'number' || !Number.isInteger(value)) {
        throw new Error('limit must be an integer');
    }
    if (value < 1 || value > 20) {
        throw new Error('limit must be between 1 and 20');
    }
    return value;
}

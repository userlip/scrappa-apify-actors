export interface IndeedJobsInput {
    query?: string;
    location?: string;
    country?: string;
    radius?: number;
    radius_unit?: 'MILES' | 'KILOMETERS' | string;
    job_type?: 'full_time' | 'part_time' | 'contract' | 'internship' | 'remote' | string;
    sort?: 'relevance' | 'date' | string;
    limit?: number;
    cursor?: string;
    hl?: string;
    gl?: string;
}

export const DEFAULT_INDEED_JOBS_QUERY = 'software engineer';

export const DEFAULT_INDEED_JOBS_INPUT: IndeedJobsInput = {
    query: DEFAULT_INDEED_JOBS_QUERY,
    location: 'New York',
    country: 'US',
    limit: 20,
};

const KNOWN_INPUT_KEYS = new Set<keyof IndeedJobsInput>([
    'query',
    'location',
    'country',
    'radius',
    'radius_unit',
    'job_type',
    'sort',
    'limit',
    'cursor',
    'hl',
    'gl',
]);

export function normalizeIndeedJobsInput(input?: IndeedJobsInput | null): IndeedJobsInput {
    if (!input) {
        return { ...DEFAULT_INDEED_JOBS_INPUT };
    }

    const normalized = normalizeFields(input);
    const hasKnownInput = Object.entries(normalized).some(([key, value]) => {
        return KNOWN_INPUT_KEYS.has(key as keyof IndeedJobsInput) && value !== undefined && value !== '';
    });

    if (!hasKnownInput) {
        return { ...DEFAULT_INDEED_JOBS_INPUT };
    }

    return {
        ...DEFAULT_INDEED_JOBS_INPUT,
        ...normalized,
        query: normalized.query ?? DEFAULT_INDEED_JOBS_QUERY,
    };
}

export function buildIndeedJobsParams(input: IndeedJobsInput): Record<string, unknown> {
    const params: Record<string, unknown> = {};
    const entries: [keyof IndeedJobsInput, unknown][] = [
        ['query', input.query],
        ['location', input.location],
        ['country', input.country],
        ['radius', input.radius],
        ['radius_unit', input.radius_unit],
        ['job_type', input.job_type],
        ['sort', input.sort],
        ['limit', input.limit],
        ['cursor', input.cursor],
        ['hl', input.hl],
        ['gl', input.gl],
    ];

    for (const [key, value] of entries) {
        if (value !== undefined) {
            params[key] = value;
        }
    }

    return params;
}

function normalizeFields(input: IndeedJobsInput): IndeedJobsInput {
    const normalized: IndeedJobsInput = {};

    for (const [key, value] of Object.entries(input) as [keyof IndeedJobsInput, unknown][]) {
        if (!KNOWN_INPUT_KEYS.has(key)) {
            continue;
        }

        if (typeof value === 'string') {
            const trimmed = value.trim();
            if (trimmed !== '') {
                normalized[key] = normalizeStringValue(key, trimmed) as never;
            }
        } else if (value !== undefined && value !== null) {
            normalized[key] = value as never;
        }
    }

    return normalized;
}

function normalizeStringValue(key: keyof IndeedJobsInput, value: string): string {
    if (key === 'country' || key === 'gl' || key === 'radius_unit') {
        return value.toUpperCase();
    }

    if (key === 'hl') {
        return value.toLowerCase();
    }

    return value;
}

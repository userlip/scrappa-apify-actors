export interface StepstoneJobsInput {
    query?: string;
    location?: string;
    country?: 'de' | 'at' | 'nl' | 'be' | string;
    radius?: number;
    sort?: 'relevance' | 'date' | string;
    job_type?: 'full_time' | 'part_time' | 'internship' | 'freelance' | string;
    work_from_home?: boolean;
    date_posted?: number;
    page?: number;
    limit?: number;
}

export const DEFAULT_STEPSTONE_JOBS_QUERY = 'software engineer';

export const DEFAULT_STEPSTONE_JOBS_INPUT: StepstoneJobsInput = {
    query: DEFAULT_STEPSTONE_JOBS_QUERY,
    location: 'Berlin',
    country: 'de',
    page: 1,
    limit: 25,
};

const KNOWN_INPUT_KEYS = new Set<keyof StepstoneJobsInput>([
    'query',
    'location',
    'country',
    'radius',
    'sort',
    'job_type',
    'work_from_home',
    'date_posted',
    'page',
    'limit',
]);

export function normalizeStepstoneJobsInput(input?: StepstoneJobsInput | null): StepstoneJobsInput {
    if (!input) {
        return { ...DEFAULT_STEPSTONE_JOBS_INPUT };
    }

    const normalized = normalizeFields(input);
    const hasKnownInput = Object.entries(normalized).some(([key, value]) => {
        return KNOWN_INPUT_KEYS.has(key as keyof StepstoneJobsInput) && value !== undefined && value !== '';
    });

    if (!hasKnownInput) {
        return { ...DEFAULT_STEPSTONE_JOBS_INPUT };
    }

    return {
        ...DEFAULT_STEPSTONE_JOBS_INPUT,
        ...normalized,
        query: normalized.query ?? DEFAULT_STEPSTONE_JOBS_QUERY,
    };
}

export function buildStepstoneJobsParams(input: StepstoneJobsInput): Record<string, unknown> {
    const params: Record<string, unknown> = {};
    const entries: [keyof StepstoneJobsInput, unknown][] = [
        ['query', input.query],
        ['location', input.location],
        ['country', input.country],
        ['radius', input.radius],
        ['sort', input.sort],
        ['job_type', input.job_type],
        ['date_posted', input.date_posted],
        ['page', input.page],
        ['limit', input.limit],
    ];

    for (const [key, value] of entries) {
        if (value !== undefined) {
            params[key] = value;
        }
    }

    // Laravel validation rejects query-string booleans; send integer 1 for true and omit false.
    if (input.work_from_home === true) {
        params.work_from_home = 1;
    }

    return params;
}

function normalizeFields(input: StepstoneJobsInput): StepstoneJobsInput {
    const normalized: StepstoneJobsInput = {};

    for (const [key, value] of Object.entries(input) as [keyof StepstoneJobsInput, unknown][]) {
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

function normalizeStringValue(key: keyof StepstoneJobsInput, value: string): string {
    if (key === 'country') {
        return value.toLowerCase();
    }

    return value;
}

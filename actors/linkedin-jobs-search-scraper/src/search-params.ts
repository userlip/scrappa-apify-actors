export interface LinkedInJobsSearchInput {
    query?: string;
    num?: number;
    page?: number;
    start?: number;
    hl?: string;
    lr?: string;
    gl?: string;
    cr?: string;
    safe?: 'active' | 'off' | string;
    dateRestrict?: string;
    sort?: string;
    filter?: 0 | 1 | number;
    rights?: string;
}

export const DEFAULT_LINKEDIN_JOBS_SEARCH_QUERY = 'software engineer remote';

export const DEFAULT_LINKEDIN_JOBS_SEARCH_INPUT: LinkedInJobsSearchInput = {
    query: DEFAULT_LINKEDIN_JOBS_SEARCH_QUERY,
    num: 10,
    hl: 'en',
    gl: 'us',
    safe: 'off',
};

const KNOWN_INPUT_KEYS = new Set<keyof LinkedInJobsSearchInput>([
    'query',
    'num',
    'page',
    'start',
    'hl',
    'lr',
    'gl',
    'cr',
    'safe',
    'dateRestrict',
    'sort',
    'filter',
    'rights',
]);

export function normalizeLinkedInJobsSearchInput(input?: LinkedInJobsSearchInput | null): LinkedInJobsSearchInput {
    if (!input) {
        return { ...DEFAULT_LINKEDIN_JOBS_SEARCH_INPUT };
    }

    const normalized = trimStringFields(input);
    const hasKnownInput = Object.entries(normalized).some(([key, value]) => {
        return KNOWN_INPUT_KEYS.has(key as keyof LinkedInJobsSearchInput) && value !== undefined && value !== '';
    });

    if (!hasKnownInput) {
        return { ...DEFAULT_LINKEDIN_JOBS_SEARCH_INPUT };
    }

    if (normalized.query) {
        return {
            ...DEFAULT_LINKEDIN_JOBS_SEARCH_INPUT,
            ...normalized,
        };
    }

    return {
        ...DEFAULT_LINKEDIN_JOBS_SEARCH_INPUT,
        ...normalized,
        query: DEFAULT_LINKEDIN_JOBS_SEARCH_QUERY,
    };
}

export function buildLinkedInJobsSearchParams(input: LinkedInJobsSearchInput): Record<string, unknown> {
    const params: Record<string, unknown> = {};
    const entries: [keyof LinkedInJobsSearchInput, unknown][] = [
        ['query', input.query],
        ['num', input.num],
        ['page', input.page],
        ['start', input.start],
        ['hl', input.hl],
        ['lr', input.lr],
        ['gl', input.gl],
        ['cr', input.cr],
        ['safe', input.safe],
        ['dateRestrict', input.dateRestrict],
        ['sort', input.sort],
        ['filter', input.filter],
        ['rights', input.rights],
    ];

    for (const [key, value] of entries) {
        if (value !== undefined) {
            params[key] = value;
        }
    }

    return params;
}

function trimStringFields(input: LinkedInJobsSearchInput): LinkedInJobsSearchInput {
    const normalized: LinkedInJobsSearchInput = {};

    for (const [key, value] of Object.entries(input) as [keyof LinkedInJobsSearchInput, unknown][]) {
        if (!KNOWN_INPUT_KEYS.has(key)) {
            continue;
        }

        if (typeof value === 'string') {
            const trimmed = value.trim();
            if (trimmed !== '') {
                normalized[key] = trimmed as never;
            }
        } else if (value !== undefined && value !== null) {
            normalized[key] = value as never;
        }
    }

    return normalized;
}

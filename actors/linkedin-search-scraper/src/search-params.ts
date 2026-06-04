export interface LinkedInSearchInput {
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
    filter?: number;
    rights?: string;
}

export const DEFAULT_LINKEDIN_SEARCH_QUERY = 'site:linkedin.com/in founder AI Berlin';

export const DEFAULT_LINKEDIN_SEARCH_INPUT: LinkedInSearchInput = {
    query: DEFAULT_LINKEDIN_SEARCH_QUERY,
    num: 10,
    hl: 'en',
    gl: 'us',
    safe: 'off',
};

const KNOWN_INPUT_KEYS = new Set<keyof LinkedInSearchInput>([
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

export function normalizeLinkedInSearchInput(input?: LinkedInSearchInput | null): LinkedInSearchInput {
    if (!input) {
        return { ...DEFAULT_LINKEDIN_SEARCH_INPUT };
    }

    const normalized = trimStringFields(input);
    const hasKnownInput = Object.entries(normalized).some(([key, value]) => {
        return KNOWN_INPUT_KEYS.has(key as keyof LinkedInSearchInput) && value !== undefined && value !== '';
    });

    if (!hasKnownInput) {
        return { ...DEFAULT_LINKEDIN_SEARCH_INPUT };
    }

    if (normalized.query) {
        return {
            ...DEFAULT_LINKEDIN_SEARCH_INPUT,
            ...normalized,
        };
    }

    return {
        ...DEFAULT_LINKEDIN_SEARCH_INPUT,
        ...normalized,
        query: DEFAULT_LINKEDIN_SEARCH_QUERY,
    };
}

export function validateLinkedInSearchInput(input: LinkedInSearchInput): void {
    if (!input.query) {
        throw new Error('LinkedIn search query is required.');
    }

    assertIntegerRange(input.num, 'num', 1, 20);
    assertIntegerRange(input.page, 'page', 1, 10);
    assertIntegerRange(input.start, 'start', 0, 170);
    assertIntegerRange(input.filter, 'filter', 0, 1);

    if (input.page !== undefined && input.start !== undefined) {
        throw new Error('Use either page or start for pagination, not both.');
    }
}

export function limitLinkedInSearchResultCount(input: LinkedInSearchInput, maxResults: number | null): LinkedInSearchInput {
    if (maxResults === null) {
        return input;
    }

    return {
        ...input,
        num: Math.min(input.num ?? DEFAULT_LINKEDIN_SEARCH_INPUT.num ?? 10, maxResults),
    };
}

export function buildLinkedInSearchParams(input: LinkedInSearchInput): Record<string, unknown> {
    const params: Record<string, unknown> = {};
    const entries: [keyof LinkedInSearchInput, unknown][] = [
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

function trimStringFields(input: LinkedInSearchInput): LinkedInSearchInput {
    const normalized: LinkedInSearchInput = {};

    for (const [key, value] of Object.entries(input) as [keyof LinkedInSearchInput, unknown][]) {
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

function assertIntegerRange(value: unknown, field: string, min: number, max: number): void {
    if (value === undefined) {
        return;
    }

    if (typeof value !== 'number' || !Number.isInteger(value) || value < min || value > max) {
        throw new Error(`${field} must be an integer from ${min} to ${max}.`);
    }
}

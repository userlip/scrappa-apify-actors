export interface GoogleJobsInput {
    q?: string;
    next_page_token?: string;
    hl?: string;
    gl?: string;
    google_domain?: string;
    uule?: string;
    lrad?: number;
    uds?: string;
}

export const DEFAULT_JOB_SEARCH_QUERY = 'nurse jobs in Austin';

export const DEFAULT_JOB_SEARCH_INPUT: GoogleJobsInput = {
    q: DEFAULT_JOB_SEARCH_QUERY,
    gl: 'us',
    hl: 'en',
    google_domain: 'google.com',
};

const KNOWN_INPUT_KEYS = new Set<keyof GoogleJobsInput>([
    'q',
    'next_page_token',
    'hl',
    'gl',
    'google_domain',
    'uule',
    'lrad',
    'uds',
]);

export function normalizeJobsInput(input?: GoogleJobsInput | null): GoogleJobsInput {
    if (!input) {
        return { ...DEFAULT_JOB_SEARCH_INPUT };
    }

    const normalized = trimStringFields(input);
    const hasKnownInput = Object.entries(normalized).some(([key, value]) => {
        return KNOWN_INPUT_KEYS.has(key as keyof GoogleJobsInput) && value !== undefined && value !== '';
    });

    if (!hasKnownInput) {
        return { ...DEFAULT_JOB_SEARCH_INPUT };
    }

    if (normalized.q || normalized.next_page_token) {
        return normalized;
    }

    return {
        ...DEFAULT_JOB_SEARCH_INPUT,
        ...normalized,
    };
}

export function buildJobsParams(input: GoogleJobsInput): Record<string, unknown> {
    const params: Record<string, unknown> = {};
    const entries: [keyof GoogleJobsInput, unknown][] = [
        ['q', input.q],
        ['next_page_token', input.next_page_token],
        ['hl', input.hl],
        ['gl', input.gl],
        ['google_domain', input.google_domain],
        ['uule', input.uule],
        ['lrad', input.lrad],
        ['uds', input.uds],
    ];

    for (const [key, value] of entries) {
        if (value !== undefined) {
            params[key] = value;
        }
    }

    return params;
}

function trimStringFields(input: GoogleJobsInput): GoogleJobsInput {
    const normalized: GoogleJobsInput = {};

    for (const [key, value] of Object.entries(input) as [keyof GoogleJobsInput, unknown][]) {
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

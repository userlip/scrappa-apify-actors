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

export const DEFAULT_JOB_SEARCH_QUERY = 'software engineer';

export function normalizeJobsInput(input?: GoogleJobsInput | null): GoogleJobsInput {
    if (!input) {
        return { q: DEFAULT_JOB_SEARCH_QUERY };
    }

    if (Object.keys(input).length === 0) {
        return { q: DEFAULT_JOB_SEARCH_QUERY };
    }

    if (input.q || input.next_page_token) {
        return input;
    }

    return input;
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

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

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
    return {
        q: input.q,
        next_page_token: input.next_page_token,
        hl: input.hl,
        gl: input.gl,
        google_domain: input.google_domain,
        uule: input.uule,
        lrad: input.lrad,
        uds: input.uds,
    };
}

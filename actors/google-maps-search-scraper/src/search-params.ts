export interface GoogleMapsSearchInput {
    query: string;
    hl?: string;
    gl?: string;
    debug?: boolean;
    use_cache?: boolean;
    maximum_cache_age?: number;
    fallback_zoom?: number;
}

export function buildSearchParams(input: GoogleMapsSearchInput): Record<string, unknown> {
    const params: Record<string, unknown> = {
        query: input.query,
        hl: input.hl ?? 'en',
    };

    if (input.gl !== undefined) {
        params.gl = input.gl;
    }

    // Also forward debug to the API for server-side diagnostics when enabled.
    if (input.debug !== undefined) {
        params.debug = input.debug;
    }

    if (input.use_cache !== false) {
        params.use_cache = 1;
        if (
            typeof input.maximum_cache_age === 'number'
            && Number.isInteger(input.maximum_cache_age)
            && input.maximum_cache_age >= 1
        ) {
            params.maximum_cache_age = input.maximum_cache_age;
        }
    }

    return params;
}

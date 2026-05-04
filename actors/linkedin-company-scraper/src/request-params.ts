export interface LinkedInCompanyRequestInput {
    url: string;
    use_cache?: boolean;
    // Input is deserialized from Actor.getInput(), so validate at runtime before forwarding it.
    maximum_cache_age?: unknown;
}

export function buildLinkedInCompanyParams(input: LinkedInCompanyRequestInput): Record<string, unknown> {
    const params: Record<string, unknown> = {
        url: input.url,
    };

    if (!input.use_cache) {
        return params;
    }

    params.use_cache = 1;

    if (
        typeof input.maximum_cache_age === 'number'
        && Number.isInteger(input.maximum_cache_age)
        && input.maximum_cache_age >= 1
    ) {
        params.maximum_cache_age = input.maximum_cache_age;
    }

    return params;
}

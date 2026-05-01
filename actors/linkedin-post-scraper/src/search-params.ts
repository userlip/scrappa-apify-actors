export interface LinkedInPostInput {
    url: string;
    use_cache?: boolean;
    maximum_cache_age?: number;
}

export function buildLinkedInPostParams(
    input: LinkedInPostInput,
    warn: (message: string) => void = console.warn,
): Record<string, unknown> {
    const params: Record<string, unknown> = {
        url: input.url,
    };

    if (input.use_cache) {
        params.use_cache = 1;
    }

    if (input.maximum_cache_age !== undefined) {
        if (input.maximum_cache_age >= 1) {
            params.maximum_cache_age = input.maximum_cache_age;
        } else {
            warn(`maximum_cache_age must be at least 1, got ${input.maximum_cache_age}. Using default cache behavior.`);
        }
    }

    return params;
}

import { normalizeLinkedInProfileUrl } from './url.js';

export interface LinkedInProfileInput {
    url?: string;
    urls?: string[];
    use_cache?: boolean;
    maximum_cache_age?: number;
}

export interface LinkedInProfileUrlRequest {
    input_url: string;
    normalized_url?: string;
    validation_error?: string;
}

export function getInputUrls(input: LinkedInProfileInput | null): LinkedInProfileUrlRequest[] {
    const rawUrls = Array.isArray(input?.urls) && input.urls.length > 0
        ? input.urls
        : typeof input?.url === 'string' ? [input.url] : [];

    const seen = new Set<string>();
    const requests: LinkedInProfileUrlRequest[] = [];

    for (const rawUrl of rawUrls) {
        const inputUrl = rawUrl.trim();
        if (!inputUrl) {
            continue;
        }

        try {
            const normalizedUrl = normalizeLinkedInProfileUrl(inputUrl);
            if (seen.has(normalizedUrl)) {
                continue;
            }

            seen.add(normalizedUrl);
            requests.push({
                input_url: inputUrl,
                normalized_url: normalizedUrl,
            });
        } catch (error) {
            const key = `invalid:${inputUrl}`;
            if (seen.has(key)) {
                continue;
            }

            seen.add(key);
            requests.push({
                input_url: inputUrl,
                validation_error: error instanceof Error ? error.message : String(error),
            });
        }
    }

    return requests;
}

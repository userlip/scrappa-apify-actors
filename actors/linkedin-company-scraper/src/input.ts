import { normalizeLinkedInCompanyUrl } from './url.js';

export interface LinkedInCompanyInput {
    url?: string;
    urls?: string[];
    use_cache?: boolean;
    maximum_cache_age?: number;
}

export interface LinkedInCompanyUrlRequest {
    input_url: string;
    normalized_url?: string;
    validation_error?: string;
}

export function getInputUrls(input: LinkedInCompanyInput | null): LinkedInCompanyUrlRequest[] {
    const rawUrls = [
        ...(typeof input?.url === 'string' ? [input.url] : []),
        ...(Array.isArray(input?.urls) ? input.urls : []),
    ];

    const seen = new Set<string>();
    const requests: LinkedInCompanyUrlRequest[] = [];

    for (const rawUrl of rawUrls) {
        const inputUrl = rawUrl.trim();
        if (!inputUrl) {
            continue;
        }

        try {
            const normalizedUrl = normalizeLinkedInCompanyUrl(inputUrl);
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

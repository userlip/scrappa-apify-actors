export type WebScraperResponseType = 'json' | 'markdown';

export interface WebsiteContentExtractorInput {
    url?: unknown;
    urls?: unknown;
    include_html?: boolean;
    response_type?: WebScraperResponseType;
}

export interface UrlRequest {
    input_url: string;
    request_url: string;
}

export function getResponseType(input: WebsiteContentExtractorInput | null): WebScraperResponseType {
    const responseType = input?.response_type ?? 'json';
    if (responseType !== 'json' && responseType !== 'markdown') {
        throw new Error('response_type must be either "json" or "markdown".');
    }

    return responseType;
}

export function getInputUrls(input: WebsiteContentExtractorInput | null): UrlRequest[] {
    const rawUrls = [
        ...(input?.url !== undefined && input.url !== null ? [{ value: input.url, field: 'url' }] : []),
        ...(Array.isArray(input?.urls) ? input.urls : []),
    ];

    const seen = new Set<string>();
    const requests: UrlRequest[] = [];

    for (const [index, rawUrl] of rawUrls.entries()) {
        const value = typeof rawUrl === 'object' && rawUrl !== null && 'value' in rawUrl
            ? (rawUrl as { value: unknown }).value
            : rawUrl;
        const field = typeof rawUrl === 'object' && rawUrl !== null && 'field' in rawUrl
            ? String((rawUrl as { field: unknown }).field)
            : `urls[${index - (input?.url !== undefined && input.url !== null ? 1 : 0)}]`;

        if (typeof value !== 'string') {
            throw new Error(`${field} must be a string.`);
        }

        const inputUrl = value.trim();
        if (!inputUrl) {
            continue;
        }

        const dedupeKey = normalizeForDedupe(inputUrl);
        if (seen.has(dedupeKey)) {
            continue;
        }

        seen.add(dedupeKey);
        requests.push({
            input_url: inputUrl,
            request_url: inputUrl,
        });
    }

    return requests;
}

function normalizeForDedupe(url: string): string {
    const candidate = /^https?:\/\//i.test(url) ? url : `https://${url}`;

    try {
        const parsed = new URL(candidate);
        parsed.hash = '';
        return parsed.toString();
    } catch {
        return url;
    }
}

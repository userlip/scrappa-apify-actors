export type WebScraperResponseType = 'json' | 'markdown';

export interface WebsiteContentExtractorInput {
    url?: string;
    urls?: string[];
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
        ...(typeof input?.url === 'string' ? [input.url] : []),
        ...(Array.isArray(input?.urls) ? input.urls : []),
    ];

    const seen = new Set<string>();
    const requests: UrlRequest[] = [];

    for (const rawUrl of rawUrls) {
        const inputUrl = rawUrl.trim();
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
        new URL(candidate);
        const withoutHash = candidate.split('#')[0] ?? candidate;
        const match = withoutHash.match(/^(https?):\/\/([^/?#]*)(.*)$/i);
        if (!match) {
            return withoutHash;
        }

        const [, protocol, host, pathAndSearch] = match;
        return `${protocol.toLowerCase()}://${host.toLowerCase()}${pathAndSearch}`;
    } catch {
        return url;
    }
}

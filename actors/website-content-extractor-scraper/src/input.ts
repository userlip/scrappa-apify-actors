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
        ...(Array.isArray(input?.urls) ? input.urls.filter((url): url is string => typeof url === 'string') : []),
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
    const hasProtocol = /^https?:\/\//i.test(url);
    const candidate = hasProtocol ? url : `https://${url}`;

    try {
        new URL(candidate);
        if (!hasProtocol) {
            return `schemeless:${url}`;
        }

        const match = candidate.match(/^(https?):\/\/([^/?#]*)(.*)$/i);
        if (!match) {
            return candidate;
        }

        const [, protocol, authority, pathAndSearch] = match;
        return `${protocol.toLowerCase()}://${normalizeAuthorityForDedupe(authority)}${pathAndSearch}`;
    } catch {
        return url;
    }
}

function normalizeAuthorityForDedupe(authority: string): string {
    const userInfoSeparator = authority.lastIndexOf('@');
    const userInfo = userInfoSeparator >= 0 ? authority.slice(0, userInfoSeparator + 1) : '';
    const hostAndPort = userInfoSeparator >= 0 ? authority.slice(userInfoSeparator + 1) : authority;

    if (hostAndPort.startsWith('[')) {
        const bracketEnd = hostAndPort.indexOf(']');
        if (bracketEnd >= 0) {
            return `${userInfo}${hostAndPort.slice(0, bracketEnd + 1).toLowerCase()}${hostAndPort.slice(bracketEnd + 1)}`;
        }
    }

    const portSeparator = hostAndPort.lastIndexOf(':');
    const hasSingleColon = portSeparator >= 0 && hostAndPort.indexOf(':') === portSeparator;
    const host = hasSingleColon ? hostAndPort.slice(0, portSeparator) : hostAndPort;
    const port = hasSingleColon ? hostAndPort.slice(portSeparator) : '';

    return `${userInfo}${host.toLowerCase()}${port}`;
}

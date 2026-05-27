import type { WebScraperResponseType, WebsiteContentExtractorInput, UrlRequest } from './input.js';

export interface WebScraperParams {
    url: string;
    include_html?: boolean;
    response_type: WebScraperResponseType;
}

export function buildWebScraperParams(
    request: UrlRequest,
    input: WebsiteContentExtractorInput | null,
    responseType: WebScraperResponseType,
): WebScraperParams {
    return {
        url: request.request_url,
        include_html: responseType === 'json' ? input?.include_html === true : undefined,
        response_type: responseType,
    };
}

export function describeWebScraperRequest(params: WebScraperParams): string {
    const parts = [`url=${params.url}`, `response_type=${params.response_type}`];
    if (params.response_type === 'json') {
        parts.push(`include_html=${params.include_html === true ? 'true' : 'false'}`);
    }

    return parts.join(', ');
}

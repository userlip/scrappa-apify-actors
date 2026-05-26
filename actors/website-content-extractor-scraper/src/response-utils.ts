import {
    ScrappaWebScraperHttpError,
    ScrappaWebScraperTimeoutError,
} from './web-scraper-client.js';
import type { UrlRequest } from './input.js';
import type { WebScraperParams } from './request-params.js';

export interface WebScraperJsonResponse {
    success?: boolean;
    site_status_code?: number;
    url?: string;
    final_url?: string;
    data?: {
        title?: string | null;
        description?: string | null;
        keywords?: string[];
        favicon?: string | null;
        social_links?: Record<string, string | null>;
        extracted_keywords?: string[];
        links?: string[];
        emails?: string[];
        phone_numbers?: string[];
        images?: string[];
        body_text?: string | null;
        languages_detected?: string[];
        html?: string | null;
        [key: string]: unknown;
    };
    error?: string;
    error_code?: string;
    diagnostics?: unknown;
    [key: string]: unknown;
}

export interface WebScraperDatasetItem extends Record<string, unknown> {
    success: boolean;
    input_url: string;
    request_url: string;
    response_type: 'json' | 'markdown';
    include_html: boolean;
    url?: string;
    final_url?: string | null;
    site_status_code?: number | null;
    error?: string;
    error_type?: string;
}

export function buildJsonDatasetItem(
    response: WebScraperJsonResponse,
    request: UrlRequest,
    params: WebScraperParams,
): WebScraperDatasetItem {
    const data = response.data ?? {};

    return {
        ...response,
        success: response.success === true,
        input_url: request.input_url,
        request_url: params.url,
        response_type: 'json',
        include_html: params.include_html === true,
        url: response.url ?? params.url,
        final_url: response.final_url ?? null,
        site_status_code: response.site_status_code ?? null,
        title: data.title ?? null,
        description: data.description ?? null,
        body_text: data.body_text ?? null,
        links_count: Array.isArray(data.links) ? data.links.length : 0,
        emails_count: Array.isArray(data.emails) ? data.emails.length : 0,
        phone_numbers_count: Array.isArray(data.phone_numbers) ? data.phone_numbers.length : 0,
        images_count: Array.isArray(data.images) ? data.images.length : 0,
        languages_detected: data.languages_detected ?? [],
    };
}

export function buildMarkdownDatasetItem(
    markdown: string,
    request: UrlRequest,
    params: WebScraperParams,
): WebScraperDatasetItem {
    return {
        success: true,
        input_url: request.input_url,
        request_url: params.url,
        response_type: 'markdown',
        include_html: false,
        url: params.url,
        final_url: null,
        site_status_code: null,
        markdown,
        markdown_length: Array.from(markdown).length,
    };
}

export function buildFailureDatasetItem(
    error: unknown,
    request: UrlRequest,
    params: WebScraperParams,
): WebScraperDatasetItem {
    const message = describeError(error);
    const statusCode = error instanceof ScrappaWebScraperHttpError ? error.status : null;
    const body = error instanceof ScrappaWebScraperHttpError ? error.body : null;

    return {
        success: false,
        input_url: request.input_url,
        request_url: params.url,
        response_type: params.response_type,
        include_html: params.include_html === true,
        url: params.url,
        final_url: null,
        site_status_code: null,
        status_code: statusCode,
        error: message,
        error_type: classifyError(error),
        error_code: body?.error_code ?? null,
        diagnostics: body?.diagnostics ?? null,
    };
}

export function isSuccessfulDatasetItem(item: WebScraperDatasetItem): boolean {
    return item.success === true;
}

function classifyError(error: unknown): string {
    if (error instanceof ScrappaWebScraperHttpError) {
        return 'scrappa_api_error';
    }

    if (error instanceof ScrappaWebScraperTimeoutError) {
        return 'scrappa_api_timeout';
    }

    return 'error';
}

function describeError(error: unknown): string {
    if (error instanceof Error) {
        return error.message;
    }

    if (typeof error === 'string') {
        return error;
    }

    try {
        const json = JSON.stringify(error);
        if (json !== undefined) {
            return json;
        }
    } catch {
        return String(error);
    }

    return String(error);
}

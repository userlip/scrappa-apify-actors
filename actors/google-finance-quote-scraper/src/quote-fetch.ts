import type { GoogleFinanceQuoteResponse } from './response-utils.js';
import { ScrappaClient, ScrappaHttpError } from './shared/index.js';

export interface QuoteFetchResult {
    response: GoogleFinanceQuoteResponse;
    fallback?: {
        reason: string;
        omitted_params: string[];
        primary_error: string;
        source_endpoint?: string;
        unavailable_sections?: string[];
    };
}

interface GoogleFinanceSearchResult {
    symbol?: unknown;
    exchange?: unknown;
    name?: unknown;
    currency?: unknown;
    current_price?: unknown;
    price_change?: unknown;
    percent_change?: unknown;
    previous_close?: unknown;
    country?: unknown;
}

interface GoogleFinanceSearchResponse {
    results?: GoogleFinanceSearchResult[];
}

export function shouldRetryBaseQuote(error: unknown, params: Record<string, unknown>): error is ScrappaHttpError {
    return error instanceof ScrappaHttpError
        && error.status >= 500
        && error.status <= 599
        && params.period_type !== undefined;
}

function normalizeString(value: unknown): string | undefined {
    return typeof value === 'string' && value.trim() !== '' ? value.trim() : undefined;
}

function matchesRequestedQuote(result: GoogleFinanceSearchResult, params: Record<string, unknown>): boolean {
    const resultSymbol = normalizeString(result.symbol)?.toUpperCase();
    const requestSymbol = normalizeString(params.symbol)?.toUpperCase();
    const resultExchange = normalizeString(result.exchange)?.toUpperCase();
    const requestExchange = normalizeString(params.exchange)?.toUpperCase();

    if (!resultSymbol || resultSymbol !== requestSymbol) {
        return false;
    }

    return !requestExchange || resultExchange === requestExchange;
}

export function buildQuoteResponseFromSearchResult(
    result: GoogleFinanceSearchResult,
    params: Record<string, unknown>,
): GoogleFinanceQuoteResponse {
    return {
        quote: {
            summary: {
                name: result.name ?? null,
                symbol: result.symbol ?? params.symbol ?? null,
                exchange: result.exchange ?? params.exchange ?? null,
                current_price: result.current_price ?? null,
                price_change: result.price_change ?? null,
                percent_change: result.percent_change ?? null,
                currency: result.currency ?? null,
                country: result.country ?? null,
            },
            key_stats: {
                previous_close: result.previous_close ?? null,
            },
            about: {},
            financials: [],
            news: [],
            discover_more: [],
        },
    };
}

export async function fetchSearchQuoteFallback(
    client: ScrappaClient,
    params: Record<string, unknown>,
    attempts: number,
): Promise<QuoteFetchResult | null> {
    const searchParams: Record<string, unknown> = {
        q: params.symbol,
    };

    if (params.hl !== undefined) searchParams.hl = params.hl;
    if (params.gl !== undefined) searchParams.gl = params.gl;

    const response = await client.get<GoogleFinanceSearchResponse>('/google-finance/search', searchParams, {
        attempts,
    });
    const result = response.results?.find((item) => matchesRequestedQuote(item, params));

    if (!result) {
        return null;
    }

    return {
        response: buildQuoteResponseFromSearchResult(result, params),
        fallback: {
            reason: 'scrappa_quote_empty_search_result',
            omitted_params: [],
            primary_error: 'Scrappa quote response did not contain usable price, key stats, profile, financials, news, or related ticker data.',
            source_endpoint: '/google-finance/search',
            unavailable_sections: ['about', 'financials', 'news', 'discover_more'],
        },
    };
}

export async function fetchQuoteWithFallback(
    client: ScrappaClient,
    params: Record<string, unknown>,
    attempts: number,
): Promise<QuoteFetchResult> {
    try {
        const response = await client.get<GoogleFinanceQuoteResponse>('/google-finance/quote', params, { attempts });
        return { response };
    } catch (error) {
        if (!shouldRetryBaseQuote(error, params)) {
            throw error;
        }

        const fallbackParams = { ...params };
        delete fallbackParams.period_type;

        console.warn(
            `Scrappa quote request failed with ${error.message}. Retrying base quote without period_type so the actor can still return quote data.`,
        );

        const response = await client.get<GoogleFinanceQuoteResponse>('/google-finance/quote', fallbackParams, {
            attempts,
        });

        return {
            response,
            fallback: {
                reason: 'scrappa_5xx_after_financial_period_request',
                omitted_params: ['period_type'],
                primary_error: error.message,
            },
        };
    }
}

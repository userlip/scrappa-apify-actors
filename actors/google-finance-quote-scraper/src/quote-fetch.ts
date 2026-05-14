import type { GoogleFinanceQuoteResponse } from './response-utils.js';
import { ScrappaClient, ScrappaHttpError } from './shared/index.js';

export interface QuoteFetchResult {
    response: GoogleFinanceQuoteResponse;
    fallback?: {
        reason: string;
        omitted_params: string[];
        primary_error: string;
    };
}

export function shouldRetryBaseQuote(error: unknown, params: Record<string, unknown>): error is ScrappaHttpError {
    return error instanceof ScrappaHttpError
        && error.status >= 500
        && error.status <= 599
        && params.period_type !== undefined;
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

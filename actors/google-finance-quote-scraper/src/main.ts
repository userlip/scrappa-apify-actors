import { Actor } from 'apify';
import { buildGoogleFinanceQuoteParams, describeGoogleFinanceQuoteRequest } from './request-params.js';
import type { GoogleFinanceQuoteInput } from './request-params.js';
import { buildQuoteDatasetItem } from './response-utils.js';
import type { GoogleFinanceQuoteResponse } from './response-utils.js';
import { ScrappaClient, ScrappaHttpError, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;
const SCRAPPA_MAX_ATTEMPTS = 3;
const QUOTE_RESULT_CHARGE_EVENT = 'quote-result';

interface QuoteFetchResult {
    response: GoogleFinanceQuoteResponse;
    fallback?: {
        reason: string;
        omitted_params: string[];
        primary_error: string;
    };
}

function shouldRetryBaseQuote(error: unknown, params: Record<string, unknown>): error is ScrappaHttpError {
    return error instanceof ScrappaHttpError
        && error.status >= 500
        && error.status <= 599
        && params.period_type !== undefined;
}

async function fetchQuoteWithFallback(
    client: ScrappaClient,
    params: Record<string, unknown>,
): Promise<QuoteFetchResult> {
    try {
        const response = await client.get<GoogleFinanceQuoteResponse>('/google-finance/quote', params, {
            attempts: SCRAPPA_MAX_ATTEMPTS,
        });
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
            attempts: SCRAPPA_MAX_ATTEMPTS,
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

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<GoogleFinanceQuoteInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const params = buildGoogleFinanceQuoteParams(input);
        console.log(`Fetching Google Finance quote for ${describeGoogleFinanceQuoteRequest(params)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const fetchResult = await fetchQuoteWithFallback(client, params);
        const { response } = fetchResult;
        const item: Record<string, unknown> = {
            ...buildQuoteDatasetItem(response, params),
            upstream_fallback: fetchResult.fallback ?? null,
        };

        const { isPayPerEvent } = Actor.getChargingManager().getPricingInfo();
        if (isPayPerEvent) {
            const chargeResult = await Actor.pushData(item, QUOTE_RESULT_CHARGE_EVENT);
            if (chargeResult.eventChargeLimitReached && chargeResult.chargedCount < 1) {
                const statusMessage = 'Charge limit reached before saving the Google Finance quote result; OUTPUT was not written.';
                console.log(statusMessage, JSON.stringify({
                    event: QUOTE_RESULT_CHARGE_EVENT,
                    charged_count: chargeResult.chargedCount,
                }));
                await Actor.exit({ statusMessage });
                return;
            }
        } else {
            await Actor.pushData(item);
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', response);

        const counts = item.result_counts as Record<string, number>;
        console.log('Google Finance quote scraping completed successfully');
        console.log('Results summary:', JSON.stringify({
            symbol: item.symbol,
            exchange: item.exchange,
            financials: counts.financials,
            news: counts.news,
            related_tickers: counts.related_tickers,
            fallback: fetchResult.fallback?.reason ?? null,
        }));
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The Google Finance quote request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Provide an exchange code to reduce lookup latency, or run the request again.`
            : rawMessage;
        console.error('Actor failed: ' + message);
        await Actor.fail(message);
        return;
    }

    await Actor.exit();
}

main().catch((error) => {
    const message = error instanceof Error ? error.message : String(error);
    console.error('Actor failed: ' + message);
    process.exitCode = 1;
});

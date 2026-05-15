import { Actor } from 'apify';
import { fetchQuoteWithFallback, fetchSearchQuoteFallback } from './quote-fetch.js';
import { buildGoogleFinanceQuoteParams, describeGoogleFinanceQuoteRequest } from './request-params.js';
import type { GoogleFinanceQuoteInput } from './request-params.js';
import { buildQuoteDatasetItem, hasMeaningfulQuoteData } from './response-utils.js';
import { ScrappaClient, ScrappaHttpError, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 25000;
const SCRAPPA_MAX_ATTEMPTS = 3;
const QUOTE_RESULT_CHARGE_EVENT = 'quote-result';

function isScrappaUpstreamFailure(error: unknown): error is ScrappaHttpError {
    return error instanceof ScrappaHttpError && error.status >= 500 && error.status <= 599;
}

function describeUnknownError(error: unknown): string {
    if (error instanceof Error) {
        return error.message;
    }

    if (typeof error === 'string') {
        return error;
    }

    try {
        const json = JSON.stringify(error);
        if (json !== undefined) {
            const typeName = error === null ? 'null' : typeof error;
            return `Non-Error thrown (${typeName}): ${json}`;
        }
    } catch {
        // Fall back to String() below for circular or unserializable values.
    }

    return `Non-Error thrown (${typeof error}): ${String(error)}`;
}

async function exitWithoutQuoteResult(statusMessage: string): Promise<void> {
    console.warn(statusMessage);
    await Actor.exit({ statusMessage });
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
        let fetchResult = await fetchQuoteWithFallback(client, params, SCRAPPA_MAX_ATTEMPTS);
        let { response } = fetchResult;

        if (!hasMeaningfulQuoteData(response)) {
            console.warn(
                `Scrappa returned no usable Google Finance quote data for ${describeGoogleFinanceQuoteRequest(params)}; trying Google Finance search fallback.`,
            );

            const fallbackResult = await fetchSearchQuoteFallback(client, params, SCRAPPA_MAX_ATTEMPTS);
            if (!fallbackResult || !hasMeaningfulQuoteData(fallbackResult.response)) {
                await exitWithoutQuoteResult(
                    `Scrappa returned no usable Google Finance quote data for ${describeGoogleFinanceQuoteRequest(params)}; no dataset item was written or charged.`,
                );
                return;
            }

            fetchResult = fallbackResult;
            response = fallbackResult.response;
        }

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
        if (isScrappaUpstreamFailure(error)) {
            const statusMessage = `Scrappa upstream returned ${error.status} after retries; no Google Finance quote result was written or charged. Try the run again later.`;
            await exitWithoutQuoteResult(statusMessage);
            return;
        }

        if (error instanceof ScrappaTimeoutError) {
            const statusMessage = `${error.message}. No Google Finance quote result was written or charged. Try the run again later, or provide an exchange code if the symbol is ambiguous.`;
            await exitWithoutQuoteResult(statusMessage);
            return;
        }

        const rawMessage = describeUnknownError(error);
        console.error('Actor failed: ' + rawMessage);
        await Actor.fail(rawMessage);
        return;
    }

    await Actor.exit();
}

main().catch((error) => {
    const message = describeUnknownError(error);
    console.error('Actor failed: ' + message);
    process.exitCode = 1;
});

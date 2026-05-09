import { Actor } from 'apify';
import { buildGoogleFinanceQuoteParams, describeGoogleFinanceQuoteRequest } from './request-params.js';
import type { GoogleFinanceQuoteInput } from './request-params.js';
import { buildQuoteDatasetItem } from './response-utils.js';
import type { GoogleFinanceQuoteResponse } from './response-utils.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;
const SCRAPPA_MAX_ATTEMPTS = 3;
const QUOTE_RESULT_CHARGE_EVENT = 'quote-result';

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
        const response = await client.get<GoogleFinanceQuoteResponse>('/google-finance/quote', params, {
            attempts: SCRAPPA_MAX_ATTEMPTS,
        });
        const item = buildQuoteDatasetItem(response, params);

        const chargeResult = await Actor.pushData(item, QUOTE_RESULT_CHARGE_EVENT);
        if (chargeResult.eventChargeLimitReached) {
            const statusMessage = 'Charge limit reached after saving the Google Finance quote result; OUTPUT was not written.';
            console.log(statusMessage, JSON.stringify({
                event: QUOTE_RESULT_CHARGE_EVENT,
                charged_count: chargeResult.chargedCount,
            }));
            await Actor.exit({ statusMessage });
            return;
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

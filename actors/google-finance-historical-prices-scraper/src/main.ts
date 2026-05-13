import { Actor } from 'apify';
import { buildGoogleFinanceHistoricalPricesParams, describeGoogleFinanceHistoricalPricesRequest } from './request-params.js';
import type { GoogleFinanceHistoricalPricesInput } from './request-params.js';
import { buildHistoricalPriceDatasetItems } from './response-utils.js';
import type { GoogleFinanceHistoricalPricesResponse } from './response-utils.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;
const SCRAPPA_MAX_ATTEMPTS = 3;
const PRICE_POINT_CHARGE_EVENT = 'price-point';

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<GoogleFinanceHistoricalPricesInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const params = buildGoogleFinanceHistoricalPricesParams(input);
        console.log(`Fetching Google Finance historical prices for ${describeGoogleFinanceHistoricalPricesRequest(params)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const response = await client.get<GoogleFinanceHistoricalPricesResponse>('/google-finance/historical', params, {
            attempts: SCRAPPA_MAX_ATTEMPTS,
        });
        const datasetItems = buildHistoricalPriceDatasetItems(response, params);

        if (datasetItems.length > 0) {
            const { isPayPerEvent } = Actor.getChargingManager().getPricingInfo();
            if (isPayPerEvent) {
                const chargeResult = await Actor.pushData(datasetItems, PRICE_POINT_CHARGE_EVENT);
                if (chargeResult.eventChargeLimitReached && chargeResult.chargedCount < datasetItems.length) {
                    const statusMessage = 'Charge limit reached before saving all Google Finance historical price points.';
                    console.log(statusMessage, JSON.stringify({
                        event: PRICE_POINT_CHARGE_EVENT,
                        charged_count: chargeResult.chargedCount,
                        requested_count: datasetItems.length,
                    }));
                    await Actor.exit({ statusMessage });
                    return;
                }
            } else {
                await Actor.pushData(datasetItems);
            }
        } else {
            console.log('No Google Finance historical price points found for this request');
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', response);

        console.log('Google Finance historical prices scraping completed successfully');
        console.log('Results summary:', JSON.stringify({
            symbol: response.symbol ?? params.symbol ?? null,
            exchange: response.exchange ?? params.exchange ?? null,
            prices: datasetItems.length,
            range: params.range ?? null,
            start_date: params.start_date ?? null,
            end_date: params.end_date ?? null,
            interval: params.interval ?? null,
        }));
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The Google Finance historical prices request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Provide an exchange code, shorten the date range, or run the request again.`
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

import { Actor } from 'apify';
import { buildGoogleFinanceMarketsParams, describeGoogleFinanceMarketsRequest } from './request-params.js';
import type { GoogleFinanceMarketsInput } from './request-params.js';
import { buildMarketsDatasetItems, buildMarketsResultCounts } from './response-utils.js';
import type { GoogleFinanceMarketsResponse } from './response-utils.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;
const SCRAPPA_MAX_ATTEMPTS = 3;
const MARKET_ITEM_CHARGE_EVENT = 'market-item';

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<GoogleFinanceMarketsInput>() ?? {};
        const params = buildGoogleFinanceMarketsParams(input);
        console.log(`Fetching Google Finance markets data for ${describeGoogleFinanceMarketsRequest(params)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const response = await client.get<GoogleFinanceMarketsResponse>('/google-finance/markets', params, {
            attempts: SCRAPPA_MAX_ATTEMPTS,
        });
        const datasetItems = buildMarketsDatasetItems(response, params);

        if (datasetItems.length > 0) {
            const { isPayPerEvent } = Actor.getChargingManager().getPricingInfo();
            if (isPayPerEvent) {
                const chargeResult = await Actor.pushData(datasetItems, MARKET_ITEM_CHARGE_EVENT);
                if (chargeResult.eventChargeLimitReached && chargeResult.chargedCount < datasetItems.length) {
                    const statusMessage = 'Charge limit reached before saving all Google Finance market items; OUTPUT was not written.';
                    console.log(statusMessage, JSON.stringify({
                        event: MARKET_ITEM_CHARGE_EVENT,
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
            console.log('No Google Finance market items found for this request');
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', response);

        console.log('Google Finance markets scraping completed successfully');
        console.log('Results summary:', JSON.stringify({
            trend: params.trend ?? null,
            index_market: params.index_market ?? null,
            ...buildMarketsResultCounts(response),
        }));
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The Google Finance markets request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Narrow the request with trend/index_market, or run it again.`
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

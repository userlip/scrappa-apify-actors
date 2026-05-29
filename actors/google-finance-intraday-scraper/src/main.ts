import { Actor } from 'apify';
import { buildGoogleFinanceIntradayRequests, describeGoogleFinanceIntradayRequest } from './request-params.js';
import type { GoogleFinanceIntradayInput } from './request-params.js';
import { buildIntradayPricePointDatasetItems } from './response-utils.js';
import type { GoogleFinanceIntradayResponse } from './response-utils.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;
const SCRAPPA_MAX_ATTEMPTS = 3;
const INTRADAY_PRICE_POINT_CHARGE_EVENT = 'intraday-price-point';

function isNoDataError(error: unknown): boolean {
    return error instanceof Error && /Scrappa API error \(404\)/.test(error.message);
}

async function pushItems(datasetItems: Record<string, unknown>[]): Promise<boolean> {
    if (datasetItems.length === 0) {
        return true;
    }

    const { isPayPerEvent } = Actor.getChargingManager().getPricingInfo();
    if (!isPayPerEvent) {
        await Actor.pushData(datasetItems);
        return true;
    }

    const chargeResult = await Actor.pushData(datasetItems, INTRADAY_PRICE_POINT_CHARGE_EVENT);
    if (chargeResult.eventChargeLimitReached && chargeResult.chargedCount < datasetItems.length) {
        const statusMessage = 'Charge limit reached before saving all Google Finance intraday price points; remaining symbols were not processed.';
        console.log(statusMessage, JSON.stringify({
            event: INTRADAY_PRICE_POINT_CHARGE_EVENT,
            charged_count: chargeResult.chargedCount,
            requested_count: datasetItems.length,
        }));
        await Actor.exit({ statusMessage });
        return false;
    }

    return true;
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<GoogleFinanceIntradayInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const requests = buildGoogleFinanceIntradayRequests(input);
        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const summary = {
            requested: requests.length,
            succeeded: 0,
            no_data: 0,
            failed: 0,
            graph_points: 0,
        };

        console.log(`Fetching Google Finance intraday data for ${requests.length} symbol${requests.length === 1 ? '' : 's'}`);

        for (const params of requests) {
            console.log(`Fetching Google Finance intraday data for ${describeGoogleFinanceIntradayRequest(params)}`);

            try {
                const response = await client.get<GoogleFinanceIntradayResponse>('/google-finance/intraday', params, {
                    attempts: SCRAPPA_MAX_ATTEMPTS,
                });
                const datasetItems = buildIntradayPricePointDatasetItems(response, params);

                if (datasetItems.length === 0) {
                    console.log(`No Google Finance intraday graph points found for ${describeGoogleFinanceIntradayRequest(params)}`);
                    summary.no_data += 1;
                    continue;
                }

                const pushed = await pushItems(datasetItems);
                if (!pushed) {
                    return;
                }

                summary.succeeded += 1;
                summary.graph_points += datasetItems.length;
            } catch (error) {
                if (isNoDataError(error)) {
                    console.log(`No Google Finance intraday graph points found for ${describeGoogleFinanceIntradayRequest(params)}`);
                    summary.no_data += 1;
                    continue;
                }

                throw error;
            }
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', summary);

        console.log('Google Finance intraday scraping completed successfully');
        console.log('Results summary:', JSON.stringify(summary));
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The Google Finance intraday request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Provide exchange codes, reduce the symbol batch, or run the request again.`
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

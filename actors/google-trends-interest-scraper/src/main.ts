import { Actor } from 'apify';
import { buildGoogleTrendsInterestParams, describeGoogleTrendsInterestRequest } from './request-params.js';
import type { GoogleTrendsInterestInput } from './request-params.js';
import { buildTimelineDatasetItems } from './response-utils.js';
import type { GoogleTrendsInterestResponse } from './response-utils.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;
const SCRAPPA_MAX_ATTEMPTS = 3;
const TIMELINE_POINT_CHARGE_EVENT = 'timeline-point';

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<GoogleTrendsInterestInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const params = buildGoogleTrendsInterestParams(input);
        console.log(`Fetching Google Trends interest over time for ${describeGoogleTrendsInterestRequest(params)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const response = await client.get<GoogleTrendsInterestResponse>('/google-trends/interest', params, {
            attempts: SCRAPPA_MAX_ATTEMPTS,
        });
        const datasetItems = buildTimelineDatasetItems(response, params);

        if (datasetItems.length > 0) {
            const { isPayPerEvent } = Actor.getChargingManager().getPricingInfo();
            if (isPayPerEvent) {
                const chargeResult = await Actor.pushData(datasetItems, TIMELINE_POINT_CHARGE_EVENT);
                if (chargeResult.eventChargeLimitReached && chargeResult.chargedCount < datasetItems.length) {
                    const statusMessage = 'Charge limit reached before saving all Google Trends timeline points.';
                    console.log(statusMessage, JSON.stringify({
                        event: TIMELINE_POINT_CHARGE_EVENT,
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
            console.log('No Google Trends timeline points found for this request');
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', response);

        console.log('Google Trends interest scraping completed successfully');
        console.log('Results summary:', JSON.stringify({
            timeline_points: datasetItems.length,
            average: response.interest_over_time?.average ?? null,
            max_value: response.interest_over_time?.max_value ?? null,
            min_value: response.interest_over_time?.min_value ?? null,
            response_time_ms: response.response_time_ms ?? null,
        }));
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The Google Trends interest request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try a shorter time range, a more specific keyword, or run the request again.`
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

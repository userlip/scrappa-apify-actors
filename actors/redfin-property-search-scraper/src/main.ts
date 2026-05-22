import { Actor } from 'apify';
import { buildRedfinPropertySearchRequests, describeRedfinPropertySearchRequest } from './request-params.js';
import type { RedfinPropertySearchInput } from './request-params.js';
import { buildRedfinDatasetItem, getRedfinPropertyListings, getRedfinSearchCount } from './response-utils.js';
import type { RedfinPropertySearchResponse } from './response-utils.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 90000;
const SCRAPPA_MAX_ATTEMPTS = 3;
const REDFIN_PROPERTY_RESULT_CHARGE_EVENT = 'property-result';

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<RedfinPropertySearchInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const requests = buildRedfinPropertySearchRequests(input);
        console.log(`Running ${requests.length} Redfin property search request(s)`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        let totalResults = 0;

        for (const request of requests) {
            console.log(`Searching Redfin for ${describeRedfinPropertySearchRequest(request.params)}`);

            const response = await client.get<RedfinPropertySearchResponse>('/redfin/search', request.params, {
                attempts: SCRAPPA_MAX_ATTEMPTS,
            });
            const properties = getRedfinPropertyListings(response)
                .map((property) => buildRedfinDatasetItem(property, request.params, request.index));

            if (properties.length > 0) {
                const { isPayPerEvent } = Actor.getChargingManager().getPricingInfo();
                if (isPayPerEvent) {
                    const chargeResult = await Actor.pushData(properties, REDFIN_PROPERTY_RESULT_CHARGE_EVENT);
                    if (chargeResult.eventChargeLimitReached) {
                        const statusMessage = `Charge limit reached after saving ${chargeResult.chargedCount} of ${properties.length} Redfin property result(s) for search ${request.index + 1}.`;
                        console.log(statusMessage, JSON.stringify({
                            event: REDFIN_PROPERTY_RESULT_CHARGE_EVENT,
                            charged_count: chargeResult.chargedCount,
                            requested_count: properties.length,
                            search_index: request.index,
                        }));
                        await Actor.exit({ statusMessage });
                        return;
                    }
                } else {
                    await Actor.pushData(properties);
                }
            }

            totalResults += properties.length;
            console.log(`Search ${request.index + 1} returned ${properties.length} Redfin property result(s)`, JSON.stringify({
                search_index: request.index,
                api_count: getRedfinSearchCount(response),
                request_region_id: request.params.region_id,
                request_market: request.params.market,
            }));
        }

        console.log('Redfin property search completed successfully');
        console.log('Results summary:', JSON.stringify({
            searches: requests.length,
            results: totalResults,
        }));
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The Redfin request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try fewer homes, fewer batched searches, or run the request again.`
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

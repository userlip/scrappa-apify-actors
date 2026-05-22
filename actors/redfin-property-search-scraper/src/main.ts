import { Actor } from 'apify';
import { buildRedfinPropertySearchRequests, describeRedfinPropertySearchRequest } from './request-params.js';
import type { RedfinPropertySearchInput } from './request-params.js';
import { buildRedfinDatasetItem, getRedfinPropertyListings, getRedfinSearchCount } from './response-utils.js';
import type { RedfinPropertySearchResponse } from './response-utils.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 90000;
const SCRAPPA_MAX_ATTEMPTS = 3;
const REDFIN_PROPERTY_RESULT_CHARGE_EVENT = 'property-result';

interface PushChargedPropertiesResult {
    savedCount: number;
    statusMessage: string | null;
}

function getChargeLimitStatus(totalResults: number, searchIndex: number): string | null {
    const chargingManager = Actor.getChargingManager();
    const { isPayPerEvent } = chargingManager.getPricingInfo();
    if (!isPayPerEvent) {
        return null;
    }

    if (chargingManager.calculateMaxEventChargeCountWithinLimit(REDFIN_PROPERTY_RESULT_CHARGE_EVENT) > 0) {
        return null;
    }

    return `Charge limit reached before fetching Redfin search ${searchIndex + 1}; ${totalResults} property result(s) were saved.`;
}

async function pushChargedProperties(
    properties: Record<string, unknown>[],
    searchIndex: number,
): Promise<PushChargedPropertiesResult> {
    const { isPayPerEvent } = Actor.getChargingManager().getPricingInfo();
    if (!isPayPerEvent) {
        await Actor.pushData(properties);
        return { savedCount: properties.length, statusMessage: null };
    }

    let savedCount = 0;
    for (const property of properties) {
        const chargeResult = await Actor.pushData(property, REDFIN_PROPERTY_RESULT_CHARGE_EVENT);
        if (chargeResult.chargedCount >= 1) {
            savedCount += 1;
        }

        if (chargeResult.eventChargeLimitReached) {
            const statusMessage = chargeResult.chargedCount >= 1
                ? `Charge limit reached after saving ${savedCount} of ${properties.length} Redfin property result(s) for search ${searchIndex + 1}.`
                : `Charge limit reached before saving the next Redfin property result for search ${searchIndex + 1}.`;
            console.log(statusMessage, JSON.stringify({
                event: REDFIN_PROPERTY_RESULT_CHARGE_EVENT,
                charged_count: chargeResult.chargedCount,
                saved_count: savedCount,
                requested_count: properties.length,
                search_index: searchIndex,
            }));
            return { savedCount, statusMessage };
        }
    }

    return { savedCount, statusMessage: null };
}

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
        let statusMessage: string | null = null;

        for (const request of requests) {
            statusMessage = getChargeLimitStatus(totalResults, request.index);
            if (statusMessage) {
                console.log(statusMessage, JSON.stringify({
                    event: REDFIN_PROPERTY_RESULT_CHARGE_EVENT,
                    searches_requested: requests.length,
                    results: totalResults,
                    next_search_index: request.index,
                }));
                break;
            }

            console.log(`Searching Redfin for ${describeRedfinPropertySearchRequest(request.params)}`);

            const response = await client.get<RedfinPropertySearchResponse>('/redfin/search', request.params, {
                attempts: SCRAPPA_MAX_ATTEMPTS,
            });
            const properties = getRedfinPropertyListings(response)
                .map((property) => buildRedfinDatasetItem(property, request.params, request.index));

            if (properties.length > 0) {
                const pushResult = await pushChargedProperties(properties, request.index);
                totalResults += pushResult.savedCount;
                if (pushResult.statusMessage) {
                    statusMessage = pushResult.statusMessage;
                    break;
                }
            } else {
                console.log(`Search ${request.index + 1} returned no Redfin property results`, JSON.stringify({
                    search_index: request.index,
                    api_count: getRedfinSearchCount(response),
                    request_region_id: request.params.region_id,
                    request_market: request.params.market,
                }));
                continue;
            }

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
            status_message: statusMessage,
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

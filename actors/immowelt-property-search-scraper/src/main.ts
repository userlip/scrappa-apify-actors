import { Actor } from 'apify';
import {
    buildImmoweltPropertySearchParams,
    describeImmoweltPropertySearchRequest,
    normalizeImmoweltPropertySearchInput,
} from './request-params.js';
import type { ImmoweltPropertySearchInput } from './request-params.js';
import {
    buildImmoweltDatasetItem,
    getImmoweltPage,
    getImmoweltPropertyListings,
    getImmoweltTotalPages,
    getImmoweltTotalResults,
} from './response-utils.js';
import type { ImmoweltPropertySearchResponse } from './response-utils.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;
const SCRAPPA_MAX_ATTEMPTS = 3;
const PROPERTY_RESULT_CHARGE_EVENT = 'property-result';

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = normalizeImmoweltPropertySearchInput(await Actor.getInput<ImmoweltPropertySearchInput>());
        const params = buildImmoweltPropertySearchParams(input);
        console.log(`Searching Immowelt for ${describeImmoweltPropertySearchRequest(params)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const response = await client.get<ImmoweltPropertySearchResponse>('/immowelt/search', params, {
            attempts: SCRAPPA_MAX_ATTEMPTS,
        });
        const rawListings = getImmoweltPropertyListings(response);
        const requestedLimit = params.limit as number;
        const listings = rawListings
            .slice(0, requestedLimit)
            .map((listing) => buildImmoweltDatasetItem(listing, params));

        if (listings.length > 0) {
            const { isPayPerEvent } = Actor.getChargingManager().getPricingInfo();
            if (isPayPerEvent) {
                const chargeResult = await Actor.pushData(listings, PROPERTY_RESULT_CHARGE_EVENT);
                if (chargeResult.eventChargeLimitReached && chargeResult.chargedCount < listings.length) {
                    const statusMessage = `Charge limit reached after saving ${chargeResult.chargedCount} of ${listings.length} Immowelt property result(s); OUTPUT was not written.`;
                    console.log(statusMessage, JSON.stringify({
                        event: PROPERTY_RESULT_CHARGE_EVENT,
                        charged_count: chargeResult.chargedCount,
                        requested_count: listings.length,
                    }));
                    await Actor.exit({ statusMessage });
                    return;
                }
            } else {
                await Actor.pushData(listings);
            }

            console.log(`Found ${listings.length} Immowelt property result(s)`);
            if (rawListings.length > listings.length) {
                console.log(`Scrappa returned ${rawListings.length} result(s); saved the requested limit of ${listings.length}.`);
            }
        } else {
            console.log('No Immowelt property results found for this request');
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', response);

        console.log('Immowelt property search completed successfully');
        console.log('Results summary:', JSON.stringify({
            listings: listings.length,
            total_results: getImmoweltTotalResults(response),
            page: getImmoweltPage(response),
            total_pages: getImmoweltTotalPages(response),
            request_location: params.location,
            request_property_type: params.property_type,
        }));
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The Immowelt request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try a smaller page size or run the request again.`
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

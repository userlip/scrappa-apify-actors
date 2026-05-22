import { Actor } from 'apify';
import {
    buildImmobilienscout24SearchParams,
    describeImmobilienscout24SearchRequest,
    normalizeImmobilienscout24SearchInput,
} from './request-params.js';
import type { Immobilienscout24SearchInput } from './request-params.js';
import {
    buildImmobilienscout24DatasetItem,
    getImmobilienscout24Listings,
    getImmobilienscout24Page,
    getImmobilienscout24TotalPages,
    getImmobilienscout24TotalResults,
    limitImmobilienscout24SearchResponse,
} from './response-utils.js';
import type { Immobilienscout24SearchResponse } from './response-utils.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 90000;
const SCRAPPA_MAX_ATTEMPTS = 3;
const PROPERTY_RESULT_CHARGE_EVENT = 'property-result';

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = normalizeImmobilienscout24SearchInput(await Actor.getInput<Immobilienscout24SearchInput>());
        const params = buildImmobilienscout24SearchParams(input);
        console.log(`Searching ImmobilienScout24 for ${describeImmobilienscout24SearchRequest(params)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const response = await client.get<Immobilienscout24SearchResponse>('/immobilienscout24/search', params, {
            attempts: SCRAPPA_MAX_ATTEMPTS,
        });
        const rawListings = getImmobilienscout24Listings(response);
        const requestedLimit = params.per_page as number;
        const listings = rawListings
            .slice(0, requestedLimit)
            .map((listing) => buildImmobilienscout24DatasetItem(listing, params));

        if (listings.length > 0) {
            const { isPayPerEvent } = Actor.getChargingManager().getPricingInfo();
            if (isPayPerEvent) {
                const chargeResult = await Actor.pushData(listings, PROPERTY_RESULT_CHARGE_EVENT);
                if (chargeResult.eventChargeLimitReached && chargeResult.chargedCount < listings.length) {
                    const statusMessage = `Charge limit reached after saving ${chargeResult.chargedCount} of ${listings.length} ImmobilienScout24 property result(s); OUTPUT was not written.`;
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

            console.log(`Found ${listings.length} ImmobilienScout24 property result(s)`);
            if (rawListings.length > listings.length) {
                console.log(`Scrappa returned ${rawListings.length} result(s); saved the requested limit of ${listings.length}.`);
            }
        } else {
            console.log('No ImmobilienScout24 property results found for this request');
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', limitImmobilienscout24SearchResponse(response, listings.length));

        console.log('ImmobilienScout24 property search completed successfully');
        console.log('Results summary:', JSON.stringify({
            listings: listings.length,
            total_results: getImmobilienscout24TotalResults(response),
            page: getImmobilienscout24Page(response),
            total_pages: getImmobilienscout24TotalPages(response),
            request_location: params.location,
            request_type: params.type,
        }));
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The ImmobilienScout24 request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try a smaller page size or run the request again.`
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

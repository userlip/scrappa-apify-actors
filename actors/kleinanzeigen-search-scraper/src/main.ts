import { Actor } from 'apify';
import {
    buildKleinanzeigenSearchPlan,
    describeKleinanzeigenSearchRequest,
} from './request-params.js';
import type { KleinanzeigenSearchInput } from './request-params.js';
import {
    buildKleinanzeigenDatasetItem,
    getKleinanzeigenListings,
    limitKleinanzeigenSearchResponse,
} from './response-utils.js';
import type { KleinanzeigenSearchResponse } from './response-utils.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 90000;
const SCRAPPA_MAX_ATTEMPTS = 3;
const LISTING_RESULT_CHARGE_EVENT = 'listing-result';

interface PushChargedListingsResult {
    savedCount: number;
    statusMessage: string | null;
}

async function pushChargedListings(
    listings: Record<string, unknown>[],
    params: Record<string, unknown>,
): Promise<PushChargedListingsResult> {
    if (listings.length === 0) {
        return { savedCount: 0, statusMessage: null };
    }

    const { isPayPerEvent } = Actor.getChargingManager().getPricingInfo();
    if (!isPayPerEvent) {
        await Actor.pushData(listings);
        return { savedCount: listings.length, statusMessage: null };
    }

    const chargeResult = await Actor.pushData(listings, LISTING_RESULT_CHARGE_EVENT);
    if (chargeResult.eventChargeLimitReached) {
        const savedCount = Math.min(chargeResult.chargedCount, listings.length);
        const statusMessage = `Charge limit reached after saving ${savedCount} of ${listings.length} Kleinanzeigen listing result(s) for query ${String(params.query)}.`;
        console.log(statusMessage, JSON.stringify({
            event: LISTING_RESULT_CHARGE_EVENT,
            charged_count: chargeResult.chargedCount,
            requested_count: listings.length,
            saved_count: savedCount,
            query: params.query,
            page: params.page,
        }));
        return { savedCount, statusMessage };
    }

    return { savedCount: listings.length, statusMessage: null };
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<KleinanzeigenSearchInput>() ?? {};
        const plan = buildKleinanzeigenSearchPlan(input);
        console.log(`Searching Kleinanzeigen for ${describeKleinanzeigenSearchRequest(plan)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const responses: Array<{
            index: number;
            request: Record<string, unknown>;
            listings_saved: number;
            response: KleinanzeigenSearchResponse;
        }> = [];
        let savedListings = 0;
        let statusMessage: string | null = null;

        for (const search of plan.searches) {
            const params = search.params;
            console.log(`Fetching Kleinanzeigen query ${String(params.query)} on page ${String(params.page)}`);

            const response = await client.get<KleinanzeigenSearchResponse>('/kleinanzeigen/search', params, {
                attempts: SCRAPPA_MAX_ATTEMPTS,
            });

            const listings = getKleinanzeigenListings(response).map((listing) => (
                buildKleinanzeigenDatasetItem(listing, params, response)
            ));

            const result = await pushChargedListings(listings, params);
            savedListings += result.savedCount;
            responses.push({
                index: search.index,
                request: params,
                listings_saved: result.savedCount,
                response: limitKleinanzeigenSearchResponse(response, result.savedCount),
            });

            console.log(`Found ${listings.length} listing(s); saved ${result.savedCount}`);

            if (result.statusMessage) {
                statusMessage = result.statusMessage;
                break;
            }
        }

        const output = {
            searches_requested: plan.searches.length,
            searches_completed: responses.length,
            listings_extracted: savedListings,
            status_message: statusMessage,
            responses,
        };

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', output);

        console.log('Kleinanzeigen search completed successfully');
        console.log('Results summary:', JSON.stringify({
            searches_requested: output.searches_requested,
            searches_completed: output.searches_completed,
            listings_extracted: output.listings_extracted,
        }));

        if (statusMessage) {
            await Actor.exit({ statusMessage });
            return;
        }
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The Kleinanzeigen search request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try fewer searches, narrower filters, or run the request again.`
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

import { Actor } from 'apify';
import { buildGoogleHotelsSearchParams, describeGoogleHotelsSearchRequest } from './request-params.js';
import type { GoogleHotelsSearchInput } from './request-params.js';
import { buildHotelDatasetItem, getHotelProperties } from './response-utils.js';
import type { GoogleHotelsSearchResponse } from './response-utils.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 90000;
const SCRAPPA_MAX_ATTEMPTS = 3;
const HOTEL_RESULT_CHARGE_EVENT = 'hotel-result';

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<GoogleHotelsSearchInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const params = buildGoogleHotelsSearchParams(input);
        console.log(`Searching Google Hotels for ${describeGoogleHotelsSearchRequest(params)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const response = await client.get<GoogleHotelsSearchResponse>('/google-hotels/search', params, {
            attempts: SCRAPPA_MAX_ATTEMPTS,
        });
        const hotels = getHotelProperties(response).map((hotel) => buildHotelDatasetItem(hotel, params));

        if (hotels.length > 0) {
            const { isPayPerEvent } = Actor.getChargingManager().getPricingInfo();
            if (isPayPerEvent) {
                const chargeResult = await Actor.pushData(hotels, HOTEL_RESULT_CHARGE_EVENT);
                if (chargeResult.eventChargeLimitReached && chargeResult.chargedCount < hotels.length) {
                    const statusMessage = `Charge limit reached after saving ${chargeResult.chargedCount} of ${hotels.length} Google Hotels results; OUTPUT was not written.`;
                    console.log(statusMessage, JSON.stringify({
                        event: HOTEL_RESULT_CHARGE_EVENT,
                        charged_count: chargeResult.chargedCount,
                        requested_count: hotels.length,
                    }));
                    await Actor.exit({ statusMessage });
                    return;
                }
            } else {
                await Actor.pushData(hotels);
            }

            console.log(`Found ${hotels.length} hotel result(s)`);
        } else {
            console.log('No Google Hotels results found for this request');
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', response);

        console.log('Google Hotels search completed successfully');
        console.log('Results summary:', JSON.stringify({
            hotels: hotels.length,
            brands: Array.isArray(response.brands) ? response.brands.length : 0,
            has_next_page: Boolean(response.pagination?.next_page_token || response.pagination?.next),
            response_time_ms: response.response_time_ms ?? null,
        }));
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The Google Hotels request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try a narrower destination, fewer filters, or run the request again.`
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

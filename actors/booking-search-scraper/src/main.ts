import { Actor } from 'apify';
import { buildBookingSearchRequests, describeBookingSearchRequest } from './request-params.js';
import type { BookingSearchInput } from './request-params.js';
import { buildBookingDatasetItem, getBookingSearchResults } from './response-utils.js';
import type { BookingSearchResponse } from './response-utils.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 90000;
const SCRAPPA_MAX_ATTEMPTS = 3;
const BOOKING_RESULT_CHARGE_EVENT = 'booking-result';

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<BookingSearchInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const requests = buildBookingSearchRequests(input);
        console.log(`Running ${requests.length} Booking.com search request(s)`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        let totalResults = 0;

        for (const request of requests) {
            console.log(`Searching Booking.com for ${describeBookingSearchRequest(request.params)}`);

            const response = await client.get<BookingSearchResponse>('/booking/search', request.params, {
                attempts: SCRAPPA_MAX_ATTEMPTS,
            });
            const properties = getBookingSearchResults(response)
                .map((property) => buildBookingDatasetItem(property, request.params, request.index));

            if (properties.length > 0) {
                const { isPayPerEvent } = Actor.getChargingManager().getPricingInfo();
                if (isPayPerEvent) {
                    const chargeResult = await Actor.pushData(properties, BOOKING_RESULT_CHARGE_EVENT);
                    if (chargeResult.eventChargeLimitReached && chargeResult.chargedCount < properties.length) {
                        const statusMessage = `Charge limit reached after saving ${chargeResult.chargedCount} of ${properties.length} Booking.com results for search ${request.index + 1}.`;
                        console.log(statusMessage, JSON.stringify({
                            event: BOOKING_RESULT_CHARGE_EVENT,
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
            console.log(`Search ${request.index + 1} returned ${properties.length} Booking.com result(s)`);
        }

        console.log('Booking.com search completed successfully');
        console.log('Results summary:', JSON.stringify({
            searches: requests.length,
            results: totalResults,
        }));
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The Booking.com request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try a more specific destination, include check-in/check-out dates, or run the request again.`
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

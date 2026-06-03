import { Actor } from 'apify';
import {
    BOOKING_HOTEL_RESULT_CHARGE_EVENT,
    getHotelChargeLimitStatus,
    pushErrorHotelItem,
    pushSuccessfulHotelItem,
} from './charging.js';
import { buildBookingHotelRequests, describeBookingHotelRequest } from './request-params.js';
import type { BookingHotelInput } from './request-params.js';
import { buildBookingHotelDatasetItem, buildBookingHotelErrorItem, getBookingHotelDetails } from './response-utils.js';
import type { BookingHotelResponse } from './response-utils.js';
import { ScrappaClient, ScrappaNetworkError, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 90000;
const SCRAPPA_MAX_ATTEMPTS = 3;

function isActorLevelScrappaFailure(error: unknown): boolean {
    if (error instanceof ScrappaTimeoutError || error instanceof ScrappaNetworkError) {
        return true;
    }

    if (!(error instanceof Error)) {
        return false;
    }

    return /Scrappa API error \((?:401|403|408|429|500|502|503|504)\)/.test(error.message);
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<BookingHotelInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const requests = buildBookingHotelRequests(input);
        console.log(`Running ${requests.length} Booking.com hotel detail request(s)`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        let savedResults = 0;
        let failedRequests = 0;

        for (const request of requests) {
            const chargeLimitStatus = getHotelChargeLimitStatus(Actor, savedResults, request.index);
            if (chargeLimitStatus) {
                console.log(chargeLimitStatus, JSON.stringify({
                    event: BOOKING_HOTEL_RESULT_CHARGE_EVENT,
                    hotels_requested: requests.length,
                    results_saved: savedResults,
                    next_request_index: request.index,
                }));
                await Actor.exit({ statusMessage: chargeLimitStatus });
                return;
            }

            console.log(`Fetching Booking.com hotel details for ${describeBookingHotelRequest(request)}`);

            try {
                const response = await client.get<BookingHotelResponse>('/booking/hotel', request.params, {
                    attempts: SCRAPPA_MAX_ATTEMPTS,
                });
                const item = buildBookingHotelDatasetItem(getBookingHotelDetails(response), request);
                const pushResult = await pushSuccessfulHotelItem(Actor, item, request.index);

                if (pushResult.saved) {
                    savedResults += 1;
                    console.log(`Saved hotel detail result ${request.index + 1}`);
                }

                if (pushResult.statusMessage) {
                    console.log(pushResult.statusMessage, JSON.stringify({
                        event: BOOKING_HOTEL_RESULT_CHARGE_EVENT,
                        charged_count: pushResult.chargedCount,
                        requested_count: requests.length,
                        request_index: request.index,
                    }));
                    await Actor.exit({ statusMessage: pushResult.statusMessage });
                    return;
                }
            } catch (error) {
                if (isActorLevelScrappaFailure(error)) {
                    throw error;
                }

                failedRequests += 1;
                const rawMessage = error instanceof Error ? error.message : String(error);
                const message = error instanceof ScrappaTimeoutError
                    ? `${rawMessage}. The Booking.com hotel detail request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try the Booking.com URL form or run the request again.`
                    : rawMessage;

                console.warn(`Hotel detail request ${request.index + 1} failed: ${message}`);
                await pushErrorHotelItem(Actor, buildBookingHotelErrorItem(request, new Error(message)));
            }
        }

        console.log('Booking.com hotel details completed');
        console.log('Results summary:', JSON.stringify({
            hotels_requested: requests.length,
            results_saved: savedResults,
            failed_requests: failedRequests,
        }));
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The Booking.com hotel detail request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try the Booking.com URL form or run the request again.`
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

import { Actor } from 'apify';
import { buildGoogleFlightsRequest, describeGoogleFlightsRequest } from './request-params.js';
import type { GoogleFlightsSearchInput } from './request-params.js';
import { buildFlightDatasetItems, getFlights } from './response-utils.js';
import type { GoogleFlightsSearchResponse } from './response-utils.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 90000;
const SCRAPPA_MAX_ATTEMPTS = 3;
const FLIGHT_RESULT_CHARGE_EVENT = 'flight-result';

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<GoogleFlightsSearchInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const request = buildGoogleFlightsRequest(input);
        console.log(`Searching Google Flights for ${describeGoogleFlightsRequest(request)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const response = await client.get<GoogleFlightsSearchResponse>(request.endpoint, request.params, {
            attempts: SCRAPPA_MAX_ATTEMPTS,
        });

        const items = buildFlightDatasetItems(response, request.params, request.tripType);
        const flights = getFlights(response);

        if (items.length > 0) {
            const { isPayPerEvent } = Actor.getChargingManager().getPricingInfo();
            if (isPayPerEvent) {
                const chargeResult = await Actor.pushData(items, FLIGHT_RESULT_CHARGE_EVENT);
                if (chargeResult.eventChargeLimitReached && chargeResult.chargedCount < items.length) {
                    const statusMessage = `Charge limit reached after saving ${chargeResult.chargedCount}/${items.length} Google Flights result(s).`;
                    console.log(statusMessage, JSON.stringify({
                        event: FLIGHT_RESULT_CHARGE_EVENT,
                        charged_count: chargeResult.chargedCount,
                        result_count: items.length,
                    }));
                    await Actor.exit({ statusMessage });
                    return;
                }
            } else {
                await Actor.pushData(items);
            }
        } else {
            console.log('No flight results found for the given search criteria');
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', response);

        console.log('Google Flights search completed successfully');
        console.log('Results summary:', JSON.stringify({
            trip_type: request.tripType,
            origin: request.params.origin,
            destination: request.params.destination,
            departure_date: request.params.departure_date,
            return_date: request.params.return_date ?? null,
            flights: flights.length,
            has_baggage_info: response.baggage_info !== undefined,
        }));
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The Google Flights request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try a narrower route/date filter or run the request again.`
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

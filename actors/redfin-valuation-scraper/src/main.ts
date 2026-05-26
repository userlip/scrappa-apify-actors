import { Actor } from 'apify';
import { actorChargingApi, getChargeLimitStatus, pushChargedValuation, REDFIN_VALUATION_RESULT_CHARGE_EVENT } from './charging.js';
import { buildRedfinValuationRequests, describeRedfinValuationRequest } from './request-params.js';
import type { RedfinValuationInput } from './request-params.js';
import {
    buildRedfinValuationDatasetItem,
    buildRedfinValuationFailureItem,
    getRedfinValuationData,
    hasMeaningfulValuationData,
} from './response-utils.js';
import type { RedfinValuationResponse } from './response-utils.js';
import { ScrappaClient, ScrappaHttpError, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;
const SCRAPPA_MAX_ATTEMPTS = 3;

function isRecoverableAvailabilityError(error: unknown): error is ScrappaHttpError {
    return error instanceof ScrappaHttpError && [404, 422].includes(error.status);
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<RedfinValuationInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const requests = buildRedfinValuationRequests(input);
        console.log(`Running ${requests.length} Redfin valuation request(s)`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const failures: Record<string, unknown>[] = [];
        const successfulItems: Record<string, unknown>[] = [];
        const savedItems: Record<string, unknown>[] = [];
        let statusMessage: string | null = null;

        for (const request of requests) {
            statusMessage = getChargeLimitStatus(actorChargingApi, savedItems.length, request.index);
            if (statusMessage) {
                console.log(statusMessage, JSON.stringify({
                    event: REDFIN_VALUATION_RESULT_CHARGE_EVENT,
                    valuations_requested: requests.length,
                    results: savedItems.length,
                    next_request_index: request.index,
                }));
                break;
            }

            console.log(`Fetching Redfin valuation for ${describeRedfinValuationRequest(request)}`);

            try {
                const response = await client.get<RedfinValuationResponse>('/redfin/valuation', request.params, {
                    attempts: SCRAPPA_MAX_ATTEMPTS,
                });
                const data = getRedfinValuationData(response);

                if (!hasMeaningfulValuationData(data)) {
                    const failure = {
                        status: 'unavailable',
                        message: 'Scrappa returned no usable Redfin valuation fields.',
                    };
                    const item = buildRedfinValuationFailureItem(request, failure);
                    failures.push(item);
                    const pushResult = await pushChargedValuation(actorChargingApi, item);
                    if (pushResult.saved) {
                        savedItems.push(item);
                    }
                    if (pushResult.statusMessage) {
                        statusMessage = pushResult.statusMessage;
                        break;
                    }
                    console.warn(`No usable Redfin valuation fields for property ${request.property_id}`);
                    continue;
                }

                const item = buildRedfinValuationDatasetItem(response, request);
                const pushResult = await pushChargedValuation(actorChargingApi, item);
                if (pushResult.saved) {
                    successfulItems.push(item);
                    savedItems.push(item);
                }
                if (pushResult.statusMessage) {
                    statusMessage = pushResult.statusMessage;
                    break;
                }
            } catch (error) {
                if (isRecoverableAvailabilityError(error)) {
                    const failure = {
                        status: error.status,
                        message: error.details,
                    };
                    const item = buildRedfinValuationFailureItem(request, failure);
                    failures.push(item);
                    const pushResult = await pushChargedValuation(actorChargingApi, item);
                    if (pushResult.saved) {
                        savedItems.push(item);
                    }
                    if (pushResult.statusMessage) {
                        statusMessage = pushResult.statusMessage;
                        break;
                    }
                    console.warn(`Redfin valuation unavailable for property ${request.property_id}: ${error.details}`);
                    continue;
                }

                throw error;
            }
        }

        const store = await Actor.openKeyValueStore();
        if (requests.length === 1 && successfulItems.length === 1 && failures.length === 0) {
            await store.setValue('OUTPUT', successfulItems[0]);
        } else {
            await store.setValue('OUTPUT', {
                requested: requests.length,
                saved: successfulItems.length,
                dataset_items: savedItems.length,
                failed: failures.length,
                failures,
                status_message: statusMessage,
            });
        }

        console.log(statusMessage
            ? `Redfin valuation scraping completed: ${statusMessage}`
            : 'Redfin valuation scraping completed successfully');
        console.log('Results summary:', JSON.stringify({
            requested: requests.length,
            saved: successfulItems.length,
            dataset_items: savedItems.length,
            failed: failures.length,
            status_message: statusMessage,
        }));
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The Redfin valuation request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try fewer batched properties or run the request again.`
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

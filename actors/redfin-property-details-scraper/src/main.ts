import { Actor } from 'apify';
import {
    actorChargingApi,
    getChargeLimitStatus,
    pushChargedProperty,
    REDFIN_PROPERTY_DETAILS_RESULT_CHARGE_EVENT,
} from './charging.js';
import { isPerPropertyScrappaHttpError } from './http-errors.js';
import { buildRedfinPropertyDetailsRequests, describeRedfinPropertyDetailsRequest } from './request-params.js';
import type { RedfinPropertyDetailsInput } from './request-params.js';
import {
    buildRedfinPropertyDetailsDatasetItem,
    buildRedfinPropertyErrorDatasetItem,
    getRedfinPropertyDetails,
} from './response-utils.js';
import type { RedfinPropertyDetailsResponse } from './response-utils.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 90000;
const SCRAPPA_MAX_ATTEMPTS = 3;

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<RedfinPropertyDetailsInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const requests = buildRedfinPropertyDetailsRequests(input);
        console.log(`Fetching ${requests.length} Redfin property detail request(s)`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        let totalResults = 0;
        let totalErrors = 0;
        let statusMessage: string | null = null;
        let singleOutputItem: Record<string, unknown> | null = null;

        for (const request of requests) {
            statusMessage = getChargeLimitStatus(actorChargingApi, totalResults, request.index);
            if (statusMessage) {
                console.log(statusMessage, JSON.stringify({
                    event: REDFIN_PROPERTY_DETAILS_RESULT_CHARGE_EVENT,
                    properties_requested: requests.length,
                    results: totalResults,
                    next_property_index: request.index,
                }));
                break;
            }

            console.log(`Fetching Redfin details for ${describeRedfinPropertyDetailsRequest(request)}`);

            try {
                const response = await client.get<RedfinPropertyDetailsResponse>('/redfin/property', request.params, {
                    attempts: SCRAPPA_MAX_ATTEMPTS,
                });
                const property = getRedfinPropertyDetails(response);

                if (!property) {
                    totalErrors += 1;
                    const item = buildRedfinPropertyErrorDatasetItem(request, 'Scrappa returned no property details', null);
                    await Actor.pushData(item);
                    if (requests.length === 1) {
                        singleOutputItem = item;
                    }
                    console.log(`No Redfin property details found for property_id ${request.params.property_id}`);
                    continue;
                }

                const item = buildRedfinPropertyDetailsDatasetItem(property, request);
                const pushResult = await pushChargedProperty(actorChargingApi, item, request.index);
                if (pushResult.saved) {
                    totalResults += 1;
                    if (requests.length === 1) {
                        singleOutputItem = item;
                    }
                }
                if (pushResult.statusMessage) {
                    statusMessage = pushResult.statusMessage;
                    break;
                }
            } catch (error) {
                if (isPerPropertyScrappaHttpError(error)) {
                    totalErrors += 1;
                    const item = buildRedfinPropertyErrorDatasetItem(request, error.message, error.status);
                    await Actor.pushData(item);
                    if (requests.length === 1) {
                        singleOutputItem = item;
                    }
                    console.log(`Redfin property error for property_id ${request.params.property_id}: ${error.message}`);
                    continue;
                }

                throw error;
            }
        }

        const store = await Actor.openKeyValueStore();
        if (requests.length === 1 && singleOutputItem !== null) {
            await store.setValue('OUTPUT', singleOutputItem);
        } else {
            await store.setValue('OUTPUT', {
                properties_requested: requests.length,
                results: totalResults,
                errors: totalErrors,
                status_message: statusMessage,
            });
        }

        console.log(statusMessage
            ? `Redfin property details completed: ${statusMessage}`
            : 'Redfin property details completed successfully');
        console.log('Results summary:', JSON.stringify({
            properties_requested: requests.length,
            results: totalResults,
            errors: totalErrors,
            status_message: statusMessage,
        }));
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The Redfin property details request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try fewer batched properties or run the request again.`
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

import { Actor } from 'apify';
import {
    getVintedItemDetailChargeLimitStatus,
    pushSuccessfulVintedItemDetail,
    pushVintedItemDetailError,
    VINTED_ITEM_DETAIL_RESULT_CHARGE_EVENT,
} from './charging.js';
import { isActorLevelScrappaFailure } from './failures.js';
import {
    buildVintedItemDetailsRequests,
    describeVintedItemDetailsRequest,
} from './request-params.js';
import type { VintedItemDetailsInput } from './request-params.js';
import {
    buildVintedItemDetailsDatasetItem,
    buildVintedItemDetailsErrorItem,
    getVintedItemDetails,
} from './response-utils.js';
import type { VintedItemDetailsResponse } from './response-utils.js';
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

        const input = await Actor.getInput<VintedItemDetailsInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const requests = buildVintedItemDetailsRequests(input);
        console.log(`Running ${requests.length} Vinted item detail request(s)`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        let savedResults = 0;
        let failedRequests = 0;

        for (const request of requests) {
            const chargeLimitStatus = getVintedItemDetailChargeLimitStatus(Actor, savedResults, request.index);
            if (chargeLimitStatus) {
                console.log(chargeLimitStatus, JSON.stringify({
                    event: VINTED_ITEM_DETAIL_RESULT_CHARGE_EVENT,
                    items_requested: requests.length,
                    results_saved: savedResults,
                    next_request_index: request.index,
                }));
                await Actor.exit({ statusMessage: chargeLimitStatus });
                return;
            }

            console.log(`Fetching Vinted item details for ${describeVintedItemDetailsRequest(request)}`);

            try {
                const response = await client.get<VintedItemDetailsResponse>('/vinted/item-details', request.params, {
                    attempts: SCRAPPA_MAX_ATTEMPTS,
                });
                const item = buildVintedItemDetailsDatasetItem(getVintedItemDetails(response), request);
                const pushResult = await pushSuccessfulVintedItemDetail(Actor, item, request.index);

                if (pushResult.saved) {
                    savedResults += 1;
                    console.log(`Saved Vinted item detail result ${request.index + 1}`);
                }

                if (pushResult.statusMessage) {
                    console.log(pushResult.statusMessage, JSON.stringify({
                        event: VINTED_ITEM_DETAIL_RESULT_CHARGE_EVENT,
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
                    ? `${rawMessage}. The Vinted item detail request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try a smaller batch or run the request again.`
                    : rawMessage;

                console.warn(`Vinted item detail request ${request.index + 1} failed: ${message}`);
                await pushVintedItemDetailError(Actor, buildVintedItemDetailsErrorItem(request, new Error(message)));
            }
        }

        const output = {
            requested: requests.length,
            succeeded: savedResults,
            failed: failedRequests,
            country: requests[0]?.params.country ?? null,
        };

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', output);

        console.log('Vinted item details completed');
        console.log('Results summary:', JSON.stringify(output));
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The Vinted item detail request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try a smaller batch or run the request again.`
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

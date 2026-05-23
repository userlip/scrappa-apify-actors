import { Actor } from 'apify';
import { getPinterestChargedSaveResult } from './charging-utils.js';
import {
    buildPinterestSearchPlan,
    capPinterestSearchParamsToChargeCapacity,
    describePinterestSearchRequest,
} from './request-params.js';
import type { PinterestSearchInput } from './request-params.js';
import {
    buildPinterestDatasetItem,
    getPinterestNextBookmark,
    limitPinterestSearchResponse,
    selectPinterestPins,
} from './response-utils.js';
import type { PinterestSearchResponse } from './response-utils.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 90000;
const SCRAPPA_MAX_ATTEMPTS = 3;
const PIN_RESULT_CHARGE_EVENT = 'pin-result';

interface PushChargedItemsResult {
    savedCount: number;
    statusMessage: string | null;
}

function getChargeablePinCapacity(): number {
    const chargingManager = Actor.getChargingManager();
    const { isPayPerEvent } = chargingManager.getPricingInfo();

    if (!isPayPerEvent) {
        return Infinity;
    }

    return chargingManager.calculateMaxEventChargeCountWithinLimit(PIN_RESULT_CHARGE_EVENT);
}

async function pushChargedPins(items: Record<string, unknown>[], query: string): Promise<PushChargedItemsResult> {
    if (items.length === 0) {
        return { savedCount: 0, statusMessage: null };
    }

    const { isPayPerEvent } = Actor.getChargingManager().getPricingInfo();
    if (!isPayPerEvent) {
        await Actor.pushData(items);
        return { savedCount: items.length, statusMessage: null };
    }

    const chargeResult = await Actor.pushData(items, PIN_RESULT_CHARGE_EVENT);
    const { savedCount, chargeLimitReached } = getPinterestChargedSaveResult(chargeResult, items.length);
    if (chargeLimitReached) {
        const statusMessage = `Charge limit reached after saving ${savedCount} of ${items.length} Pinterest pin result(s) for "${query}".`;
        console.log(statusMessage, JSON.stringify({
            event: PIN_RESULT_CHARGE_EVENT,
            charged_count: chargeResult.chargedCount,
            requested_count: items.length,
            saved_count: savedCount,
            query,
        }));
        return { savedCount, statusMessage };
    }

    return { savedCount, statusMessage: null };
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<PinterestSearchInput>() ?? {};
        const plan = buildPinterestSearchPlan(input);
        console.log(`Searching Pinterest for ${describePinterestSearchRequest(plan)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const responses: PinterestSearchResponse[] = [];
        const querySummaries: Record<string, unknown>[] = [];
        let searchesFetched = 0;
        let extractedPins = 0;
        let savedPins = 0;
        let statusMessage: string | null = null;

        for (const request of plan.requests) {
            const chargeablePinCapacity = getChargeablePinCapacity();
            if (chargeablePinCapacity <= 0) {
                statusMessage = `Charge limit reached before fetching Pinterest pins for "${request.query}".`;
                console.log(statusMessage, JSON.stringify({
                    event: PIN_RESULT_CHARGE_EVENT,
                    query: request.query,
                }));
                break;
            }

            const fetchParams = capPinterestSearchParamsToChargeCapacity(
                request.params,
                plan.limit,
                chargeablePinCapacity,
            );
            const { fetchLimit, requestedLimit } = fetchParams;
            const upstreamParams = fetchParams.params;

            console.log(`Fetching Pinterest pins for "${request.query}" with limit ${String(fetchLimit)}`);

            const response = await client.get<PinterestSearchResponse>('/pinterest/search', upstreamParams, {
                attempts: SCRAPPA_MAX_ATTEMPTS,
            });
            searchesFetched += 1;

            const selection = selectPinterestPins(response);
            const pins = selection.pins;
            extractedPins += pins.length;
            const items = pins.map((pin) => buildPinterestDatasetItem(pin, upstreamParams, response));
            const result = await pushChargedPins(items, request.query);
            savedPins += result.savedCount;
            responses.push(limitPinterestSearchResponse(response, result.savedCount, selection.source));

            querySummaries.push({
                query: request.query,
                requested_limit: requestedLimit,
                fetch_limit: fetchLimit,
                request_bookmark: upstreamParams.bookmark ?? null,
                count: response.count ?? null,
                results_count: response.results_count ?? pins.length,
                pins_extracted: pins.length,
                pins_saved: result.savedCount,
                nextBookmark: getPinterestNextBookmark(response),
            });

            console.log(`Found ${pins.length} Pinterest pin result(s) for "${request.query}"; saved ${result.savedCount}`);
            if (result.statusMessage) {
                statusMessage = result.statusMessage;
                break;
            }
        }

        const output = {
            request: {
                queries: plan.requests.map((request) => request.query),
                limit: plan.limit,
                bookmark: plan.bookmark ?? null,
            },
            searches_fetched: searchesFetched,
            responses_saved: responses.length,
            pins_extracted: extractedPins,
            pins_saved: savedPins,
            status_message: statusMessage,
            query_summaries: querySummaries,
            responses,
        };

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', output);

        console.log('Pinterest search completed successfully');
        console.log('Results summary:', JSON.stringify({
            searches_fetched: searchesFetched,
            responses_saved: responses.length,
            pins_extracted: extractedPins,
            pins_saved: savedPins,
            queries: plan.requests.length,
        }));

        if (statusMessage) {
            await Actor.exit({ statusMessage });
            return;
        }
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The Pinterest search request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try fewer queries, a lower limit, or run the request again.`
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

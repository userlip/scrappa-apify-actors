import { Actor } from 'apify';
import {
    buildPageParams,
    buildVintedSearchPlan,
    describeVintedSearchRequest,
} from './request-params.js';
import type { VintedSearchInput } from './request-params.js';
import {
    buildVintedDatasetItem,
    getVintedItems,
    getVintedPagination,
} from './response-utils.js';
import type { VintedSearchResponse } from './response-utils.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 90000;
const SCRAPPA_MAX_ATTEMPTS = 3;
const ITEM_RESULT_CHARGE_EVENT = 'item-result';

interface PushChargedItemsResult {
    savedCount: number;
    statusMessage: string | null;
}

async function pushChargedItems(items: Record<string, unknown>[], page: number): Promise<PushChargedItemsResult> {
    if (items.length === 0) {
        return { savedCount: 0, statusMessage: null };
    }

    const { isPayPerEvent } = Actor.getChargingManager().getPricingInfo();
    if (!isPayPerEvent) {
        await Actor.pushData(items);
        return { savedCount: items.length, statusMessage: null };
    }

    const chargeResult = await Actor.pushData(items, ITEM_RESULT_CHARGE_EVENT);
    if (chargeResult.eventChargeLimitReached) {
        const savedCount = Math.min(chargeResult.chargedCount, items.length);
        const statusMessage = `Charge limit reached after saving ${savedCount} of ${items.length} Vinted result(s) on page ${page}.`;
        console.log(statusMessage, JSON.stringify({
            event: ITEM_RESULT_CHARGE_EVENT,
            charged_count: chargeResult.chargedCount,
            requested_count: items.length,
            saved_count: savedCount,
            page,
        }));
        return { savedCount, statusMessage };
    }

    return { savedCount: items.length, statusMessage: null };
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<VintedSearchInput>() ?? {};

        const plan = buildVintedSearchPlan(input);
        console.log(`Searching Vinted for ${describeVintedSearchRequest(plan)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const responses: VintedSearchResponse[] = [];
        let pagesFetched = 0;
        let savedItems = 0;
        let statusMessage: string | null = null;
        let latestPagination = undefined as ReturnType<typeof getVintedPagination>;

        for (let offset = 0; offset < plan.maxPages; offset += 1) {
            const page = plan.startPage + offset;
            const params = buildPageParams(plan, page);
            console.log(`Fetching Vinted page ${page} in ${String(params.country)} for query ${String(params.query ?? 'none')}`);

            const response = await client.get<VintedSearchResponse>('/vinted/search', params, {
                attempts: SCRAPPA_MAX_ATTEMPTS,
            });
            pagesFetched += 1;
            latestPagination = getVintedPagination(response);

            const items = getVintedItems(response).map((item) => buildVintedDatasetItem(item, params, response));
            responses.push(response);

            if (items.length === 0) {
                console.log(`No Vinted listings found on page ${page}`);
                break;
            }

            const result = await pushChargedItems(items, page);
            savedItems += result.savedCount;
            console.log(`Found ${items.length} listing(s) on page ${page}; saved ${result.savedCount}`);
            if (result.statusMessage) {
                statusMessage = result.statusMessage;
                break;
            }

            const hasNextPage = latestPagination?.has_next_page;
            const totalPages = latestPagination?.total_pages;
            if (hasNextPage === false || (typeof totalPages === 'number' && page >= totalPages)) {
                console.log(`Stopping after page ${page}; Scrappa reported no additional Vinted pages`);
                break;
            }
        }

        const output = {
            request: {
                ...plan.baseParams,
                start_page: plan.startPage,
                max_pages: plan.maxPages,
            },
            pages_fetched: pagesFetched,
            responses_saved: responses.length,
            items_extracted: savedItems,
            status_message: statusMessage,
            total_pages: latestPagination?.total_pages ?? null,
            total_entries: latestPagination?.total_entries ?? latestPagination?.total_items ?? null,
            responses,
        };

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', output);

        console.log('Vinted search completed successfully');
        console.log('Results summary:', JSON.stringify({
            pages_fetched: pagesFetched,
            responses_saved: responses.length,
            items_extracted: savedItems,
            total_pages: output.total_pages,
            total_entries: output.total_entries,
        }));

        if (statusMessage) {
            await Actor.exit({ statusMessage });
            return;
        }
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The Vinted search request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try fewer pages, a narrower filter, or run the request again.`
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

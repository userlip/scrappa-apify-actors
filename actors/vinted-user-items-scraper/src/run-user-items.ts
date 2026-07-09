import {
    buildUserPageParams,
    type VintedUserItemsPlan,
} from './request-params.js';
import {
    buildVintedDatasetItem,
    getVintedItems,
    getVintedPagination,
    type VintedUserItemsResponse,
} from './response-utils.js';

export interface VintedUserItemsClient {
    get<T>(endpoint: string, params: Record<string, unknown>, options?: { attempts?: number }): Promise<T>;
}

export interface VintedUserItemsDataset {
    pushData(items: Record<string, unknown>[], eventName?: string): Promise<{
        chargedCount?: number;
        eventChargeLimitReached?: boolean;
    } | void>;
}

export interface VintedUserItemsRunOptions {
    client: VintedUserItemsClient;
    dataset: VintedUserItemsDataset;
    plan: VintedUserItemsPlan;
    isPayPerEvent: boolean;
    chargeEventName: string;
    attempts: number;
}

export interface VintedUserItemsRunSummary {
    pagesFetched: number;
    savedItems: number;
    statusMessage: string | null;
}

async function pushChargedItems(
    dataset: VintedUserItemsDataset,
    items: Record<string, unknown>[],
    userId: string,
    page: number,
    isPayPerEvent: boolean,
    chargeEventName: string,
): Promise<{ savedCount: number; statusMessage: string | null }> {
    if (items.length === 0) {
        return { savedCount: 0, statusMessage: null };
    }

    if (!isPayPerEvent) {
        await dataset.pushData(items);
        return { savedCount: items.length, statusMessage: null };
    }

    const chargeResult = await dataset.pushData(items, chargeEventName);
    if (chargeResult?.eventChargeLimitReached) {
        const chargedCount = chargeResult.chargedCount ?? 0;
        const savedCount = Math.min(chargedCount, items.length);
        const statusMessage = `Charge limit reached after saving ${savedCount} of ${items.length} Vinted item(s) for user ${userId} on page ${page}.`;
        console.log(statusMessage, JSON.stringify({
            event: chargeEventName,
            charged_count: chargedCount,
            requested_count: items.length,
            saved_count: savedCount,
            user_id: userId,
            page,
        }));
        return { savedCount, statusMessage };
    }

    return { savedCount: items.length, statusMessage: null };
}

export async function runVintedUserItems(options: VintedUserItemsRunOptions): Promise<VintedUserItemsRunSummary> {
    const {
        client,
        dataset,
        plan,
        isPayPerEvent,
        chargeEventName,
        attempts,
    } = options;
    let pagesFetched = 0;
    let savedItems = 0;
    let statusMessage: string | null = null;

    for (const userId of plan.userIds) {
        for (let offset = 0; offset < plan.maxPages; offset += 1) {
            const page = plan.startPage + offset;
            const params = buildUserPageParams(plan, userId, page);
            console.log(`Fetching Vinted seller ${userId} page ${page} in ${String(params.country)}`);

            const response = await client.get<VintedUserItemsResponse>('/vinted/user-items', params, { attempts });
            pagesFetched += 1;

            const pagination = getVintedPagination(response);
            const items = getVintedItems(response).map((item) => buildVintedDatasetItem(item, params, response));

            if (items.length === 0) {
                console.log(`No Vinted listings found for seller ${userId} on page ${page}`);
                break;
            }

            const result = await pushChargedItems(dataset, items, userId, page, isPayPerEvent, chargeEventName);
            savedItems += result.savedCount;
            console.log(`Found ${items.length} listing(s) for seller ${userId} on page ${page}; saved ${result.savedCount}`);
            if (result.statusMessage) {
                statusMessage = result.statusMessage;
                break;
            }

            const hasNextPage = pagination?.has_next_page;
            const totalPages = pagination?.total_pages;
            if (hasNextPage === false || (typeof totalPages === 'number' && page >= totalPages)) {
                console.log(`Stopping seller ${userId} after page ${page}; Scrappa reported no additional Vinted pages`);
                break;
            }
        }

        if (statusMessage) {
            break;
        }
    }

    return { pagesFetched, savedItems, statusMessage };
}

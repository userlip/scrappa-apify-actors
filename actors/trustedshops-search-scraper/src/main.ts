import { Actor } from 'apify';
import {
    buildPageParams,
    buildTrustedShopsSearchPlan,
    describeTrustedShopsSearchRequest,
} from './request-params.js';
import type { TrustedShopsSearchInput } from './request-params.js';
import {
    buildTrustedShopsDatasetItem,
    getTrustedShops,
} from './response-utils.js';
import type { TrustedShopsSearchResponse } from './response-utils.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 90000;
const SCRAPPA_MAX_ATTEMPTS = 3;
const SHOP_RESULT_CHARGE_EVENT = 'shop-result';

async function pushChargedItems(items: Record<string, unknown>[]): Promise<boolean> {
    if (items.length === 0) {
        return true;
    }

    const { isPayPerEvent } = Actor.getChargingManager().getPricingInfo();
    if (!isPayPerEvent) {
        await Actor.pushData(items);
        return true;
    }

    const chargeResult = await Actor.pushData(items, SHOP_RESULT_CHARGE_EVENT);
    if (chargeResult.eventChargeLimitReached && chargeResult.chargedCount < items.length) {
        const statusMessage = `Charge limit reached after saving ${chargeResult.chargedCount} of ${items.length} Trusted Shops results on the current page; OUTPUT was not written.`;
        console.log(statusMessage, JSON.stringify({
            event: SHOP_RESULT_CHARGE_EVENT,
            charged_count: chargeResult.chargedCount,
            requested_count: items.length,
        }));
        await Actor.exit({ statusMessage });
        return false;
    }

    return true;
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<TrustedShopsSearchInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const plan = buildTrustedShopsSearchPlan(input);
        console.log(`Searching Trusted Shops for ${describeTrustedShopsSearchRequest(plan)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const responses: TrustedShopsSearchResponse[] = [];
        let savedShops = 0;

        for (let offset = 0; offset < plan.maxPages; offset += 1) {
            const page = plan.startPage + offset;
            const params = buildPageParams(plan, page);
            console.log(`Fetching Trusted Shops page ${page} for ${String(params.q)} in ${String(params.market)}`);

            const response = await client.get<TrustedShopsSearchResponse>('/trustedshops/search', params, {
                attempts: SCRAPPA_MAX_ATTEMPTS,
            });
            responses.push(response);

            const shops = getTrustedShops(response).map((shop) => buildTrustedShopsDatasetItem(shop, params, response));
            if (shops.length > 0) {
                const saved = await pushChargedItems(shops);
                if (!saved) {
                    return;
                }

                savedShops += shops.length;
                console.log(`Found ${shops.length} shop result(s) on page ${page}`);
            } else {
                console.log(`No Trusted Shops results found on page ${page}`);
                break;
            }

            const totalPageCount = response.metaData?.totalPageCount;
            if (typeof totalPageCount === 'number' && page + 1 >= totalPageCount) {
                console.log(`Stopping after page ${page}; Scrappa reported ${totalPageCount} total page(s)`);
                break;
            }
        }

        const lastResponse = responses[responses.length - 1];
        const output = {
            request: {
                ...plan.baseParams,
                start_page: plan.startPage,
                max_pages: plan.maxPages,
            },
            pages_fetched: responses.length,
            shops_extracted: savedShops,
            total_shop_count: lastResponse?.metaData?.totalShopCount ?? null,
            total_page_count: lastResponse?.metaData?.totalPageCount ?? null,
            responses,
        };

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', output);

        console.log('Trusted Shops search completed successfully');
        console.log('Results summary:', JSON.stringify({
            pages_fetched: responses.length,
            shops_extracted: savedShops,
            total_shop_count: output.total_shop_count,
            total_page_count: output.total_page_count,
        }));
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The Trusted Shops search request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try fewer pages or run the request again.`
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

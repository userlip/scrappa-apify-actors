import { Actor } from 'apify';
import { pushChargedItems } from './charging.js';
import {
    buildPageParams,
    buildTrustpilotBusinessSearchPlan,
    describeTrustpilotBusinessSearchRequest,
} from './request-params.js';
import type { TrustpilotBusinessSearchInput } from './request-params.js';
import {
    buildTrustpilotBusinessDatasetItem,
    getTrustpilotBusinesses,
    hasNextPage,
} from './response-utils.js';
import type { TrustpilotBusinessSearchResponse } from './response-utils.js';
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

        const input = await Actor.getInput<TrustpilotBusinessSearchInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const plan = buildTrustpilotBusinessSearchPlan(input);
        console.log(`Searching Trustpilot businesses for ${describeTrustpilotBusinessSearchRequest(plan)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const responses: TrustpilotBusinessSearchResponse[] = [];
        let pagesFetched = 0;
        let savedBusinesses = 0;
        let statusMessage: string | null = null;

        for (let offset = 0; offset < plan.maxPages; offset += 1) {
            const page = plan.startPage + offset;
            const params = buildPageParams(plan, page);
            console.log(`Fetching Trustpilot ${plan.searchType} page ${page}`);

            const response = await client.get<TrustpilotBusinessSearchResponse>(plan.endpoint, params, {
                attempts: SCRAPPA_MAX_ATTEMPTS,
            });
            pagesFetched += 1;
            responses.push(response);

            const businesses = getTrustpilotBusinesses(response).map((business) => buildTrustpilotBusinessDatasetItem(business, {
                searchType: plan.searchType,
                params,
                response,
            }));
            if (businesses.length > 0) {
                const result = await pushChargedItems({
                    isPayPerEvent: () => Actor.getChargingManager().getPricingInfo().isPayPerEvent,
                    pushData: (items, eventName) => eventName === undefined
                        ? Actor.pushData(items)
                        : Actor.pushData(items, eventName),
                }, businesses);
                savedBusinesses += result.savedCount;
                console.log(`Found ${businesses.length} business result(s) on page ${page}; saved ${result.savedCount}`);
                if (result.statusMessage) {
                    statusMessage = result.statusMessage;
                    break;
                }
            } else {
                console.log(`No Trustpilot business results found on page ${page}`);
                break;
            }

            if (!hasNextPage(response, page)) {
                console.log(`Stopping after page ${page}; Scrappa reported no next page`);
                break;
            }
        }

        const lastResponse = responses[responses.length - 1];
        const output = {
            request: {
                search_type: plan.searchType,
                endpoint: plan.endpoint,
                ...plan.baseParams,
                start_page: plan.startPage,
                max_pages: plan.maxPages,
            },
            pages_fetched: pagesFetched,
            responses_saved: responses.length,
            businesses_extracted: savedBusinesses,
            status_message: statusMessage,
            total_results: lastResponse?.pagination?.totalResults
                ?? lastResponse?.pageProps?.pagination?.total_count
                ?? lastResponse?.pageProps?.businessUnits?.totalHits
                ?? null,
            total_pages: lastResponse?.pagination?.totalPages
                ?? lastResponse?.pageProps?.pagination?.total_pages
                ?? lastResponse?.pageProps?.businessUnits?.totalPages
                ?? null,
            responses,
        };

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', output);

        console.log('Trustpilot business search completed successfully');
        console.log('Results summary:', JSON.stringify({
            search_type: plan.searchType,
            pages_fetched: pagesFetched,
            businesses_extracted: savedBusinesses,
            total_results: output.total_results,
            total_pages: output.total_pages,
        }));

        if (statusMessage) {
            await Actor.exit({ statusMessage });
            return;
        }
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The Trustpilot business search request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try fewer pages or run the request again.`
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

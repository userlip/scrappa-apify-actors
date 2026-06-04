import { Actor } from 'apify';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';
import {
    buildLinkedInSearchParams,
    limitLinkedInSearchResultCount,
    normalizeLinkedInSearchInput,
    validateLinkedInSearchInput,
} from './search-params.js';
import type { LinkedInSearchInput } from './search-params.js';
import { getLinkedInSearchResults } from './search-response.js';
import type { LinkedInSearchResponse } from './search-response.js';
import {
    actorChargingApi,
    getLinkedInSearchChargeLimitStatus,
    getRemainingLinkedInSearchResultCharges,
    pushLinkedInSearchResult,
} from './charging.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;
const SCRAPPA_MAX_ATTEMPTS = 3;

await Actor.init();

try {
    await main();
} catch (error) {
    const rawMessage = error instanceof Error ? error.message : String(error);
    const message = error instanceof ScrappaTimeoutError
        ? `${rawMessage}. The LinkedIn Search request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try again or refine the query.`
        : rawMessage;
    console.error('Actor failed: ' + message);
    await Actor.fail(message);
}

async function main(): Promise<void> {
    const apiKey = process.env.SCRAPPA_API_KEY;
    if (!apiKey) {
        throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
    }

    const input = normalizeLinkedInSearchInput(await Actor.getInput<LinkedInSearchInput>());
    validateLinkedInSearchInput(input);

    const remainingCharges = getRemainingLinkedInSearchResultCharges(actorChargingApi);
    if (remainingCharges === 0) {
        const statusMessage = 'Charge limit reached before calling Scrappa; no LinkedIn search results were requested.';
        console.log(statusMessage);
        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', {
            results: 0,
            total_results: null,
            current_page: null,
            pages: 0,
            search_information: null,
            pagination: null,
            status: statusMessage,
        });
        await Actor.exit({ statusMessage });
        return;
    }

    const requestInput = limitLinkedInSearchResultCount(input, remainingCharges);

    console.log(`Searching LinkedIn for: "${input.query}"`);

    const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
    const response = await client.get<LinkedInSearchResponse>(
        '/linkedin/search',
        buildLinkedInSearchParams(requestInput),
        { attempts: SCRAPPA_MAX_ATTEMPTS }
    );
    const results = getLinkedInSearchResults(response);
    let savedResults = 0;
    let statusMessage: string | null = null;

    if (results.length > 0) {
        for (const result of results) {
            statusMessage = getLinkedInSearchChargeLimitStatus(actorChargingApi, savedResults, results.length);
            if (statusMessage) {
                console.log(statusMessage);
                break;
            }

            const pushResult = await pushLinkedInSearchResult(actorChargingApi, result);
            if (!pushResult.saved) {
                statusMessage = pushResult.statusMessage;
                break;
            }
            savedResults++;
        }

        console.log(`Saved ${savedResults} LinkedIn search result(s)`);
    } else {
        console.log('No LinkedIn search results found for the given search criteria');
    }

    const store = await Actor.openKeyValueStore();
    await store.setValue('OUTPUT', {
        results: savedResults,
        total_results: response.total_results ?? response.search_information?.total_results ?? null,
        current_page: response.pagination?.current_page ?? null,
        pages: Array.isArray(response.pagination?.pages) ? response.pagination.pages.length : 0,
        search_information: response.search_information ?? null,
        pagination: response.pagination ?? null,
    });

    console.log('LinkedIn search completed successfully');

    const summary = {
        results: savedResults,
        total_results: response.total_results ?? response.search_information?.total_results ?? null,
        current_page: response.pagination?.current_page ?? null,
        pages: Array.isArray(response.pagination?.pages) ? response.pagination.pages.length : 0,
    };

    console.log('Results summary:', JSON.stringify(summary));
    if (statusMessage) {
        await Actor.exit({ statusMessage });
    } else {
        await Actor.exit();
    }
}

import { Actor } from 'apify';
import { buildStartpageSearchPlan, describeStartpageSearchPlan } from './request-params.js';
import type { StartpageSearchInput } from './request-params.js';
import { buildStartpageDatasetItem, extractStartpageOrganicResults } from './response-utils.js';
import type { StartpageSearchResponse } from './response-utils.js';
import { ScrappaClient } from './shared/scrappa-client.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;

interface PushResultItemsResult {
    savedCount: number;
    statusMessage: string | null;
}

async function pushResultItems(items: Record<string, unknown>[]): Promise<PushResultItemsResult> {
    if (items.length === 0) {
        return { savedCount: 0, statusMessage: null };
    }

    await Actor.pushData(items);
    return { savedCount: items.length, statusMessage: null };
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<StartpageSearchInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const plan = buildStartpageSearchPlan(input);
        console.log(`Fetching Startpage search results for ${describeStartpageSearchPlan(plan)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        let fetchedQueries = 0;
        let extractedResults = 0;
        let savedResults = 0;
        let statusMessage: string | null = null;

        for (const request of plan.requests) {
            console.log(`Fetching Startpage results for "${request.query}"`);
            const response = await client.get<StartpageSearchResponse>('/startpage/search', request.params);
            fetchedQueries += 1;

            const organicResults = extractStartpageOrganicResults(response);
            const cappedResults = organicResults.slice(0, plan.maxResultsPerQuery);
            const datasetItems = cappedResults.map((result) => buildStartpageDatasetItem(result, request.params, response));
            extractedResults += organicResults.length;

            const pushResult = await pushResultItems(datasetItems);
            savedResults += pushResult.savedCount;

            console.log(`Found ${organicResults.length} Startpage result(s) for "${request.query}"; saved ${pushResult.savedCount}`);
            if (pushResult.statusMessage) {
                statusMessage = pushResult.statusMessage;
                break;
            }
        }

        console.log('Startpage search completed successfully');
        console.log('Results summary:', JSON.stringify({
            queries_requested: plan.requests.length,
            queries_fetched: fetchedQueries,
            results_extracted: extractedResults,
            results_saved: savedResults,
            max_results_per_query: plan.maxResultsPerQuery,
        }));

        if (statusMessage) {
            await Actor.exit({ statusMessage });
            return;
        }
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = rawMessage.includes('timed out')
            ? `${rawMessage}. The Startpage request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try fewer queries or run the request again.`
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

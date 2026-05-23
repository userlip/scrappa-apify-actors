import { Actor } from 'apify';
import { buildGoogleFinanceSearchRequests, describeGoogleFinanceSearchRequest } from './request-params.js';
import type { GoogleFinanceSearchInput } from './request-params.js';
import { buildSearchDatasetItems, countSearchResults } from './response-utils.js';
import type { GoogleFinanceSearchResponse } from './response-utils.js';
import { ScrappaClient, ScrappaHttpError, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 30000;
const SCRAPPA_MAX_ATTEMPTS = 3;
const FINANCE_SEARCH_RESULT_CHARGE_EVENT = 'finance-search-result';

function isScrappaUpstreamFailure(error: unknown): error is ScrappaHttpError {
    return error instanceof ScrappaHttpError && error.status >= 500 && error.status <= 599;
}

function describeUnknownError(error: unknown): string {
    if (error instanceof Error) {
        return error.message;
    }

    if (typeof error === 'string') {
        return error;
    }

    try {
        const json = JSON.stringify(error);
        if (json !== undefined) {
            const typeName = error === null ? 'null' : typeof error;
            return `Non-Error thrown (${typeName}): ${json}`;
        }
    } catch {
        // Fall back to String() below for circular or unserializable values.
    }

    return `Non-Error thrown (${typeof error}): ${String(error)}`;
}

async function pushSearchItems(items: Record<string, unknown>[]): Promise<boolean> {
    if (items.length === 0) {
        return true;
    }

    const { isPayPerEvent } = Actor.getChargingManager().getPricingInfo();
    if (!isPayPerEvent) {
        await Actor.pushData(items);
        return true;
    }

    const chargeResult = await Actor.pushData(items, FINANCE_SEARCH_RESULT_CHARGE_EVENT);
    if (chargeResult.eventChargeLimitReached && chargeResult.chargedCount < items.length) {
        const statusMessage = 'Charge limit reached before saving all Google Finance search results.';
        console.log(statusMessage, JSON.stringify({
            event: FINANCE_SEARCH_RESULT_CHARGE_EVENT,
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

        const input = await Actor.getInput<GoogleFinanceSearchInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const requests = buildGoogleFinanceSearchRequests(input);
        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        let totalResults = 0;
        let zeroResultQueries = 0;

        for (const params of requests) {
            console.log(`Searching Google Finance for ${describeGoogleFinanceSearchRequest(params)}`);

            const response = await client.get<GoogleFinanceSearchResponse>('/google-finance/search', params, {
                attempts: SCRAPPA_MAX_ATTEMPTS,
            });
            const datasetItems = buildSearchDatasetItems(response, params);
            const resultCount = countSearchResults(response);

            if (datasetItems.length === 0) {
                zeroResultQueries += 1;
                console.log(`No Google Finance search results found for ${describeGoogleFinanceSearchRequest(params)}`);
                continue;
            }

            const pushed = await pushSearchItems(datasetItems);
            if (!pushed) {
                return;
            }

            totalResults += datasetItems.length;
            console.log(`Saved ${datasetItems.length} Google Finance search results for ${describeGoogleFinanceSearchRequest(params)} (raw count: ${resultCount})`);
        }

        console.log('Google Finance search scraping completed successfully');
        console.log('Results summary:', JSON.stringify({
            queries: requests.length,
            total_results: totalResults,
            zero_result_queries: zeroResultQueries,
        }));
    } catch (error) {
        if (isScrappaUpstreamFailure(error)) {
            const statusMessage = `Scrappa upstream returned ${error.status} after retries; no Google Finance search results were written or charged. Try the run again later.`;
            console.warn(statusMessage);
            await Actor.exit({ statusMessage });
            return;
        }

        if (error instanceof ScrappaTimeoutError) {
            const statusMessage = `${error.message}. No Google Finance search results were written or charged. Try the run again later.`;
            console.warn(statusMessage);
            await Actor.exit({ statusMessage });
            return;
        }

        const rawMessage = describeUnknownError(error);
        console.error('Actor failed: ' + rawMessage);
        await Actor.fail(rawMessage);
        return;
    }

    await Actor.exit();
}

main().catch((error) => {
    const message = describeUnknownError(error);
    console.error('Actor failed: ' + message);
    process.exitCode = 1;
});

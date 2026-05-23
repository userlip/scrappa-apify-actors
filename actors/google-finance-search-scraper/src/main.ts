import { Actor } from 'apify';
import { buildGoogleFinanceSearchRequests, describeGoogleFinanceSearchRequest } from './request-params.js';
import type { GoogleFinanceSearchInput } from './request-params.js';
import { buildSearchDatasetItems, countSearchResults } from './response-utils.js';
import type { GoogleFinanceSearchResponse } from './response-utils.js';
import { actorChargingApi, pushSearchItems } from './charging.js';
import { ScrappaClient, ScrappaHttpError, isRetryableScrappaError } from './shared/index.js';
import { buildTransientFailureStatusMessage } from './status-messages.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 30000;
const SCRAPPA_MAX_ATTEMPTS = 3;

function isScrappaHttpFailure(error: unknown): error is ScrappaHttpError {
    return error instanceof ScrappaHttpError;
}

function describeTransientFailure(error: unknown): string {
    if (isScrappaHttpFailure(error)) {
        return `Scrappa upstream returned ${error.status} after retries`;
    }

    return describeUnknownError(error);
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

async function exitOrFailTransientFailure(
    statusMessage: string,
    totalResults: number,
    totalQueries: number,
): Promise<void> {
    console.warn(statusMessage);
    if (totalResults > 0 || totalQueries > 1) {
        await Actor.fail(statusMessage);
    } else {
        await Actor.exit({ statusMessage });
    }
}

async function main(): Promise<void> {
    await Actor.init();
    let totalResults = 0;
    let totalQueries = 0;

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
        totalQueries = requests.length;
        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
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

            const pushResult = await pushSearchItems(actorChargingApi, datasetItems);
            if (!pushResult.pushed) {
                await Actor.exit({ statusMessage: pushResult.statusMessage ?? undefined });
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
        if (isRetryableScrappaError(error)) {
            const statusMessage = buildTransientFailureStatusMessage(
                describeTransientFailure(error),
                totalResults,
                totalQueries,
            );
            await exitOrFailTransientFailure(statusMessage, totalResults, totalQueries);
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

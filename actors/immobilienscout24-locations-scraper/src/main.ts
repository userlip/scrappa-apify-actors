import { pathToFileURL } from 'node:url';
import { Actor } from 'apify';
import { buildLocationRequests } from './input.js';
import type { LocationsInput, LocationsRequest } from './input.js';
import { buildUniqueLocationItems } from './locations.js';
import type { LocationDatasetItem, LocationsResponse } from './locations.js';
import { describeError, ScrappaApiError, ScrappaClient, ScrappaTimeoutError } from './shared/scrappa-client.js';

const REQUEST_TIMEOUT_MS = 10000;
const MAX_ATTEMPTS = 2;
const REQUEST_CONCURRENCY = 10;
const CHARGE_EVENT = 'location-result';

interface LocationsClient {
    get<T>(endpoint: string, params: Record<string, unknown>, attempts: number): Promise<T>;
}

interface SaveResult {
    savedCount: number;
    limitReached: boolean;
}

interface ProcessingSummary {
    failedQueries: number;
    savedResults: number;
    limitReached: boolean;
}

type FetchOutcome =
    | { response: LocationsResponse; request: LocationsRequest }
    | { error: unknown; request: LocationsRequest };

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<LocationsInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const requests = buildLocationRequests(input);
        const client = new ScrappaClient({ apiKey, timeoutMs: REQUEST_TIMEOUT_MS });
        const summary = await processLocationRequests(requests, client, pushLocationItems);

        if (summary.limitReached) {
            await Actor.exit({ statusMessage: `Charge limit reached after saving ${summary.savedResults} location result(s).` });
            return;
        }
        if (summary.failedQueries === requests.length) {
            throw new Error(`All ${summary.failedQueries} location queries failed`);
        }

        console.log('ImmobilienScout24 location autocomplete completed', JSON.stringify({
            queries: requests.length,
            failed_queries: summary.failedQueries,
            unique_results: summary.savedResults,
        }));
    } catch (error) {
        const message = describeError(error);
        console.error('Actor failed: ' + message);
        await Actor.fail(message);
        return;
    }

    await Actor.exit();
}

export async function processLocationRequests(
    requests: LocationsRequest[],
    client: LocationsClient,
    save: (items: LocationDatasetItem[]) => Promise<SaveResult>,
): Promise<ProcessingSummary> {
    const seenGeocodes = new Set<string>();
    let savedResults = 0;
    let failedQueries = 0;

    for (let offset = 0; offset < requests.length; offset += REQUEST_CONCURRENCY) {
        const outcomes = await Promise.all(
            requests.slice(offset, offset + REQUEST_CONCURRENCY).map((request) => fetchLocations(client, request)),
        );

        for (const outcome of outcomes) {
            if ('error' in outcome) {
                failedQueries += 1;
                console.warn(`Location query “${outcome.request.query}” failed: ${formatQueryFailure(outcome.error)}`);
                continue;
            }

            const items = buildUniqueLocationItems(outcome.response, outcome.request.query, seenGeocodes);
            if (items.length === 0) {
                console.log(`No new ImmobilienScout24 location matches for “${outcome.request.query}”`);
                continue;
            }

            const result = await save(items);
            savedResults += result.savedCount;
            console.log(`Saved ${result.savedCount} location match(es) for “${outcome.request.query}”`);
            if (result.limitReached) {
                return { failedQueries, savedResults, limitReached: true };
            }
        }
    }

    return { failedQueries, savedResults, limitReached: false };
}

async function fetchLocations(client: LocationsClient, request: LocationsRequest): Promise<FetchOutcome> {
    try {
        const response = await client.get<LocationsResponse>('/immobilienscout24/locations', {
            query: request.query,
            limit: request.limit,
        }, MAX_ATTEMPTS);
        return { request, response };
    } catch (error) {
        return { request, error };
    }
}

async function pushLocationItems(items: LocationDatasetItem[]): Promise<SaveResult> {
    if (!Actor.getChargingManager().getPricingInfo().isPayPerEvent) {
        await Actor.pushData(items);
        return { savedCount: items.length, limitReached: false };
    }

    const result = await Actor.pushData(items, CHARGE_EVENT);
    const savedCount = Math.min(result.chargedCount, items.length);
    return {
        savedCount,
        limitReached: result.eventChargeLimitReached || savedCount < items.length,
    };
}

function formatQueryFailure(error: unknown): string {
    if (error instanceof ScrappaTimeoutError) {
        return `${error.message}; retry this query later`;
    }
    if (error instanceof ScrappaApiError) {
        return `Scrappa returned HTTP ${error.status}: ${error.responseMessage}`;
    }
    return describeError(error);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
    main().catch((error) => {
        console.error('Actor failed: ' + describeError(error));
        process.exitCode = 1;
    });
}

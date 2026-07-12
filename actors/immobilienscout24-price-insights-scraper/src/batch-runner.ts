import type { PriceInsightsRequest } from './request-params.js';
import { buildPriceInsightItem } from './response-utils.js';
import type { PriceInsightsResponse } from './response-utils.js';
import { ScrappaApiError } from './shared/index.js';

export const PRICE_INSIGHT_RESULT_EVENT = 'price-insight-result';
const MAX_ATTEMPTS = 2;
const REQUEST_CONCURRENCY = 10;

interface PriceInsightsClient {
    get<T>(endpoint: string, params: Record<string, unknown>, options: { attempts: number }): Promise<T>;
}

interface DatasetWriter {
    pushData(item: Record<string, unknown>, eventName?: string): Promise<{
        chargedCount: number;
        eventChargeLimitReached: boolean;
    }>;
    isPayPerEvent(): boolean;
}

export interface BatchFailure {
    location: string;
    message: string;
    status: number | null;
}

export interface BatchResult {
    succeeded: number;
    failures: BatchFailure[];
    chargeLimitReached: boolean;
}

type FetchOutcome =
    | { item: Record<string, unknown> }
    | { failure: BatchFailure };

export async function runPriceInsightsBatch(
    requests: PriceInsightsRequest[],
    client: PriceInsightsClient,
    writer: DatasetWriter,
): Promise<BatchResult> {
    const failures: BatchFailure[] = [];
    let succeeded = 0;

    for (let offset = 0; offset < requests.length; offset += REQUEST_CONCURRENCY) {
        const outcomes = await Promise.all(
            requests.slice(offset, offset + REQUEST_CONCURRENCY).map((request) => fetchPriceInsight(client, request)),
        );

        for (const outcome of outcomes) {
            if ('failure' in outcome) {
                failures.push(outcome.failure);
                continue;
            }

            const isPayPerEvent = writer.isPayPerEvent();
            const pushResult = await writer.pushData(
                outcome.item,
                isPayPerEvent ? PRICE_INSIGHT_RESULT_EVENT : undefined,
            );
            // PPE writes count as successful only when Apify confirms the charge.
            if (pushResult.chargedCount >= 1 || !isPayPerEvent) {
                succeeded += 1;
            }
            if (pushResult.eventChargeLimitReached) {
                return { succeeded, failures, chargeLimitReached: true };
            }
        }
    }

    return { succeeded, failures, chargeLimitReached: false };
}

async function fetchPriceInsight(
    client: PriceInsightsClient,
    request: PriceInsightsRequest,
): Promise<FetchOutcome> {
    try {
        const response = await client.get<PriceInsightsResponse>(
            '/immobilienscout24/price-insights',
            { location: request.location },
            { attempts: MAX_ATTEMPTS },
        );
        const item = buildPriceInsightItem(response, request);
        if (item) {
            return { item };
        }

        return {
            failure: {
                location: request.location,
                message: 'Scrappa returned an incomplete price-insights snapshot',
                status: null,
            },
        };
    } catch (error) {
        return {
            failure: {
                location: request.location,
                message: error instanceof ScrappaApiError ? error.responseMessage : error instanceof Error ? error.message : String(error),
                status: error instanceof ScrappaApiError ? error.status : null,
            },
        };
    }
}

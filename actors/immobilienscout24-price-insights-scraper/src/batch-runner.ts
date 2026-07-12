import type { PriceInsightsRequest } from './request-params.js';
import { buildPriceInsightItem } from './response-utils.js';
import type { PriceInsightsResponse } from './response-utils.js';
import { ScrappaApiError } from './shared/index.js';

export const PRICE_INSIGHT_RESULT_EVENT = 'price-insight-result';

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

export async function runPriceInsightsBatch(
    requests: PriceInsightsRequest[],
    client: PriceInsightsClient,
    writer: DatasetWriter,
): Promise<BatchResult> {
    const failures: BatchFailure[] = [];
    let succeeded = 0;

    for (const request of requests) {
        let item: Record<string, unknown> | null;
        try {
            const response = await client.get<PriceInsightsResponse>(
                '/immobilienscout24/price-insights',
                { location: request.location },
                { attempts: 3 },
            );
            item = buildPriceInsightItem(response, request);
        } catch (error) {
            failures.push({
                location: request.location,
                message: error instanceof ScrappaApiError ? error.responseMessage : error instanceof Error ? error.message : String(error),
                status: error instanceof ScrappaApiError ? error.status : null,
            });
            continue;
        }

        if (!item) {
            failures.push({
                location: request.location,
                message: 'Scrappa returned an incomplete price-insights snapshot',
                status: null,
            });
            continue;
        }

        const isPayPerEvent = writer.isPayPerEvent();
        const pushResult = await writer.pushData(
            item,
            isPayPerEvent ? PRICE_INSIGHT_RESULT_EVENT : undefined,
        );
        if (pushResult.chargedCount >= 1 || !isPayPerEvent) {
            succeeded += 1;
        }
        if (pushResult.eventChargeLimitReached) {
            return { succeeded, failures, chargeLimitReached: true };
        }
    }

    return { succeeded, failures, chargeLimitReached: false };
}

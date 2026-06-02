import type { TranslationRequest } from './input.js';
import {
    buildTranslationDatasetItem,
    buildTranslationFailureItem,
    type GoogleTranslateResponse,
    type TranslationDatasetItem,
} from './results.js';
import { ScrappaHttpError, describeScrappaError } from './shared/index.js';

const SCRAPPA_MAX_ATTEMPTS = 2;

interface TranslationRunner {
    get(responsePath: string, params: Record<string, unknown>, options: { attempts: number }): Promise<unknown>;
}

interface TranslationDataset {
    push(item: TranslationDatasetItem): Promise<{ saved: boolean; statusMessage: string | null }>;
}

export interface TranslationRunSummary {
    requested: number;
    succeeded: number;
    failed: number;
    saved: number;
    statusMessage: string | null;
    firstItem: TranslationDatasetItem | null;
}

export async function runTranslations(
    requests: TranslationRequest[],
    runner: TranslationRunner,
    dataset: TranslationDataset,
    getChargeLimitStatus: (processed: number, requested: number) => string | null,
): Promise<TranslationRunSummary> {
    let succeeded = 0;
    let failed = 0;
    let saved = 0;
    let statusMessage: string | null = null;
    let firstItem: TranslationDatasetItem | null = null;

    for (const request of requests) {
        statusMessage = getChargeLimitStatus(saved, requests.length);
        if (statusMessage) {
            console.log(statusMessage);
            break;
        }

        console.log(`Translating item ${request.index + 1}/${requests.length} from ${request.source} to ${request.target}`);

        let item: TranslationDatasetItem;
        try {
            const response = await runner.get('/google-translate', request.params, { attempts: SCRAPPA_MAX_ATTEMPTS }) as GoogleTranslateResponse;
            item = buildTranslationDatasetItem(request, response);
        } catch (error) {
            if (error instanceof ScrappaHttpError && [401, 403].includes(error.status)) {
                throw error;
            }

            console.warn(`Translation item ${request.index + 1} failed: ${describeScrappaError(error)}`);
            item = buildTranslationFailureItem(request, error);
        }

        const pushResult = await dataset.push(item);
        if (!pushResult.saved) {
            statusMessage = pushResult.statusMessage ?? 'Charge limit reached before saving all successful translation results.';
            break;
        }

        firstItem ??= item;
        saved += 1;
        if (item.success) {
            succeeded += 1;
        } else {
            failed += 1;
        }
    }

    return {
        requested: requests.length,
        succeeded,
        failed,
        saved,
        statusMessage,
        firstItem,
    };
}

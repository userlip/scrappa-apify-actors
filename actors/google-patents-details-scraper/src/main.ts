import { Actor } from 'apify';
import {
    collectGooglePatentsDetailsRequests,
    describeGooglePatentsDetailsRequest,
} from './request-params.js';
import type { GooglePatentsDetailsInput } from './request-params.js';
import {
    buildErrorDatasetItem,
    buildSuccessDatasetItem,
} from './response-utils.js';
import type { GooglePatentsDetailsResponse } from './response-utils.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/scrappa-client.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;
const SCRAPPA_REQUEST_ATTEMPTS = 3;
const PATENT_DETAILS_CHARGE_EVENT = 'apify-default-dataset-item';

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<GooglePatentsDetailsInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const requests = collectGooglePatentsDetailsRequests(input);
        console.log(`Fetching Google Patents details for ${describeGooglePatentsDetailsRequest(requests)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const datasetItems: Record<string, unknown>[] = [];
        let succeeded = 0;
        let failed = 0;

        for (const request of requests) {
            console.log(`Fetching patent details: ${request.normalizedPatentId}`);

            try {
                const response = await client.get<GooglePatentsDetailsResponse>('/google-patents/details', request.params, {
                    attempts: SCRAPPA_REQUEST_ATTEMPTS,
                });

                const item = response.success
                    ? buildSuccessDatasetItem(response, request)
                    : buildErrorDatasetItem(response.error ?? response.message ?? 'Scrappa API returned success=false', request);

                datasetItems.push(item);
                if (item.success) {
                    succeeded += 1;
                } else {
                    failed += 1;
                }
            } catch (error) {
                const item = buildErrorDatasetItem(formatScrappaError(error), request);
                datasetItems.push(item);
                failed += 1;
                console.warn(`Patent details failed for ${request.normalizedPatentId}: ${String(item.error)}`);
            }
        }

        if (datasetItems.length > 0) {
            const chargeResult = await Actor.pushData(datasetItems, PATENT_DETAILS_CHARGE_EVENT);
            if (chargeResult.eventChargeLimitReached && chargeResult.chargedCount < datasetItems.length) {
                const statusMessage = `Charge limit reached after saving ${chargeResult.chargedCount}/${datasetItems.length} Google Patents detail result(s).`;
                console.log(statusMessage, JSON.stringify({
                    event: PATENT_DETAILS_CHARGE_EVENT,
                    charged_count: chargeResult.chargedCount,
                    result_count: datasetItems.length,
                }));

                await Actor.exit({ statusMessage });
                return;
            }
        }

        if (datasetItems.length === 1) {
            const store = await Actor.openKeyValueStore();
            await store.setValue('OUTPUT', datasetItems[0]);
        }

        console.log('Google Patents details scraping completed successfully');
        console.log('Details summary:', JSON.stringify({
            requested: requests.length,
            succeeded,
            failed,
        }));
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        console.error('Actor failed: ' + message);
        await Actor.fail(message);
        return;
    }

    await Actor.exit();
}

function formatScrappaError(error: unknown): unknown {
    if (error instanceof ScrappaTimeoutError) {
        return `${error.message}. The Google Patents details request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Run the request again or try a smaller batch.`;
    }

    return error;
}

main().catch((error) => {
    const message = error instanceof Error ? error.message : String(error);
    console.error('Actor failed: ' + message);
    process.exitCode = 1;
});

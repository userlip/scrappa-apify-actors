import { Actor } from 'apify';
import { formatGooglePatentsDetailsError } from './error-utils.js';
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
import { ScrappaClient } from './shared/scrappa-client.js';

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
        let firstItem: Record<string, unknown> | undefined;
        let succeeded = 0;
        let failed = 0;
        let saved = 0;

        for (const request of requests) {
            console.log(`Fetching patent details: ${request.normalizedPatentId}`);
            let item: Record<string, unknown>;

            try {
                const response = await client.get<GooglePatentsDetailsResponse>('/google-patents/details', request.params, {
                    attempts: SCRAPPA_REQUEST_ATTEMPTS,
                });

                item = response.success
                    ? buildSuccessDatasetItem(response, request)
                    : buildErrorDatasetItem(response.error ?? response.message ?? 'Scrappa API returned success=false', request);

                if (item.success) {
                    succeeded += 1;
                } else {
                    failed += 1;
                }
            } catch (error) {
                item = buildErrorDatasetItem(formatGooglePatentsDetailsError(error, SCRAPPA_REQUEST_TIMEOUT_MS), request);
                failed += 1;
                console.warn(`Patent details failed for ${request.normalizedPatentId}: ${String(item.error)}`);
            }

            firstItem ??= item;
            const chargeResult = await Actor.pushData(item, PATENT_DETAILS_CHARGE_EVENT);
            saved += chargeResult.chargedCount;

            if (chargeResult.eventChargeLimitReached && chargeResult.chargedCount < 1) {
                const statusMessage = `Charge limit reached after saving ${saved}/${requests.length} Google Patents detail result(s).`;
                console.log(statusMessage, JSON.stringify({
                    event: PATENT_DETAILS_CHARGE_EVENT,
                    charged_count: saved,
                    result_count: requests.length,
                }));

                await Actor.exit({ statusMessage });
                return;
            }
        }

        if (requests.length === 1 && firstItem) {
            const store = await Actor.openKeyValueStore();
            await store.setValue('OUTPUT', firstItem);
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

main().catch((error) => {
    const message = error instanceof Error ? error.message : String(error);
    console.error('Actor failed: ' + message);
    process.exitCode = 1;
});

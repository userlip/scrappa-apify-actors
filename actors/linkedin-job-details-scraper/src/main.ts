import { Actor } from 'apify';
import { pushChargedItems } from './charging.js';
import { getInputUrls, type LinkedInJobDetailsInput } from './input.js';
import { buildLinkedInJobDetailsParams } from './request-params.js';
import {
    buildLinkedInJobDetailsDatasetItem,
    buildLinkedInJobDetailsFailureItem,
    buildLinkedInJobDetailsOutput,
    isRecoverableLinkedInJobDetailsError,
    type LinkedInJobDetailsResponse,
    type LinkedInJobDetailsResult,
} from './results.js';
import { ScrappaClient } from './shared/index.js';

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<LinkedInJobDetailsInput>();
        const urls = getInputUrls(input);
        if (urls.length === 0) {
            throw new Error('At least one LinkedIn job URL is required. Provide either url (single URL) or urls (array of URLs).');
        }

        const client = new ScrappaClient({ apiKey });
        let firstResult: LinkedInJobDetailsResult | undefined;
        let succeeded = 0;
        let failed = 0;
        let statusMessage: string | null = null;

        console.log(`Scraping ${urls.length} LinkedIn job URL${urls.length === 1 ? '' : 's'}`);

        for (const request of urls) {
            const { input_url: inputUrl, normalized_url: normalizedUrl } = request;
            let result: LinkedInJobDetailsResult;

            if (!normalizedUrl) {
                console.warn(`Invalid LinkedIn job URL: "${inputUrl}"`);
                result = buildLinkedInJobDetailsFailureItem(
                    new Error(request.validation_error ?? 'Invalid LinkedIn job URL'),
                    inputUrl,
                );
            } else {
                console.log(`Fetching LinkedIn job details: ${normalizedUrl}`);

                const params = buildLinkedInJobDetailsParams({
                    url: normalizedUrl,
                    use_cache: input?.use_cache,
                    maximum_cache_age: input?.maximum_cache_age,
                });

                try {
                    const response = await client.get<LinkedInJobDetailsResponse>('/linkedin/job', params);
                    result = buildLinkedInJobDetailsDatasetItem(response, inputUrl, normalizedUrl);
                } catch (error) {
                    if (!isRecoverableLinkedInJobDetailsError(error)) {
                        throw error;
                    }

                    console.warn(`Job detail scraping returned a per-item failure for ${normalizedUrl}: ${error instanceof Error ? error.message : String(error)}`);
                    result = buildLinkedInJobDetailsFailureItem(error, inputUrl, normalizedUrl);
                }
            }

            const pushResult = await pushChargedItems({
                isPayPerEvent: () => Actor.getChargingManager().getPricingInfo().isPayPerEvent,
                pushData: (items, eventName) => eventName === undefined
                    ? Actor.pushData(items)
                    : Actor.pushData(items, eventName),
            }, [result], { chargeEvent: result.success === true });

            if (pushResult.statusMessage) {
                statusMessage = pushResult.statusMessage;
            }

            firstResult ??= result;
            if (result.success) {
                succeeded += pushResult.savedCount;
                console.log(`Saved LinkedIn job detail: ${result.title ?? normalizedUrl ?? inputUrl}`);
            } else {
                failed += pushResult.savedCount;
                console.warn('LinkedIn job detail failed' + (result.message ? ` (${result.message})` : ''));
            }

            if (statusMessage) {
                break;
            }
        }

        const store = await Actor.openKeyValueStore();
        if (urls.length === 1 && firstResult) {
            await store.setValue('OUTPUT', buildLinkedInJobDetailsOutput(firstResult));
        } else {
            await store.setValue('OUTPUT', {
                requested: urls.length,
                succeeded,
                failed,
            });
        }

        console.log('LinkedIn job detail scraping completed');
        console.log('Job detail summary:', JSON.stringify({
            requested: urls.length,
            succeeded,
            failed,
        }, null, 2));

        if (statusMessage) {
            await Actor.exit({ statusMessage });
            return;
        }
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        console.error(`Actor failed: ${message}`);
        await Actor.fail(message);
        return;
    }

    await Actor.exit();
}

main();

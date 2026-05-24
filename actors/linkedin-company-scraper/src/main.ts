import { Actor } from 'apify';
import { ScrappaClient } from './shared/index.js';
import { getInputUrls, type LinkedInCompanyInput } from './input.js';
import { buildLinkedInCompanyParams } from './request-params.js';
import {
    buildLinkedInCompanyDatasetItem,
    buildLinkedInCompanyFailureItem,
    isRecoverableLinkedInCompanyError,
    type LinkedInCompanyResponse,
    type LinkedInCompanyResult,
} from './results.js';

async function main(): Promise<void> {
    await Actor.init();

    try {
        // Get API key from environment variable (set as Apify secret)
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<LinkedInCompanyInput>();
        const requests = getInputUrls(input);
        if (requests.length === 0) {
            throw new Error('At least one LinkedIn company URL is required. Provide url or urls.');
        }

        const client = new ScrappaClient({ apiKey });
        let firstResult: LinkedInCompanyResult | undefined;
        let succeeded = 0;
        let failed = 0;

        console.log(`Scraping ${requests.length} LinkedIn company URL${requests.length === 1 ? '' : 's'}`);

        for (const request of requests) {
            const { input_url: inputUrl, normalized_url: normalizedUrl } = request;

            if (!normalizedUrl) {
                console.warn(`Invalid LinkedIn company URL: "${inputUrl}"`);
                const result = buildLinkedInCompanyFailureItem(
                    new Error(request.validation_error ?? 'Invalid LinkedIn company URL'),
                    inputUrl,
                );
                await Actor.pushData(result);
                firstResult ??= result;
                failed += 1;
                continue;
            }

            console.log(`Scraping LinkedIn company: "${normalizedUrl}"`);

            const params = buildLinkedInCompanyParams({
                url: normalizedUrl,
                use_cache: input?.use_cache,
                maximum_cache_age: input?.maximum_cache_age,
            });

            let result: LinkedInCompanyResult;

            try {
                const response = await client.get<LinkedInCompanyResponse>('/linkedin/company', params);
                result = buildLinkedInCompanyDatasetItem(response, inputUrl, normalizedUrl);
            } catch (error) {
                if (!isRecoverableLinkedInCompanyError(error)) {
                    throw error;
                }

                console.warn(`Company scraping returned a per-item failure for ${normalizedUrl}: ${error instanceof Error ? error.message : String(error)}`);
                result = buildLinkedInCompanyFailureItem(error, inputUrl, normalizedUrl);
            }

            if (!result.success && result.status_code !== 404) {
                console.warn('Company scraping returned success: false' + (result.message ? ` (${result.message})` : ''));
            } else if (result.success) {
                console.log(`Successfully scraped company: ${result.name || 'Unknown'}`);
            }

            await Actor.pushData(result);
            firstResult ??= result;

            if (result.success) {
                succeeded += 1;
            } else {
                failed += 1;
            }
        }

        if (requests.length === 1 && firstResult) {
            const store = await Actor.openKeyValueStore();
            await store.setValue('OUTPUT', firstResult);
        } else {
            const store = await Actor.openKeyValueStore();
            await store.setValue('OUTPUT', {
                requested: requests.length,
                succeeded,
                failed,
            });
        }

        // Log summary
        console.log('LinkedIn Company scrape completed successfully');

        const summary = {
            requested: requests.length,
            succeeded,
            failed,
        };

        console.log('Results summary:', JSON.stringify(summary, null, 2));

    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        console.error(`Actor failed: ${message}`);
        await Actor.fail(message);
        return;
    }

    await Actor.exit();
}

main();

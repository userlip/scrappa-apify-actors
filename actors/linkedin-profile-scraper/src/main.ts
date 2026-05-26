import { Actor } from 'apify';
import { ScrappaClient } from './shared/index.js';
import { getInputUrls, type LinkedInProfileInput } from './input.js';
import { buildLinkedInProfileParams } from './request-params.js';
import {
    buildLinkedInProfileDatasetItem,
    buildLinkedInProfileFailureItem,
    isRecoverableLinkedInProfileError,
    type LinkedInProfileResponse,
    type LinkedInProfileResult,
} from './results.js';

async function main(): Promise<void> {
    await Actor.init();

    try {
        // Get API key from environment variable (set as Apify secret)
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<LinkedInProfileInput>();
        const urls = getInputUrls(input);
        if (urls.length === 0) {
            throw new Error('At least one LinkedIn profile URL is required. Provide url or urls.');
        }

        const client = new ScrappaClient({ apiKey });
        let firstResult: LinkedInProfileResult | undefined;
        let succeeded = 0;
        let failed = 0;

        console.log(`Scraping ${urls.length} LinkedIn profile URL${urls.length === 1 ? '' : 's'}`);

        for (const request of urls) {
            const { input_url: inputUrl, normalized_url: normalizedUrl } = request;

            if (!normalizedUrl) {
                console.warn(`Invalid LinkedIn profile URL: "${inputUrl}"`);
                const result = buildLinkedInProfileFailureItem(
                    new Error(request.validation_error ?? 'Invalid LinkedIn profile URL'),
                    inputUrl,
                );
                await Actor.pushData(result);
                firstResult ??= result;
                failed += 1;
                continue;
            }

            console.log(`Fetching LinkedIn profile: ${normalizedUrl}`);

            const params = buildLinkedInProfileParams({
                url: normalizedUrl,
                use_cache: input?.use_cache,
                maximum_cache_age: input?.maximum_cache_age,
            });

            let result: LinkedInProfileResult;

            try {
                const response = await client.get<LinkedInProfileResponse>('/linkedin/profile', params);
                result = buildLinkedInProfileDatasetItem(response, inputUrl, normalizedUrl);
            } catch (error) {
                if (!isRecoverableLinkedInProfileError(error)) {
                    throw error;
                }

                console.warn(`Profile scraping returned a per-item failure for ${normalizedUrl}: ${error instanceof Error ? error.message : String(error)}`);
                result = buildLinkedInProfileFailureItem(error, inputUrl, normalizedUrl);
            }

            // Push the entire profile as a single dataset item
            if (result.success) {
                console.log(`Successfully scraped profile: ${result.name || 'Unknown'}`);
            } else if (result.status_code !== 404) {
                console.warn('Profile scraping returned success: false' + (result.message ? ` (${result.message})` : ''));
            }

            await Actor.pushData(result);
            firstResult ??= result;

            if (result.success) {
                succeeded += 1;
            } else {
                failed += 1;
            }
        }

        if (urls.length === 1 && firstResult) {
            const store = await Actor.openKeyValueStore();
            await store.setValue('OUTPUT', firstResult);
        } else {
            const store = await Actor.openKeyValueStore();
            await store.setValue('OUTPUT', {
                requested: urls.length,
                succeeded,
                failed,
            });
        }

        // Log summary
        console.log('LinkedIn profile scraping completed');

        const summary = {
            requested: urls.length,
            succeeded,
            failed,
        };

        console.log('Profile summary:', JSON.stringify(summary, null, 2));

    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        console.error(`Actor failed: ${message}`);
        await Actor.fail(message);
        return;
    }

    await Actor.exit();
}

main();

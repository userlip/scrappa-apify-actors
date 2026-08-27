import { Actor } from 'apify';
import { ScrappaClient } from './shared/index.js';
import { getInputUrls, type LinkedInProfileInput } from './input.js';
import { publishLinkedInProfileResults } from './publication.js';
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
            throw new Error('At least one LinkedIn profile URL is required. Provide either url (single URL) or urls (array of URLs).');
        }

        const client = new ScrappaClient({ apiKey });
        const results: LinkedInProfileResult[] = [];

        console.log(`Scraping ${urls.length} LinkedIn profile URL${urls.length === 1 ? '' : 's'}`);

        for (const request of urls) {
            const { input_url: inputUrl, normalized_url: normalizedUrl } = request;

            if (!normalizedUrl) {
                console.warn(`Invalid LinkedIn profile URL: "${inputUrl}"`);
                const result = buildLinkedInProfileFailureItem(
                    new Error(request.validation_error ?? 'Invalid LinkedIn profile URL'),
                    inputUrl,
                );
                results.push(result);
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

            results.push(result);
        }

        const store = await Actor.openKeyValueStore();
        const summary = await publishLinkedInProfileResults(results, {
            pushData: (result) => Actor.pushData(result),
            setValue: (key, value) => store.setValue(key, value),
        });

        // Log summary
        console.log('LinkedIn profile scraping completed');

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

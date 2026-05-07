import { Actor } from 'apify';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';
import {
    buildLinkedInJobsSearchParams,
    normalizeLinkedInJobsSearchInput,
} from './search-params.js';
import type { LinkedInJobsSearchInput } from './search-params.js';
import { getJobSearchResults } from './search-response.js';
import type { LinkedInJobsSearchResponse } from './search-response.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;
const SCRAPPA_MAX_ATTEMPTS = 3;

await Actor.init();

try {
    const apiKey = process.env.SCRAPPA_API_KEY;
    if (!apiKey) {
        throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
    }

    const input = normalizeLinkedInJobsSearchInput(await Actor.getInput<LinkedInJobsSearchInput>());
    if (!input.query) {
        throw new Error('LinkedIn jobs search query is required.');
    }
    if (input.page !== undefined && input.start !== undefined) {
        throw new Error('Use either page or start for pagination, not both.');
    }

    console.log(`Searching LinkedIn Jobs for: "${input.query}"`);

    const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
    const response = await client.get<LinkedInJobsSearchResponse>(
        '/linkedin/jobs/search',
        buildLinkedInJobsSearchParams(input),
        { attempts: SCRAPPA_MAX_ATTEMPTS }
    );
    const jobs = getJobSearchResults(response);

    if (jobs.length > 0) {
        await Actor.pushData(jobs);
        console.log(`Found ${jobs.length} LinkedIn job result(s)`);
    } else {
        console.log('No LinkedIn job results found for the given search criteria');
    }

    const store = await Actor.openKeyValueStore();
    await store.setValue('OUTPUT', response);

    console.log('LinkedIn Jobs search completed successfully');

    const summary = {
        jobs: jobs.length,
        total_results: response.total_results ?? response.search_information?.total_results ?? null,
        current_page: response.pagination?.current_page ?? null,
        pages: Array.isArray(response.pagination?.pages) ? response.pagination.pages.length : 0,
    };

    console.log('Results summary:', JSON.stringify(summary));

} catch (error) {
    const rawMessage = error instanceof Error ? error.message : String(error);
    const message = error instanceof ScrappaTimeoutError
        ? `${rawMessage}. The LinkedIn Jobs Search request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try again or refine the query.`
        : rawMessage;
    console.error('Actor failed: ' + message);
    await Actor.fail(message);
}

await Actor.exit();

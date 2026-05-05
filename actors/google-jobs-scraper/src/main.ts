import { Actor } from 'apify';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';
import { buildJobsParams, normalizeJobsInput } from './jobs-params.js';
import type { GoogleJobsInput } from './jobs-params.js';
import { getJobs, getNextPageToken } from './jobs-response.js';
import type { GoogleJobsResponse } from './jobs-response.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;
const SCRAPPA_MAX_ATTEMPTS = 3;

await Actor.init();

try {
    const apiKey = process.env.SCRAPPA_API_KEY;
    if (!apiKey) {
        throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
    }

    const input = normalizeJobsInput(await Actor.getInput<GoogleJobsInput>());
    if (!input?.q && !input?.next_page_token) {
        throw new Error('Job search query is required unless next_page_token is provided.');
    }
    if (input.lrad !== undefined && !input.uule) {
        throw new Error('Search radius (lrad) requires an encoded location (uule).');
    }

    const searchLabel = input.q ? `"${input.q}"` : 'next page token';
    console.log(`Searching Google Jobs for: ${searchLabel}`);

    const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
    const response = await client.get<GoogleJobsResponse>('/google/jobs', buildJobsParams(input), {
        attempts: SCRAPPA_MAX_ATTEMPTS,
    });
    const jobs = getJobs(response);

    if (jobs.length > 0) {
        await Actor.pushData(jobs);
        console.log(`Found ${jobs.length} job result(s)`);
    } else {
        console.log('No job results found for the given search criteria');
    }

    const store = await Actor.openKeyValueStore();
    await store.setValue('OUTPUT', response);

    console.log('Google Jobs search completed successfully');

    const summary = {
        jobs: jobs.length,
        filters: Array.isArray(response.filters) ? response.filters.length : 0,
        has_next_page: Boolean(getNextPageToken(response)),
    };

    console.log('Results summary:', JSON.stringify(summary));

} catch (error) {
    const rawMessage = error instanceof Error ? error.message : String(error);
    const message = error instanceof ScrappaTimeoutError
        ? `${rawMessage}. The Google Jobs request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try again or refine the query.`
        : rawMessage;
    console.error('Actor failed: ' + message);
    await Actor.fail(message);
}

await Actor.exit();

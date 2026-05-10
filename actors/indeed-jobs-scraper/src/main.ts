import { Actor } from 'apify';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';
import { buildIndeedJobsParams, normalizeIndeedJobsInput } from './indeed-params.js';
import type { IndeedJobsInput } from './indeed-params.js';
import {
    getCompanyName,
    getFormattedLocation,
    getIndeedJobs,
    getIndeedMetadata,
    getIndeedPagination,
} from './indeed-response.js';
import type { IndeedJobsResponse } from './indeed-response.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;
const SCRAPPA_MAX_ATTEMPTS = 3;

await Actor.init();

try {
    const apiKey = process.env.SCRAPPA_API_KEY;
    if (!apiKey) {
        throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
    }

    const input = normalizeIndeedJobsInput(await Actor.getInput<IndeedJobsInput>());
    if (!input.query) {
        throw new Error('Indeed jobs search query is required.');
    }

    console.log(`Searching Indeed Jobs for: "${input.query}"`);

    const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
    const response = await client.get<IndeedJobsResponse>(
        '/indeed/jobs',
        buildIndeedJobsParams(input),
        { attempts: SCRAPPA_MAX_ATTEMPTS }
    );
    const jobs = getIndeedJobs(response);

    if (jobs.length > 0) {
        await Actor.pushData(jobs);
        console.log(`Found ${jobs.length} Indeed job result(s)`);
    } else {
        console.log('No Indeed job results found for the given search criteria');
    }

    const store = await Actor.openKeyValueStore();
    await store.setValue('OUTPUT', response);

    console.log('Indeed Jobs search completed successfully');

    const pagination = getIndeedPagination(response);
    const metadata = getIndeedMetadata(response);
    const firstJob = jobs[0];
    const summary = {
        jobs: jobs.length,
        has_more: Boolean(pagination?.has_more),
        next_cursor: pagination?.next_cursor ? 'present' : null,
        total_results: metadata?.total_results ?? jobs.length,
        first_job: firstJob ? {
            title: firstJob.title ?? null,
            company: getCompanyName(firstJob.company) ?? null,
            location: getFormattedLocation(firstJob.location) ?? null,
        } : null,
    };

    console.log('Results summary:', JSON.stringify(summary));

} catch (error) {
    const rawMessage = error instanceof Error ? error.message : String(error);
    const message = error instanceof ScrappaTimeoutError
        ? `${rawMessage}. The Indeed Jobs request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try again or refine the query.`
        : rawMessage;
    console.error('Actor failed: ' + message);
    await Actor.fail(message);
}

await Actor.exit();

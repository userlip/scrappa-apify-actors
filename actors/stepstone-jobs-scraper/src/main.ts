import { Actor } from 'apify';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';
import { buildStepstoneJobsParams, normalizeStepstoneJobsInput } from './stepstone-params.js';
import type { StepstoneJobsInput } from './stepstone-params.js';
import {
    getCompanyName,
    getFormattedLocation,
    getStepstoneJobs,
    getStepstoneMetadata,
    getStepstonePagination,
    toStepstoneDatasetJob,
} from './stepstone-response.js';
import type { StepstoneJobsResponse } from './stepstone-response.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;
const SCRAPPA_MAX_ATTEMPTS = 3;

await Actor.init();

try {
    const apiKey = process.env.SCRAPPA_API_KEY;
    if (!apiKey) {
        throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
    }

    const input = normalizeStepstoneJobsInput(await Actor.getInput<StepstoneJobsInput>());
    if (!input.query) {
        throw new Error('Stepstone jobs search query is required.');
    }

    console.log(`Searching Stepstone Jobs for: "${input.query}"`);

    const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
    const response = await client.get<StepstoneJobsResponse>(
        '/stepstone/jobs',
        buildStepstoneJobsParams(input),
        { attempts: SCRAPPA_MAX_ATTEMPTS }
    );
    const jobs = getStepstoneJobs(response);
    const datasetJobs = jobs.map(toStepstoneDatasetJob);

    if (datasetJobs.length > 0) {
        await Actor.pushData(datasetJobs);
        console.log(`Found ${datasetJobs.length} Stepstone job result(s)`);
    } else {
        console.log('No Stepstone job results found for the given search criteria');
    }

    const store = await Actor.openKeyValueStore();
    await store.setValue('OUTPUT', response);

    console.log('Stepstone Jobs search completed successfully');

    const pagination = getStepstonePagination(response);
    const metadata = getStepstoneMetadata(response);
    const firstJob = jobs[0];
    const summary = {
        jobs: jobs.length,
        has_more: Boolean(pagination?.has_more),
        next_page: pagination?.next_page ?? null,
        total_jobs: pagination?.total_jobs ?? jobs.length,
        query: metadata?.query ?? input.query,
        country: metadata?.country ?? input.country,
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
        ? `${rawMessage}. The Stepstone Jobs request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try again or refine the query.`
        : rawMessage;
    console.error('Actor failed: ' + message);
    await Actor.fail(message);
}

await Actor.exit();

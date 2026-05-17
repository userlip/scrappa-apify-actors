import { Actor } from 'apify';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';
import { buildArbeitsagenturJobsParams, normalizeArbeitsagenturJobsInput } from './arbeitsagentur-params.js';
import type { ArbeitsagenturJobsInput } from './arbeitsagentur-params.js';
import {
    getArbeitsagenturJobs,
    getArbeitsagenturMetadata,
    getFormattedLocation,
    toArbeitsagenturDatasetJob,
} from './arbeitsagentur-response.js';
import type { ArbeitsagenturJobsResponse } from './arbeitsagentur-response.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;
const SCRAPPA_MAX_ATTEMPTS = 3;

await Actor.init();

try {
    const apiKey = process.env.SCRAPPA_API_KEY;
    if (!apiKey) {
        throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
    }

    const input = normalizeArbeitsagenturJobsInput(await Actor.getInput<ArbeitsagenturJobsInput>());
    if (!input.was) {
        throw new Error('Arbeitsagentur jobs search keyword is required.');
    }

    console.log(`Searching Arbeitsagentur Jobs for: "${input.was}"`);

    const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
    const response = await client.get<ArbeitsagenturJobsResponse>(
        '/arbeitsagentur/jobs',
        buildArbeitsagenturJobsParams(input),
        { attempts: SCRAPPA_MAX_ATTEMPTS }
    );
    const jobs = getArbeitsagenturJobs(response);
    const datasetJobs = jobs.map(toArbeitsagenturDatasetJob);

    if (datasetJobs.length > 0) {
        await Actor.pushData(datasetJobs);
        console.log(`Found ${datasetJobs.length} Arbeitsagentur job result(s)`);
    } else {
        console.log('No Arbeitsagentur job results found for the given search criteria');
    }

    const store = await Actor.openKeyValueStore();
    await store.setValue('OUTPUT', response);

    console.log('Arbeitsagentur Jobs search completed successfully');

    const metadata = getArbeitsagenturMetadata(response);
    const firstJob = jobs[0];
    const summary = {
        jobs: jobs.length,
        total_jobs: metadata?.maxErgebnisse ?? jobs.length,
        page: metadata?.page ?? input.page,
        size: metadata?.size ?? input.size,
        query: input.was,
        location: input.wo ?? null,
        first_job: firstJob ? {
            title: firstJob.titel ?? null,
            company: firstJob.arbeitgeber ?? null,
            location: getFormattedLocation(firstJob.arbeitsort) ?? null,
            reference_number: firstJob.refnr ?? null,
        } : null,
    };

    console.log('Results summary:', JSON.stringify(summary));

} catch (error) {
    const rawMessage = error instanceof Error ? error.message : String(error);
    const message = error instanceof ScrappaTimeoutError
        ? `${rawMessage}. The Arbeitsagentur Jobs request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try again or refine the query.`
        : rawMessage;
    console.error('Actor failed: ' + message);
    await Actor.fail(message);
}

await Actor.exit();

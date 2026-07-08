import { Actor } from 'apify';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';
import {
    buildKununuJobsSearchPlan,
    describeKununuJobsRequest,
} from './kununu-jobs-params.js';
import type { KununuJobsInput } from './kununu-jobs-params.js';
import {
    getCompanyName,
    getFormattedLocation,
    getKununuJobs,
    getKununuMetadata,
    getKununuPagination,
    toKununuDatasetJob,
} from './kununu-jobs-response.js';
import type { KununuJobsResponse } from './kununu-jobs-response.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;
const SCRAPPA_MAX_ATTEMPTS = 3;
const KUNUNU_JOB_RESULT_CHARGE_EVENT = 'kununu-job-result';

await Actor.init();

try {
    const apiKey = process.env.SCRAPPA_API_KEY;
    if (!apiKey) {
        throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
    }

    const plan = buildKununuJobsSearchPlan(await Actor.getInput<KununuJobsInput>());
    console.log(`Searching Kununu Jobs for: ${describeKununuJobsRequest(plan.params)}`);

    const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
    const response = await client.get<KununuJobsResponse>(
        '/kununu/jobs',
        plan.params,
        { attempts: SCRAPPA_MAX_ATTEMPTS }
    );
    const jobs = getKununuJobs(response);
    const datasetJobs = jobs.map((job) => toKununuDatasetJob(job, { includeRawJob: plan.includeRawJob }));

    let savedJobs = 0;
    if (datasetJobs.length > 0) {
        const chargeResult = await Actor.pushData(datasetJobs, KUNUNU_JOB_RESULT_CHARGE_EVENT);
        savedJobs = chargeResult.chargedCount > 0 || chargeResult.eventChargeLimitReached
            ? Math.min(chargeResult.chargedCount, datasetJobs.length)
            : datasetJobs.length;

        if (chargeResult.eventChargeLimitReached && savedJobs < datasetJobs.length) {
            const statusMessage = `Charge limit reached after saving ${savedJobs} of ${datasetJobs.length} Kununu job result(s).`;
            console.log(statusMessage, JSON.stringify({
                event: KUNUNU_JOB_RESULT_CHARGE_EVENT,
                charged_count: chargeResult.chargedCount,
                requested_count: datasetJobs.length,
            }));
            await Actor.exit({ statusMessage });
        }

        console.log(`Found ${datasetJobs.length} Kununu job result(s)`);
    } else {
        console.log('No Kununu job results found for the given search criteria');
    }

    const store = await Actor.openKeyValueStore();
    await store.setValue('OUTPUT', response);

    console.log('Kununu Jobs search completed successfully');

    const pagination = getKununuPagination(response);
    const metadata = getKununuMetadata(response);
    const firstJob = jobs[0];
    const summary = {
        jobs: jobs.length,
        saved_jobs: savedJobs,
        page: plan.params.page,
        total_jobs: pagination?.total_jobs ?? pagination?.totalResults ?? metadata?.total_jobs ?? jobs.length,
        first_job: firstJob ? {
            title: firstJob.title ?? null,
            company: getCompanyName(firstJob.company, firstJob) ?? null,
            location: getFormattedLocation(firstJob.location, firstJob) ?? null,
        } : null,
    };

    console.log('Results summary:', JSON.stringify(summary));

} catch (error) {
    const rawMessage = error instanceof Error ? error.message : String(error);
    const message = error instanceof ScrappaTimeoutError
        ? `${rawMessage}. The Kununu Jobs request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try again or refine the query.`
        : rawMessage;
    console.error('Actor failed: ' + message);
    await Actor.fail(message);
}

await Actor.exit();

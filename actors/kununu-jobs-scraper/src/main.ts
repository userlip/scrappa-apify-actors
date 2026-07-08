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
    getKununuPagination,
    toKununuDatasetJob,
} from './kununu-jobs-response.js';
import type { KununuJobsResponse } from './kununu-jobs-response.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;
const SCRAPPA_MAX_ATTEMPTS = 3;
const KUNUNU_JOB_RESULT_CHARGE_EVENT = 'kununu-job-result';

await Actor.init();

interface PageSummary {
    page: number;
    count: number;
    pagination?: Record<string, unknown>;
}

let actorFinished = false;

try {
    const apiKey = process.env.SCRAPPA_API_KEY;
    if (!apiKey) {
        throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
    }

    const plan = buildKununuJobsSearchPlan(await Actor.getInput<KununuJobsInput>());
    console.log(`Searching Kununu Jobs for: ${describeKununuJobsRequest(plan.params)}`);

    const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
    const pages: PageSummary[] = [];
    let totalJobs = 0;
    let savedJobs = 0;
    let firstJobSummary: Record<string, unknown> | null = null;
    let exitStatusMessage: string | null = null;

    for (let offset = 0; offset < plan.maxPages; offset++) {
        const page = plan.startPage + offset;
        const params = { ...plan.params, page };
        console.log(`Fetching Kununu Jobs page ${page}`);

        const response = await client.get<KununuJobsResponse>(
            '/kununu/jobs',
            params,
            { attempts: SCRAPPA_MAX_ATTEMPTS }
        );
        const jobs = getKununuJobs(response);
        const pagination = getKununuPagination(response);
        const datasetJobs = jobs.map((job) => toKununuDatasetJob(job, { includeRawJob: plan.includeRawJob }));

        pages.push({ page, count: jobs.length, pagination });
        totalJobs += jobs.length;

        if (!firstJobSummary && jobs[0]) {
            firstJobSummary = {
                title: jobs[0].title ?? null,
                company: getCompanyName(jobs[0].company, jobs[0]) ?? null,
                location: getFormattedLocation(jobs[0].location, jobs[0]) ?? null,
            };
        }

        if (datasetJobs.length > 0) {
            const chargeResult = await Actor.pushData(datasetJobs, KUNUNU_JOB_RESULT_CHARGE_EVENT);
            const pageSavedJobs = chargeResult.chargedCount > 0 || chargeResult.eventChargeLimitReached
                ? Math.min(chargeResult.chargedCount, datasetJobs.length)
                : datasetJobs.length;
            savedJobs += pageSavedJobs;

            if (chargeResult.chargedCount === 0 && !chargeResult.eventChargeLimitReached) {
                console.warn(`Saved ${datasetJobs.length} Kununu job result(s) on page ${page}, but no ${KUNUNU_JOB_RESULT_CHARGE_EVENT} events were charged. Check actor pricing if this was a paid run.`);
            }

            if (chargeResult.eventChargeLimitReached && pageSavedJobs < datasetJobs.length) {
                const statusMessage = `Charge limit reached after saving ${pageSavedJobs} of ${datasetJobs.length} Kununu job result(s) on page ${page}.`;
                console.log(statusMessage, JSON.stringify({
                    event: KUNUNU_JOB_RESULT_CHARGE_EVENT,
                    charged_count: chargeResult.chargedCount,
                    requested_count: datasetJobs.length,
                    page,
                }));
                actorFinished = true;
                exitStatusMessage = statusMessage;
                break;
            }

            console.log(`Found ${datasetJobs.length} Kununu job result(s) on page ${page}`);
        } else {
            console.log(`No Kununu job results found on page ${page}`);
            break;
        }

        const lastPage = getLastPage(pagination);
        if (lastPage !== undefined && page >= lastPage) {
            console.log(`Stopping after page ${page}; Scrappa reported ${lastPage} total page(s)`);
            break;
        }
    }

    if (exitStatusMessage) {
        await Actor.exit({ statusMessage: exitStatusMessage });
    }

    if (!actorFinished) {
        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', {
            request: {
                ...plan.params,
                start_page: plan.startPage,
                max_pages: plan.maxPages,
            },
            pages_fetched: pages.length,
            jobs_extracted: totalJobs,
            jobs_saved: savedJobs,
            pages,
        });

        console.log('Kununu Jobs search completed successfully');

        const summary = {
            jobs: totalJobs,
            saved_jobs: savedJobs,
            pages_fetched: pages.length,
            start_page: plan.startPage,
            first_job: firstJobSummary,
        };

        console.log('Results summary:', JSON.stringify(summary));
    }

} catch (error) {
    const rawMessage = error instanceof Error ? error.message : String(error);
    const message = error instanceof ScrappaTimeoutError
        ? `${rawMessage}. The Kununu Jobs request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try again or refine the query.`
        : rawMessage;
    console.error('Actor failed: ' + message);
    await Actor.fail(message);
    actorFinished = true;
}

if (!actorFinished) {
    await Actor.exit();
}

function getLastPage(pagination: Record<string, unknown> | undefined): number | undefined {
    const value = pagination?.lastPage ?? pagination?.last_page ?? pagination?.totalPages ?? pagination?.total_pages;
    return typeof value === 'number' && Number.isInteger(value) ? value : undefined;
}

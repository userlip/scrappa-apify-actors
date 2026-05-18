import { Actor } from 'apify';
import {
    buildJamedaSearchPlan,
    buildPageParams,
    describeJamedaSearchRequest,
} from './request-params.js';
import type { JamedaSearchInput } from './request-params.js';
import {
    buildJamedaDoctorDatasetItem,
    getJamedaDoctors,
} from './response-utils.js';
import type { JamedaSearchResponse } from './response-utils.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 90000;
const SCRAPPA_MAX_ATTEMPTS = 3;
const DOCTOR_RESULT_CHARGE_EVENT = 'doctor-result';

interface PushChargedItemsResult {
    savedCount: number;
    statusMessage: string | null;
}

async function pushChargedItems(items: Record<string, unknown>[], page: number): Promise<PushChargedItemsResult> {
    if (items.length === 0) {
        return { savedCount: 0, statusMessage: null };
    }

    const { isPayPerEvent } = Actor.getChargingManager().getPricingInfo();
    if (!isPayPerEvent) {
        await Actor.pushData(items);
        return { savedCount: items.length, statusMessage: null };
    }

    const chargeResult = await Actor.pushData(items, DOCTOR_RESULT_CHARGE_EVENT);
    if (chargeResult.eventChargeLimitReached) {
        const savedCount = Math.min(chargeResult.chargedCount, items.length);
        const statusMessage = `Charge limit reached after saving ${savedCount} of ${items.length} Jameda doctor results on page ${page}.`;
        console.log(statusMessage, JSON.stringify({
            event: DOCTOR_RESULT_CHARGE_EVENT,
            charged_count: chargeResult.chargedCount,
            requested_count: items.length,
            saved_count: savedCount,
        }));
        return { savedCount, statusMessage };
    }

    return { savedCount: items.length, statusMessage: null };
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<JamedaSearchInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const plan = buildJamedaSearchPlan(input);
        console.log(`Searching Jameda for ${describeJamedaSearchRequest(plan)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const responses: JamedaSearchResponse[] = [];
        let pagesFetched = 0;
        let savedDoctors = 0;
        let statusMessage: string | null = null;
        let latestMeta: JamedaSearchResponse['meta'] | undefined;

        for (let offset = 0; offset < plan.maxPages; offset += 1) {
            const page = plan.startPage + offset;
            const params = buildPageParams(plan, page);
            console.log(`Fetching Jameda page ${page} for ${String(params.q)}${params.loc ? ` in ${String(params.loc)}` : ''}`);

            const response = await client.get<JamedaSearchResponse>('/jameda/search', params, {
                attempts: SCRAPPA_MAX_ATTEMPTS,
            });
            pagesFetched += 1;
            latestMeta = response.meta;

            const doctors = getJamedaDoctors(response).map((doctor) => buildJamedaDoctorDatasetItem(doctor, params, response));
            responses.push(response);

            if (doctors.length > 0) {
                const result = await pushChargedItems(doctors, page);
                savedDoctors += result.savedCount;
                console.log(`Found ${doctors.length} doctor result(s) on page ${page}; saved ${result.savedCount}`);
                if (result.statusMessage) {
                    statusMessage = result.statusMessage;
                    break;
                }
            } else {
                console.log(`No Jameda results found on page ${page}`);
                break;
            }

            if (response.meta?.has_next_page === false) {
                console.log(`Stopping after page ${page}; Scrappa reported no next page`);
                break;
            }

            const totalPages = response.meta?.total_pages;
            if (typeof totalPages === 'number' && page >= totalPages) {
                console.log(`Stopping after page ${page}; Scrappa reported ${totalPages} total page(s)`);
                break;
            }
        }

        const output = {
            request: {
                ...plan.baseParams,
                start_page: plan.startPage,
                max_pages: plan.maxPages,
            },
            pages_fetched: pagesFetched,
            responses_saved: responses.length,
            doctors_extracted: savedDoctors,
            status_message: statusMessage,
            total_results: latestMeta?.total_results ?? null,
            total_pages: latestMeta?.total_pages ?? null,
            responses,
        };

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', output);

        console.log('Jameda search completed successfully');
        console.log('Results summary:', JSON.stringify({
            pages_fetched: pagesFetched,
            responses_saved: responses.length,
            doctors_extracted: savedDoctors,
            total_results: output.total_results,
            total_pages: output.total_pages,
        }));

        if (statusMessage) {
            await Actor.exit({ statusMessage });
            return;
        }
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The Jameda search request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try fewer pages or run the request again.`
            : rawMessage;
        console.error('Actor failed: ' + message);
        await Actor.fail(message);
        return;
    }

    await Actor.exit();
}

main().catch((error) => {
    const message = error instanceof Error ? error.message : String(error);
    console.error('Actor failed: ' + message);
    process.exitCode = 1;
});

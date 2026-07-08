import { Actor } from 'apify';
import { pushChargedItems } from './charging.js';
import {
    buildJamedaReviewsParams,
    buildJamedaReviewsPlan,
    describeJamedaReviewsRequest,
} from './request-params.js';
import type { JamedaReviewsInput } from './request-params.js';
import {
    buildJamedaReviewDatasetItem,
    buildJamedaReviewsOutputSummary,
    getJamedaReviews,
} from './response-utils.js';
import type { JamedaReviewsResponse } from './response-utils.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 90000;
const SCRAPPA_MAX_ATTEMPTS = 3;

function formatErrorMessage(error: unknown): string {
    const rawMessage = error instanceof Error ? error.message : String(error);
    return error instanceof ScrappaTimeoutError
        ? `${rawMessage}. The Jameda reviews request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try fewer doctor URLs or run the request again.`
        : rawMessage;
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<JamedaReviewsInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const plan = buildJamedaReviewsPlan(input);
        console.log(`Fetching Jameda reviews for ${describeJamedaReviewsRequest(plan)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const failures: Record<string, string>[] = [...plan.inputFailures];
        let savedReviews = 0;
        let statusMessage: string | null = null;

        for (const doctorUrl of plan.doctorUrls) {
            const params = buildJamedaReviewsParams(plan, doctorUrl);
            console.log(`Fetching Jameda reviews for ${doctorUrl}`);

            try {
                const response = await client.get<JamedaReviewsResponse>('/jameda/reviews', params, {
                    attempts: SCRAPPA_MAX_ATTEMPTS,
                });
                const reviews = getJamedaReviews(response);
                const items = reviews.map((review) => buildJamedaReviewDatasetItem(review, {
                    inputDoctorUrl: doctorUrl,
                    params,
                    response,
                }));

                const result = await pushChargedItems({
                    isPayPerEvent: () => Actor.getChargingManager().getPricingInfo().isPayPerEvent,
                    pushData: (itemsToPush, eventName) => eventName === undefined
                        ? Actor.pushData(itemsToPush)
                        : Actor.pushData(itemsToPush, eventName),
                }, items);
                savedReviews += result.savedCount;
                console.log(`Found ${reviews.length} Jameda review result(s) for ${doctorUrl}; saved ${result.savedCount}`);

                if (result.statusMessage) {
                    statusMessage = result.statusMessage;
                    break;
                }
            } catch (error) {
                const message = formatErrorMessage(error);
                failures.push({ doctor_url: doctorUrl, error: message });
                console.error(`Failed to fetch Jameda reviews for ${doctorUrl}: ${message}`);
            }
        }

        if (!statusMessage && failures.length > 0) {
            statusMessage = `${failures.length} Jameda review request(s) failed; ${savedReviews} review(s) saved.`;
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', buildJamedaReviewsOutputSummary({
            doctorUrls: plan.doctorUrls,
            savedReviews,
            failures,
            statusMessage,
        }));

        console.log('Jameda reviews extraction completed successfully');
        console.log('Results summary:', JSON.stringify({
            doctors_requested: plan.doctorUrls.length,
            reviews_saved: savedReviews,
            requests_failed: failures.length,
        }));

        if (savedReviews === 0 && failures.length > 0) {
            await Actor.fail(statusMessage ?? 'No Jameda reviews were saved.');
            return;
        }

        if (statusMessage) {
            await Actor.exit({ statusMessage });
            return;
        }
    } catch (error) {
        const message = formatErrorMessage(error);
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

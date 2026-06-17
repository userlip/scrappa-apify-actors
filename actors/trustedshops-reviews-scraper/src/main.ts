import { Actor } from 'apify';
import {
    buildPageParams,
    buildTrustedShopsReviewsPlan,
    describeTrustedShopsReviewsRequest,
} from './request-params.js';
import { getSavedCount } from './charging.js';
import type { TrustedShopsReviewsInput } from './request-params.js';
import {
    collectReviews,
    enrichReview,
    hasNextPage,
} from './response-utils.js';
import type { TrustedShopsReviewsResponse } from './response-utils.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 90000;
const SCRAPPA_MAX_ATTEMPTS = 3;
const REVIEW_RESULT_CHARGE_EVENT = 'review-result';

function getChargeableReviewCapacity(): number {
    const chargingManager = Actor.getChargingManager();
    const { isPayPerEvent } = chargingManager.getPricingInfo();

    if (!isPayPerEvent) {
        return Infinity;
    }

    return chargingManager.calculateMaxEventChargeCountWithinLimit(REVIEW_RESULT_CHARGE_EVENT);
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<TrustedShopsReviewsInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const plan = buildTrustedShopsReviewsPlan(input);
        console.log(`Fetching Trusted Shops reviews for ${describeTrustedShopsReviewsRequest(plan)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const responses: Array<{
            tsid: string;
            page: number;
            count: number;
            response?: TrustedShopsReviewsResponse;
        }> = [];
        let reviewsExtracted = 0;

        for (const target of plan.targets) {
            for (let offset = 0; offset < plan.maxPages; offset += 1) {
                const page = plan.startPage + offset;
                const params = buildPageParams(plan, page);

                const chargeableReviewCapacity = getChargeableReviewCapacity();
                if (chargeableReviewCapacity <= 0) {
                    const statusMessage = `Charge limit reached before fetching Trusted Shops reviews page ${page} for ${target.tsid}.`;
                    console.log(statusMessage, JSON.stringify({
                        event: REVIEW_RESULT_CHARGE_EVENT,
                        saved_count: reviewsExtracted,
                        tsid: target.tsid,
                        page,
                    }));
                    await Actor.exit({ statusMessage });
                    return;
                }

                console.log(`Fetching Trusted Shops reviews page ${page} for ${target.tsid}`);

                const response = await client.get<TrustedShopsReviewsResponse>(
                    `/trustedshops/reviews/${target.tsid}`,
                    params,
                    { attempts: SCRAPPA_MAX_ATTEMPTS },
                );
                const reviews = collectReviews(response);

                responses.push({
                    tsid: target.tsid,
                    page,
                    count: reviews.length,
                    ...(plan.includeRawResponses ? { response } : {}),
                });

                if (reviews.length > 0) {
                    const enrichedReviews = reviews.map((review) => enrichReview(review, target, params, response));
                    let savedCount = enrichedReviews.length;

                    const { isPayPerEvent } = Actor.getChargingManager().getPricingInfo();
                    if (isPayPerEvent) {
                        const chargeResult = await Actor.pushData(enrichedReviews, REVIEW_RESULT_CHARGE_EVENT);
                        savedCount = getSavedCount(chargeResult, enrichedReviews.length);

                        if (chargeResult.eventChargeLimitReached) {
                            const statusMessage = `Charge limit reached after saving ${savedCount} of ${enrichedReviews.length} Trusted Shops reviews for ${target.tsid} page ${page}.`;
                            console.log(statusMessage, JSON.stringify({
                                event: REVIEW_RESULT_CHARGE_EVENT,
                                saved_count: savedCount,
                                requested_count: enrichedReviews.length,
                                tsid: target.tsid,
                                page,
                            }));
                            await Actor.exit({ statusMessage });
                            return;
                        }
                    } else {
                        await Actor.pushData(enrichedReviews);
                    }

                    reviewsExtracted += savedCount;
                    console.log(`Found ${reviews.length} Trusted Shops review(s) on page ${page}; saved ${savedCount}`);
                } else {
                    console.log(`No Trusted Shops reviews found on page ${page} for ${target.tsid}`);
                    break;
                }

                if (hasNextPage(response, page) === false) {
                    console.log(`Stopping after page ${page}; Scrappa reported no next page`);
                    break;
                }
            }
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', {
            request: {
                targets: plan.targets,
                ...plan.baseParams,
                start_page: plan.startPage,
                max_pages: plan.maxPages,
            },
            pages_fetched: responses.length,
            reviews_extracted: reviewsExtracted,
            responses,
        });

        console.log('Trusted Shops reviews extraction completed successfully');
        console.log('Results summary:', JSON.stringify({
            targets: plan.targets.map((target) => target.tsid),
            pages_fetched: responses.length,
            reviews_extracted: reviewsExtracted,
        }));
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The Trusted Shops reviews request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try fewer pages, fewer TSIDs, or run the request again.`
            : rawMessage;
        console.error('Actor failed: ' + message);
        await Actor.fail(message);
        return;
    }

    await Actor.exit();
}

main();

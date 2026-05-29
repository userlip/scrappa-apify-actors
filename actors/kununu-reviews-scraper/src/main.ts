import { Actor } from 'apify';
import {
    buildKununuReviewsPlan,
    buildPageParams,
    describeKununuReviewsRequest,
} from './request-params.js';
import { getSavedCount } from './charging.js';
import type { KununuReviewsInput } from './request-params.js';
import {
    collectReviews,
    enrichReview,
} from './review-processing.js';
import type { KununuPagination, KununuReviewsResponse } from './review-processing.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 90000;
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

        const input = await Actor.getInput<KununuReviewsInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const plan = buildKununuReviewsPlan(input);
        console.log(`Fetching Kununu reviews for ${describeKununuReviewsRequest(plan)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const responses: Array<{
            target: string;
            page: number;
            response?: KununuReviewsResponse;
            pagination?: KununuPagination;
            count: number;
        }> = [];
        let reviewsExtracted = 0;

        for (const target of plan.targets) {
            for (let offset = 0; offset < plan.maxPages; offset++) {
                const page = plan.startPage + offset;
                const params = buildPageParams(plan, target, page);

                const chargeableReviewCapacity = getChargeableReviewCapacity();
                if (chargeableReviewCapacity <= 0) {
                    const statusMessage = `Charge limit reached before fetching Kununu reviews page ${page} for ${target.country}/${target.company_slug}.`;
                    console.log(statusMessage, JSON.stringify({
                        event: REVIEW_RESULT_CHARGE_EVENT,
                        saved_count: reviewsExtracted,
                        target: `${target.country}/${target.company_slug}`,
                        page,
                    }));
                    await Actor.exit({ statusMessage });
                    return;
                }

                console.log(`Fetching Kununu reviews page ${page} for ${target.country}/${target.company_slug}`);

                const response = await client.get<KununuReviewsResponse>('/kununu/reviews', params);

                const reviews = collectReviews(response);
                responses.push({
                    target: `${target.country}/${target.company_slug}`,
                    page,
                    count: reviews.length,
                    pagination: response.meta?.pagination,
                    ...(plan.includeRawResponses ? { response } : {}),
                });

                if (reviews.length > 0) {
                    const enrichedReviews = reviews.map((review) => enrichReview(review, target, params, response, {
                        includeRawReview: plan.includeRawReview,
                    }));
                    let savedCount = enrichedReviews.length;

                    const { isPayPerEvent } = Actor.getChargingManager().getPricingInfo();
                    if (isPayPerEvent) {
                        const chargeResult = await Actor.pushData(enrichedReviews, REVIEW_RESULT_CHARGE_EVENT);
                        savedCount = getSavedCount(chargeResult, enrichedReviews.length);

                        if (chargeResult.eventChargeLimitReached) {
                            const statusMessage = `Charge limit reached after saving ${savedCount} of ${enrichedReviews.length} Kununu reviews for ${target.country}/${target.company_slug} page ${page}.`;
                            console.log(statusMessage, JSON.stringify({
                                event: REVIEW_RESULT_CHARGE_EVENT,
                                saved_count: savedCount,
                                requested_count: enrichedReviews.length,
                                target: `${target.country}/${target.company_slug}`,
                                page,
                            }));
                            await Actor.exit({ statusMessage });
                            return;
                        }
                    } else {
                        await Actor.pushData(enrichedReviews);
                    }

                    reviewsExtracted += savedCount;
                    console.log(`Found ${reviews.length} reviews on page ${page}`);
                } else {
                    console.log(`No reviews found on page ${page}`);
                }

                const pagination = response.meta?.pagination;
                if (pagination?.totalPages !== undefined && page >= Number(pagination.totalPages)) {
                    console.log(`Stopping after page ${page}; Scrappa reported ${pagination.totalPages} total page(s)`);
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

        const summary = {
            targets: plan.targets.map((target) => `${target.country}/${target.company_slug}`),
            pages_fetched: responses.length,
            reviews_extracted: reviewsExtracted,
        };

        console.log('Kununu reviews extraction completed successfully');
        console.log('Results summary:', JSON.stringify(summary));
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The Kununu reviews request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try fewer pages or run the request again.`
            : rawMessage;
        console.error('Actor failed: ' + message);
        await Actor.fail(message);
        return;
    }

    await Actor.exit();
}

main();

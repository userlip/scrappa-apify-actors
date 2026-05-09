import { Actor } from 'apify';
import {
    buildPageParams,
    buildTrustpilotCompanyReviewsPlan,
    describeTrustpilotCompanyReviewsRequest,
} from './request-params.js';
import type { TrustpilotCompanyReviewsInput } from './request-params.js';
import {
    collectReviews,
    enrichReview,
} from './review-processing.js';
import type { TrustpilotCompanyReviewsResponse } from './review-processing.js';
import { ScrappaClient, ScrappaTimeoutError } from './shared/index.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 90000;

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<TrustpilotCompanyReviewsInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const plan = buildTrustpilotCompanyReviewsPlan(input);
        console.log(`Fetching Trustpilot company reviews for ${describeTrustpilotCompanyReviewsRequest(plan)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const responses: TrustpilotCompanyReviewsResponse[] = [];
        const allReviews: Record<string, unknown>[] = [];

        for (let offset = 0; offset < plan.maxPages; offset += 1) {
            const page = plan.startPage + offset;
            const params = buildPageParams(plan, page);
            console.log(`Fetching Trustpilot reviews page ${page} for ${String(params.company_domain)}`);

            const response = await client.get<TrustpilotCompanyReviewsResponse>('/trustpilot/company-reviews', params);
            responses.push(response);

            const reviews = collectReviews(response);
            if (reviews.length > 0) {
                const enrichedReviews = reviews.map(({ review, source }) => enrichReview(review, params, response, source));
                allReviews.push(...enrichedReviews);
                await Actor.pushData(enrichedReviews);
                console.log(`Found ${reviews.length} reviews on page ${page}`);
            } else {
                console.log(`No reviews found on page ${page}`);
            }

            if (response.pagination?.has_next_page === false) {
                console.log(`Stopping after page ${page}; Scrappa reported no next page`);
                break;
            }
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', {
            request: {
                ...plan.baseParams,
                start_page: plan.startPage,
                max_pages: plan.maxPages,
            },
            pages_fetched: responses.length,
            reviews_extracted: allReviews.length,
            responses,
        });

        const lastResponse = responses[responses.length - 1];
        const summary = {
            company_domain: plan.baseParams.company_domain,
            pages_fetched: responses.length,
            reviews_extracted: allReviews.length,
            total_pages: lastResponse?.pagination?.total_pages ?? null,
            total_count: lastResponse?.pagination?.total_count ?? null,
            has_next_page: lastResponse?.pagination?.has_next_page ?? false,
        };

        console.log('Trustpilot company reviews extraction completed successfully');
        console.log('Results summary:', JSON.stringify(summary));
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = error instanceof ScrappaTimeoutError
            ? `${rawMessage}. The Trustpilot reviews request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try fewer pages or run the request again.`
            : rawMessage;
        console.error('Actor failed: ' + message);
        await Actor.fail(message);
        return;
    }

    await Actor.exit();
}

main();

import { Actor } from 'apify';
import {
    buildPageParams,
    buildTrustpilotCompanyReviewsPlan,
    describeTrustpilotCompanyReviewsRequest,
} from './request-params.js';
import type { TrustpilotCompanyReviewsInput } from './request-params.js';
import { ScrappaClient } from './shared/index.js';

interface TrustpilotReviewConsumer {
    displayName?: string;
    consumerId?: string;
    profileUrl?: string;
    [key: string]: unknown;
}

interface TrustpilotReview {
    id?: string;
    title?: string;
    text?: string;
    rating?: number;
    stars?: number;
    language?: string;
    dates?: {
        publishedDate?: string;
        experiencedDate?: string;
        updatedDate?: string;
        [key: string]: unknown;
    };
    createdAt?: number | string;
    consumer?: TrustpilotReviewConsumer;
    labels?: Record<string, unknown>;
    isVerified?: boolean | null;
    reply?: unknown;
    [key: string]: unknown;
}

interface TrustpilotPagination {
    current_page?: number;
    total_pages?: number;
    total_count?: number;
    per_page?: number;
    has_next_page?: boolean;
    has_previous_page?: boolean;
    [key: string]: unknown;
}

interface TrustpilotCompanyReviewsResponse {
    reviews?: TrustpilotReview[];
    relevantReviews?: TrustpilotReview[];
    aiSummaryReviews?: TrustpilotReview[];
    businessUnit?: Record<string, unknown>;
    filters?: Record<string, unknown>;
    pagination?: TrustpilotPagination;
    meta?: Record<string, unknown>;
    [key: string]: unknown;
}

const SCRAPPA_REQUEST_TIMEOUT_MS = 90000;

function consumerName(consumer: TrustpilotReviewConsumer | undefined): string | undefined {
    return consumer?.displayName;
}

function publishedDate(review: TrustpilotReview): string | undefined {
    if (review.dates?.publishedDate) {
        return review.dates.publishedDate;
    }

    if (typeof review.createdAt === 'number') {
        return new Date(review.createdAt * 1000).toISOString();
    }

    if (typeof review.createdAt === 'string' && review.createdAt.trim() !== '') {
        return review.createdAt;
    }

    return undefined;
}

function enrichReview(
    review: TrustpilotReview,
    requestParams: Record<string, unknown>,
    response: TrustpilotCompanyReviewsResponse,
): Record<string, unknown> {
    return {
        ...review,
        consumer_name: consumerName(review.consumer),
        published_date: publishedDate(review),
        company_domain: requestParams.company_domain,
        request_locale: requestParams.locale ?? null,
        request_page: requestParams.page,
        request_sort: requestParams.sort ?? null,
        request_rating: requestParams.rating ?? null,
        request_verified: requestParams.verified ?? null,
        request_with_replies: requestParams.with_replies ?? null,
        request_query: requestParams.query ?? null,
        request_date_posted: requestParams.date_posted ?? null,
        page_total_count: response.pagination?.total_count ?? null,
        page_total_pages: response.pagination?.total_pages ?? null,
    };
}

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

            const reviews = response.reviews ?? [];
            if (reviews.length > 0) {
                const enrichedReviews = reviews.map((review) => enrichReview(review, params, response));
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
        const message = rawMessage.includes('timed out')
            ? `${rawMessage}. The Trustpilot reviews request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try fewer pages or run the request again.`
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

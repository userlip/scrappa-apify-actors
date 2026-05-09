export interface TrustpilotReviewConsumer {
    displayName?: string;
    consumerId?: string;
    profileUrl?: string;
    [key: string]: unknown;
}

export interface TrustpilotReview {
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

export interface TrustpilotPagination {
    current_page?: number;
    total_pages?: number;
    total_count?: number;
    per_page?: number;
    has_next_page?: boolean;
    has_previous_page?: boolean;
    [key: string]: unknown;
}

export interface TrustpilotCompanyReviewsResponse {
    reviews?: TrustpilotReview[];
    relevantReviews?: TrustpilotReview[];
    aiSummaryReviews?: TrustpilotReview[];
    businessUnit?: Record<string, unknown>;
    filters?: Record<string, unknown>;
    pagination?: TrustpilotPagination;
    meta?: Record<string, unknown>;
    [key: string]: unknown;
}

const REVIEW_ARRAYS = ['reviews', 'relevantReviews', 'aiSummaryReviews'] as const;
export type ReviewArrayName = typeof REVIEW_ARRAYS[number];

function consumerName(consumer: TrustpilotReviewConsumer | undefined): string | undefined {
    return consumer?.displayName;
}

export function publishedDate(review: TrustpilotReview): string | undefined {
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

export function enrichReview(
    review: TrustpilotReview,
    requestParams: Record<string, unknown>,
    response: TrustpilotCompanyReviewsResponse,
    reviewSource: ReviewArrayName,
): Record<string, unknown> {
    return {
        ...review,
        review_source: reviewSource,
        consumer_name: consumerName(review.consumer),
        published_date: publishedDate(review),
        company_domain: requestParams.company_domain,
        request_locale: requestParams.locale,
        request_page: requestParams.page,
        request_sort: requestParams.sort ?? 'recency',
        request_rating: requestParams.rating ?? null,
        request_verified: requestParams.verified ?? null,
        request_with_replies: requestParams.with_replies ?? null,
        request_query: requestParams.query ?? null,
        request_date_posted: requestParams.date_posted ?? 'any',
        page_total_count: response.pagination?.total_count ?? null,
        page_total_pages: response.pagination?.total_pages ?? null,
    };
}

export function collectReviews(response: TrustpilotCompanyReviewsResponse): Array<{
    review: TrustpilotReview;
    source: ReviewArrayName;
}> {
    const seen = new Set<string>();
    const collected: Array<{ review: TrustpilotReview; source: ReviewArrayName }> = [];

    for (const source of REVIEW_ARRAYS) {
        const reviews = response[source] ?? [];
        for (const review of reviews) {
            const id = typeof review.id === 'string' ? review.id.trim() : undefined;
            if (id && seen.has(id)) {
                continue;
            }

            if (id) {
                seen.add(id);
            }

            collected.push({ review, source });
        }
    }

    return collected;
}

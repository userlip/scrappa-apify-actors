import type { TrustedShopsTarget } from './request-params.js';

export interface TrustedShopsReview {
    id?: string;
    reviewId?: string;
    review_id?: string;
    title?: string;
    reviewTitle?: string;
    text?: string;
    reviewText?: string;
    comment?: string;
    rating?: number;
    ratingValue?: number;
    stars?: number;
    createdAt?: number | string;
    created_at?: string;
    submittedAt?: string;
    date?: string;
    verified?: boolean;
    isVerified?: boolean;
    verifiedReview?: boolean;
    verificationStatus?: string;
    criteria?: unknown;
    ratings?: unknown;
    shopName?: string;
    shop_name?: string;
    consumer?: unknown;
    [key: string]: unknown;
}

export interface TrustedShopsReviewsResponse {
    reviews?: TrustedShopsReview[];
    data?: TrustedShopsReview[] | {
        reviews?: TrustedShopsReview[];
        items?: TrustedShopsReview[];
        data?: TrustedShopsReview[];
        shop?: {
            tsId?: string;
            name?: string;
            shopName?: string;
            reviews?: TrustedShopsReview[];
            reviewCount?: number;
            [key: string]: unknown;
        };
        [key: string]: unknown;
    };
    response?: {
        code?: number;
        data?: {
            shop?: {
                tsId?: string;
                name?: string;
                shopName?: string;
                reviews?: TrustedShopsReview[];
                reviewCount?: number;
                [key: string]: unknown;
            };
            reviews?: TrustedShopsReview[];
            items?: TrustedShopsReview[];
            [key: string]: unknown;
        };
        [key: string]: unknown;
    };
    items?: TrustedShopsReview[];
    shop?: {
        name?: string;
        shopName?: string;
        [key: string]: unknown;
    };
    pagination?: {
        current_page?: number;
        total_pages?: number;
        totalPages?: number;
        total_count?: number;
        totalCount?: number;
        has_next_page?: boolean;
        hasNextPage?: boolean;
        [key: string]: unknown;
    };
    meta?: {
        pagination?: TrustedShopsReviewsResponse['pagination'];
        total_pages?: number;
        totalPages?: number;
        total_count?: number;
        totalCount?: number;
        [key: string]: unknown;
    };
    metaData?: {
        totalPageCount?: number;
        totalReviewCount?: number;
        [key: string]: unknown;
    };
    [key: string]: unknown;
}

export interface EnrichReviewOptions {
    includeRawReview?: boolean;
}

function wrappedData(response: TrustedShopsReviewsResponse): Exclude<TrustedShopsReviewsResponse['data'], TrustedShopsReview[] | undefined> | undefined {
    return response.data && !Array.isArray(response.data) ? response.data : undefined;
}

export function collectReviews(response: TrustedShopsReviewsResponse): TrustedShopsReview[] {
    const containers = [
        response.reviews,
        response.items,
        Array.isArray(response.data) ? response.data : undefined,
        wrappedData(response)?.reviews,
        wrappedData(response)?.items,
        wrappedData(response)?.data,
        wrappedData(response)?.shop?.reviews,
        response.response?.data?.reviews,
        response.response?.data?.items,
        response.response?.data?.shop?.reviews,
    ];

    const seen = new Set<string>();
    const reviews: TrustedShopsReview[] = [];

    for (const container of containers) {
        if (!Array.isArray(container)) {
            continue;
        }

        for (const review of container) {
            const dedupeKey = reviewDedupeKey(review);
            if (dedupeKey && seen.has(dedupeKey)) {
                continue;
            }

            if (dedupeKey) {
                seen.add(dedupeKey);
            }

            reviews.push(review);
        }
    }

    return reviews;
}

function reviewId(review: TrustedShopsReview): string | undefined {
    const id = review.review_id ?? review.reviewId ?? review.id;
    return typeof id === 'string' && id.trim() !== '' ? id.trim() : undefined;
}

function reviewDedupeKey(review: TrustedShopsReview): string | undefined {
    const id = reviewId(review);
    if (id) {
        return `id:${id}`;
    }

    const title = reviewTitle(review);
    const text = reviewText(review);
    const date = reviewDate(review);
    const rating = reviewRating(review);
    const verificationStatus = typeof review.verificationStatus === 'string'
        ? review.verificationStatus.trim().toUpperCase()
        : null;

    if (!title && !text && !date) {
        return undefined;
    }

    return JSON.stringify({
        rating,
        title,
        text,
        date,
        verificationStatus,
    });
}

function reviewTitle(review: TrustedShopsReview): string | null {
    const title = review.reviewTitle ?? review.title;
    return typeof title === 'string' && title.trim() !== '' ? title.trim() : null;
}

function reviewText(review: TrustedShopsReview): string | null {
    const text = review.reviewText ?? review.text ?? review.comment;
    return typeof text === 'string' && text.trim() !== '' ? text.trim() : null;
}

function reviewRating(review: TrustedShopsReview): number | null {
    const rating = review.rating ?? review.ratingValue ?? review.stars;
    return typeof rating === 'number' ? rating : null;
}

function reviewDate(review: TrustedShopsReview): string | null {
    const date = review.created_at ?? review.createdAt ?? review.submittedAt ?? review.date;
    if (typeof date === 'string' && date.trim() !== '') {
        return date.trim();
    }

    if (typeof date === 'number') {
        const millis = date > 100000000000 ? date : date * 1000;
        return new Date(millis).toISOString();
    }

    return null;
}

function reviewVerified(review: TrustedShopsReview): boolean | null {
    const verified = review.verified ?? review.isVerified ?? review.verifiedReview;
    if (typeof verified === 'boolean') {
        return verified;
    }

    if (typeof review.verificationStatus === 'string') {
        const status = review.verificationStatus.trim().toUpperCase();
        if (['MEMBER_VERIFIED', 'VERIFIED'].includes(status)) {
            return true;
        }

        if (['UNVERIFIED', 'NOT_VERIFIED', 'NOT_MEMBER_VERIFIED'].includes(status)) {
            return false;
        }
    }

    return null;
}

function shopName(review: TrustedShopsReview, response: TrustedShopsReviewsResponse): string | null {
    const data = wrappedData(response);
    const name = review.shop_name
        ?? review.shopName
        ?? response.shop?.name
        ?? response.shop?.shopName
        ?? data?.shop?.name
        ?? data?.shop?.shopName
        ?? response.response?.data?.shop?.name
        ?? response.response?.data?.shop?.shopName;

    return typeof name === 'string' && name.trim() !== '' ? name.trim() : null;
}

function totalPages(response: TrustedShopsReviewsResponse): number | null {
    const value = response.pagination?.total_pages
        ?? response.pagination?.totalPages
        ?? response.meta?.pagination?.total_pages
        ?? response.meta?.pagination?.totalPages
        ?? response.meta?.total_pages
        ?? response.meta?.totalPages
        ?? response.metaData?.totalPageCount;

    return typeof value === 'number' ? value : null;
}

function totalReviews(response: TrustedShopsReviewsResponse): number | null {
    const value = response.pagination?.total_count
        ?? response.pagination?.totalCount
        ?? response.meta?.pagination?.total_count
        ?? response.meta?.pagination?.totalCount
        ?? response.meta?.total_count
        ?? response.meta?.totalCount
        ?? response.metaData?.totalReviewCount
        ?? wrappedData(response)?.shop?.reviewCount
        ?? response.response?.data?.shop?.reviewCount;

    return typeof value === 'number' ? value : null;
}

export function hasNextPage(response: TrustedShopsReviewsResponse, page: number): boolean | null {
    const explicit = response.pagination?.has_next_page
        ?? response.pagination?.hasNextPage
        ?? response.meta?.pagination?.has_next_page
        ?? response.meta?.pagination?.hasNextPage;

    if (typeof explicit === 'boolean') {
        return explicit;
    }

    const pages = totalPages(response);
    return pages === null ? null : page < pages;
}

export function enrichReview(
    review: TrustedShopsReview,
    target: TrustedShopsTarget,
    requestParams: Record<string, unknown>,
    response: TrustedShopsReviewsResponse,
    options: EnrichReviewOptions = {},
): Record<string, unknown> {
    const item: Record<string, unknown> = {
        ...review,
        tsid: target.tsid,
        shop_name: shopName(review, response),
        rating: reviewRating(review),
        review_text: reviewText(review),
        review_title: reviewTitle(review),
        created_at: reviewDate(review),
        verified: reviewVerified(review),
        criteria: review.criteria ?? review.ratings ?? null,
        review_id: reviewId(review) ?? null,
        page: requestParams.page,
        size: requestParams.size,
        source_url: target.sourceUrl,
        input: target.input,
        request_market: requestParams.market ?? null,
        page_total_reviews: totalReviews(response),
        page_total_pages: totalPages(response),
    };

    if (options.includeRawReview) {
        item.raw_review = review;
    }

    return item;
}

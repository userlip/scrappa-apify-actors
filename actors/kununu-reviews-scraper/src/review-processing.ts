import type { KununuTarget } from './request-params.js';

export interface KununuReviewText {
    id?: string;
    text?: string;
    [key: string]: unknown;
}

export interface KununuReview {
    uuid?: string;
    type?: string;
    title?: string;
    score?: number;
    roundedScore?: number;
    createdAt?: string;
    updatedAt?: string;
    texts?: KununuReviewText[];
    company?: {
        uuid?: string;
        name?: string;
        slug?: string;
        location?: unknown;
        [key: string]: unknown;
    };
    position?: string;
    department?: string;
    employmentType?: string;
    employmentStatus?: string;
    isCurrentEmployee?: boolean;
    isRecommended?: boolean;
    jobStatus?: string;
    city?: string;
    jobTitle?: string;
    responses?: unknown[];
    ratings?: unknown[];
    [key: string]: unknown;
}

export interface KununuPagination {
    currentPage?: number;
    totalPages?: number;
    resultsStart?: number;
    resultsEnd?: number;
    totalResults?: number;
    [key: string]: unknown;
}

export interface KununuReviewsResponse {
    success?: boolean;
    data?: KununuReview[];
    meta?: {
        pagination?: KununuPagination;
        filters?: unknown;
        cached?: boolean;
        duration_ms?: number;
        [key: string]: unknown;
    };
    [key: string]: unknown;
}

function joinedReviewText(texts: KununuReviewText[] | undefined): string | null {
    if (!Array.isArray(texts) || texts.length === 0) {
        return null;
    }

    const parts = texts
        .map((item) => item.text)
        .filter((text): text is string => typeof text === 'string' && text.trim() !== '')
        .map((text) => text.trim());

    return parts.length > 0 ? parts.join('\n\n') : null;
}

export function collectReviews(response: KununuReviewsResponse): KununuReview[] {
    if (!Array.isArray(response.data)) {
        return [];
    }

    const seen = new Set<string>();
    const reviews: KununuReview[] = [];

    for (const review of response.data) {
        const id = typeof review.uuid === 'string' ? review.uuid.trim() : undefined;
        if (id && seen.has(id)) {
            continue;
        }

        if (id) {
            seen.add(id);
        }

        reviews.push(review);
    }

    return reviews;
}

export function enrichReview(
    review: KununuReview,
    target: KununuTarget,
    requestParams: Record<string, unknown>,
    response: KununuReviewsResponse,
): Record<string, unknown> {
    const pagination = response.meta?.pagination;
    const sourceUrl = `https://www.kununu.com/${target.country}/${target.company_slug}`;

    return {
        review_id: review.uuid ?? null,
        company_target: target.input,
        company_country: target.country,
        company_slug: target.company_slug,
        company_id: target.company_id ?? review.company?.uuid ?? null,
        company_name: review.company?.name ?? null,
        rating: review.score ?? null,
        rounded_rating: review.roundedScore ?? null,
        title: review.title ?? null,
        text: joinedReviewText(review.texts),
        date: review.createdAt ?? null,
        updated_at: review.updatedAt ?? null,
        review_type: review.type ?? requestParams.review_type ?? 'employees',
        reviewer_position: review.position ?? null,
        reviewer_department: review.department ?? null,
        reviewer_employment_type: review.employmentType ?? null,
        reviewer_employment_status: review.employmentStatus ?? review.jobStatus ?? null,
        reviewer_current_employee: review.isCurrentEmployee ?? null,
        reviewer_recommended: review.isRecommended ?? null,
        reviewer_city: review.city ?? null,
        reviewer_job_title: review.jobTitle ?? null,
        texts: review.texts ?? [],
        ratings: review.ratings ?? [],
        responses: review.responses ?? [],
        page: requestParams.page,
        source_url: sourceUrl,
        request_sort: requestParams.sort ?? null,
        request_score_filters: requestParams.score_filters ?? null,
        request_recommended_filters: requestParams.recommended_filters ?? null,
        request_jobstatus_filters: requestParams.jobstatus_filters ?? null,
        request_position_filters: requestParams.position_filters ?? null,
        request_department_filters: requestParams.department_filters ?? null,
        request_response_filters: requestParams.response_filters ?? null,
        request_date_filters: requestParams.date_filters ?? null,
        page_total_results: pagination?.totalResults ?? null,
        page_total_pages: pagination?.totalPages ?? null,
        raw_review: review,
    };
}

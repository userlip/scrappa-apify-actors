export interface JamedaReview {
    id?: string | number | null;
    text?: string | null;
    rating?: string | number | null;
    date?: string | null;
    date_formatted?: string | null;
    verification_badge?: string | null;
    categories?: unknown[];
    [key: string]: unknown;
}

export interface JamedaReviewsResponse {
    success?: boolean;
    message?: string | null;
    data?: JamedaReview[];
    meta?: {
        url?: string | null;
        doctor_url?: string | null;
        filters?: {
            sort?: string | null;
            rating?: string | null;
            [key: string]: unknown;
        };
        pagination?: {
            currentPage?: number | null;
            totalPages?: number | null;
            totalReviews?: number | null;
            resultsPerPage?: number | null;
            hasNextPage?: boolean | null;
            hasPreviousPage?: boolean | null;
            [key: string]: unknown;
        };
        doctor?: {
            name?: string | null;
            specializations?: string | null;
            overall_rating?: string | number | null;
            [key: string]: unknown;
        };
        duration_ms?: number;
        source?: string | null;
        [key: string]: unknown;
    };
    [key: string]: unknown;
}

export interface JamedaReviewsDatasetContext {
    inputDoctorUrl: string;
    params: Record<string, unknown>;
    response: JamedaReviewsResponse;
}

export interface JamedaReviewsOutputSummaryContext {
    doctorUrls: string[];
    savedReviews: number;
    failures: Record<string, string>[];
    statusMessage: string | null;
}

function toDecimalNumber(value: unknown): number | null {
    if (typeof value === 'number' && Number.isFinite(value)) {
        return value;
    }

    if (typeof value !== 'string') {
        return null;
    }

    const normalized = value.trim().replace(',', '.');
    if (normalized === '') {
        return null;
    }

    const parsed = Number(normalized);
    return Number.isFinite(parsed) ? parsed : null;
}

function firstNonEmptyString(...values: unknown[]): string | null {
    for (const value of values) {
        if (typeof value === 'string' && value.trim() !== '') {
            return value.trim();
        }
    }

    return null;
}

export function getJamedaReviews(response: JamedaReviewsResponse): JamedaReview[] {
    if (response.success === false) {
        throw new Error(response.message ?? 'Scrappa Jameda reviews request was not successful');
    }

    return Array.isArray(response.data) ? response.data : [];
}

export function buildJamedaReviewDatasetItem(
    review: JamedaReview,
    context: JamedaReviewsDatasetContext,
): Record<string, unknown> {
    const pagination = context.response.meta?.pagination ?? {};
    const doctor = context.response.meta?.doctor ?? {};

    return {
        ...review,
        review_id: review.id ?? null,
        review_text: review.text ?? null,
        rating: review.rating ?? null,
        rating_number: toDecimalNumber(review.rating),
        date: review.date ?? null,
        date_formatted: review.date_formatted ?? null,
        verification_badge: review.verification_badge ?? null,
        categories: review.categories ?? [],
        doctor_name: firstNonEmptyString(doctor.name),
        doctor_specializations: doctor.specializations ?? null,
        doctor_overall_rating: doctor.overall_rating ?? null,
        doctor_overall_rating_number: toDecimalNumber(doctor.overall_rating),
        input_doctor_url: context.inputDoctorUrl,
        normalized_doctor_url: context.params.doctor_url ?? null,
        request_page: context.params.page ?? null,
        request_sort: context.params.sort ?? null,
        request_rating: context.params.rating ?? null,
        request_per_page: context.params.per_page ?? null,
        response_url: context.response.meta?.url ?? null,
        response_doctor_url: context.response.meta?.doctor_url ?? null,
        total_reviews: pagination.totalReviews ?? null,
        total_pages: pagination.totalPages ?? null,
        has_next_page: pagination.hasNextPage ?? null,
        response_source: context.response.meta?.source ?? null,
    };
}

export function buildJamedaReviewsOutputSummary(
    context: JamedaReviewsOutputSummaryContext,
): Record<string, unknown> {
    return {
        request: {
            endpoint: '/jameda/reviews',
            doctor_urls: context.doctorUrls,
        },
        doctors_requested: context.doctorUrls.length,
        reviews_saved: context.savedReviews,
        requests_failed: context.failures.length,
        status_message: context.statusMessage,
        failures: context.failures,
    };
}

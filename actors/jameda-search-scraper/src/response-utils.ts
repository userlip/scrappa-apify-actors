export interface JamedaSearchResponse {
    success?: boolean;
    data?: JamedaDoctor[];
    meta?: {
        page?: number;
        per_page?: number;
        results_count?: number;
        total_results?: number | null;
        total_pages?: number | null;
        has_next_page?: boolean | null;
        has_previous_page?: boolean | null;
        location_info?: unknown;
        duration_ms?: number;
        [key: string]: unknown;
    };
    [key: string]: unknown;
}

export interface JamedaDoctor {
    name?: string | null;
    specialty?: string | null;
    url?: string | null;
    rating?: string | null;
    review_count?: string | null;
    address?: string | null;
    image_url?: string | null;
    jameda_rating?: {
        rating?: string | null;
        count?: string | null;
        [key: string]: unknown;
    } | null;
    [key: string]: unknown;
}

const JAMEDA_BASE_URL = 'https://www.jameda.de';

export function getJamedaDoctors(response: JamedaSearchResponse): JamedaDoctor[] {
    return Array.isArray(response.data) ? response.data : [];
}

function normalizeJamedaUrl(url: string | null | undefined): string | null {
    if (!url) {
        return null;
    }

    if (/^https?:\/\//i.test(url)) {
        return url.replace(/^http:\/\//i, 'https://');
    }

    return `${JAMEDA_BASE_URL}/${url.replace(/^\/+/, '')}`;
}

function parseReviewCount(value: string | null | undefined): number | null {
    if (!value) {
        return null;
    }

    const normalized = value.replace(/\./g, '').replace(/[^\d]/g, '');
    if (normalized === '') {
        return null;
    }

    return Number(normalized);
}

export function buildJamedaDoctorDatasetItem(
    doctor: JamedaDoctor,
    params: Record<string, unknown>,
    response: JamedaSearchResponse,
): Record<string, unknown> {
    return {
        ...doctor,
        name: doctor.name ?? null,
        specialty: doctor.specialty ?? null,
        url: doctor.url ?? null,
        profile_url: normalizeJamedaUrl(doctor.url),
        rating: doctor.rating ?? doctor.jameda_rating?.rating ?? null,
        review_count: doctor.review_count ?? doctor.jameda_rating?.count ?? null,
        review_count_number: parseReviewCount(doctor.review_count ?? doctor.jameda_rating?.count),
        address: doctor.address ?? null,
        image_url: doctor.image_url ?? null,
        jameda_rating: doctor.jameda_rating ?? null,
        request_q: params.q ?? null,
        request_loc: params.loc ?? null,
        request_page: params.page ?? null,
        request_per_page: params.per_page ?? null,
        total_results: response.meta?.total_results ?? null,
        total_pages: response.meta?.total_pages ?? null,
        has_next_page: response.meta?.has_next_page ?? null,
    };
}

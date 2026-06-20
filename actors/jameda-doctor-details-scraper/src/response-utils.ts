export interface JamedaDoctorDetailsMetadata {
    source?: string | null;
    scraped_at?: string | null;
    duration_ms?: number | null;
    [key: string]: unknown;
}

export interface JamedaDoctorProfile {
    basic_info?: {
        name?: string | null;
        title?: string | null;
        specialty?: string | null;
        specializations?: string | null;
        profile_url?: string | null;
        url?: string | null;
        image_url?: string | null;
        [key: string]: unknown;
    };
    description?: string | null;
    rating?: {
        rating?: string | number | null;
        score?: string | number | null;
        overall_score?: string | number | null;
        count?: string | number | null;
        review_count?: string | number | null;
        [key: string]: unknown;
    } | null;
    clinic?: {
        name?: string | null;
        [key: string]: unknown;
    } | null;
    contact?: {
        phone?: string | null;
        website?: string | null;
        [key: string]: unknown;
    } | null;
    address?: {
        street?: string | null;
        city?: string | null;
        postal_code?: string | null;
        zip?: string | null;
        full_address?: string | null;
        [key: string]: unknown;
    } | string | null;
    coordinates?: {
        latitude?: number | string | null;
        longitude?: number | string | null;
        lat?: number | string | null;
        lng?: number | string | null;
        [key: string]: unknown;
    } | null;
    opening_hours?: unknown;
    services?: unknown[];
    accepted_patients?: unknown;
    focus_areas?: unknown[];
    specialization_focus?: unknown[];
    conditions?: unknown[];
    conditions_treated?: unknown[];
    languages?: unknown[];
    booking_ids?: unknown;
    metadata?: JamedaDoctorDetailsMetadata;
    [key: string]: unknown;
}

export interface JamedaDoctorDetailsResponse extends JamedaDoctorProfile {
    success?: boolean;
    message?: string;
    data?: JamedaDoctorProfile;
    meta?: JamedaDoctorDetailsMetadata;
}

export interface JamedaDoctorDetailsDatasetContext {
    doctorUrl: string;
    params: Record<string, unknown>;
}

export interface JamedaDoctorDetailsOutputSummaryContext {
    doctorUrls: string[];
    savedProfiles: number;
    failures: Record<string, string>[];
    statusMessage: string | null;
}

function firstNonEmptyString(...values: unknown[]): string | null {
    for (const value of values) {
        if (typeof value === 'string' && value.trim() !== '') {
            return value.trim();
        }
    }

    return null;
}

function withProtocol(url: unknown): string | null {
    if (typeof url !== 'string' || url.trim() === '') {
        return null;
    }

    const trimmed = url.trim();
    if (/^\/\//.test(trimmed)) {
        return `https:${trimmed}`;
    }

    return /^https?:\/\//i.test(trimmed) ? trimmed : `https://${trimmed}`;
}

function buildFullAddress(address: JamedaDoctorProfile['address']): string | null {
    if (typeof address === 'string') {
        return address.trim() || null;
    }

    if (!address || typeof address !== 'object') {
        return null;
    }

    return firstNonEmptyString(
        address.full_address,
        [address.street, address.postal_code ?? address.zip, address.city]
            .filter((value) => typeof value === 'string' && value.trim() !== '')
            .join(', '),
    );
}

function toNumber(value: unknown): number | null {
    if (typeof value === 'number' && Number.isFinite(value)) {
        return value;
    }

    if (typeof value !== 'string') {
        return null;
    }

    const normalized = value.replace(',', '.').replace(/[^\d.-]/g, '');
    if (normalized === '') {
        return null;
    }

    const parsed = Number(normalized);
    return Number.isFinite(parsed) ? parsed : null;
}

function countItems(value: unknown): number | null {
    return Array.isArray(value) ? value.length : null;
}

export function unwrapJamedaDoctorDetailsResponse(response: JamedaDoctorDetailsResponse): JamedaDoctorProfile {
    return response.data ?? response;
}

export function buildJamedaDoctorDetailsDatasetItem(
    response: JamedaDoctorDetailsResponse,
    context: JamedaDoctorDetailsDatasetContext,
): Record<string, unknown> {
    const profile = unwrapJamedaDoctorDetailsResponse(response);
    const basicInfo = profile.basic_info ?? {};
    const rating = profile.rating ?? {};
    const clinic = profile.clinic ?? {};
    const contact = profile.contact ?? {};
    const coordinates = profile.coordinates ?? {};
    const metadata = profile.metadata ?? {};
    const scrapeMetadata = response.meta ?? {};

    return {
        ...response,
        requested_doctor_url: context.doctorUrl,
        doctor_url: firstNonEmptyString(basicInfo.profile_url, basicInfo.url, context.doctorUrl),
        doctor_name: firstNonEmptyString(basicInfo.name, profile.name),
        title: basicInfo.title ?? null,
        specialty: firstNonEmptyString(basicInfo.specialty, basicInfo.specializations, profile.specialty),
        description: profile.description ?? null,
        rating: rating.rating ?? rating.score ?? rating.overall_score ?? null,
        rating_number: toNumber(rating.rating ?? rating.score ?? rating.overall_score),
        review_count: rating.count ?? rating.review_count ?? null,
        review_count_number: toNumber(rating.count ?? rating.review_count),
        clinic_name: firstNonEmptyString(clinic.name),
        phone: firstNonEmptyString(contact.phone),
        website_url: withProtocol(contact.website),
        address: buildFullAddress(profile.address),
        city: typeof profile.address === 'object' && profile.address !== null ? profile.address.city ?? null : null,
        postal_code: typeof profile.address === 'object' && profile.address !== null ? profile.address.postal_code ?? profile.address.zip ?? null : null,
        latitude: toNumber(coordinates.latitude ?? coordinates.lat),
        longitude: toNumber(coordinates.longitude ?? coordinates.lng),
        image_url: withProtocol(basicInfo.image_url),
        services_count: countItems(profile.services),
        focus_areas_count: countItems(profile.focus_areas ?? profile.specialization_focus),
        conditions_count: countItems(profile.conditions ?? profile.conditions_treated),
        languages_count: countItems(profile.languages),
        opening_hours: profile.opening_hours ?? null,
        services: profile.services ?? null,
        accepted_patients: profile.accepted_patients ?? null,
        focus_areas: profile.focus_areas ?? profile.specialization_focus ?? null,
        conditions: profile.conditions ?? profile.conditions_treated ?? null,
        languages: profile.languages ?? null,
        booking_ids: profile.booking_ids ?? null,
        request_doctor_url: context.params.doctor_url ?? null,
        response_source: metadata.source ?? scrapeMetadata.source ?? null,
        scraped_at: metadata.scraped_at ?? scrapeMetadata.scraped_at ?? null,
    };
}

export function buildJamedaDoctorDetailsOutputSummary(
    context: JamedaDoctorDetailsOutputSummaryContext,
): Record<string, unknown> {
    return {
        request: {
            endpoint: '/jameda/doctor-details',
            doctor_urls: context.doctorUrls,
        },
        doctors_requested: context.doctorUrls.length,
        doctors_saved: context.savedProfiles,
        doctors_failed: context.failures.length,
        responses_saved: context.savedProfiles,
        status_message: context.statusMessage,
        failures: context.failures,
    };
}

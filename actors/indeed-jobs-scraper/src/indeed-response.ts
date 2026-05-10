export interface IndeedCompany {
    name?: string | null;
    logo?: string | null;
    website?: string | null;
    industry?: string | null;
    size?: string | null;
    description?: string | null;
    profile_url?: string | null;
    [key: string]: unknown;
}

export interface IndeedLocation {
    city?: string | null;
    state?: string | null;
    country?: string | null;
    country_name?: string | null;
    postal_code?: string | null;
    formatted?: string | null;
    latitude?: number | null;
    longitude?: number | null;
    is_remote?: boolean;
    [key: string]: unknown;
}

export interface IndeedJob {
    id?: string | null;
    title?: string | null;
    company?: IndeedCompany | string | null;
    location?: IndeedLocation | string | null;
    salary?: Record<string, unknown> | null;
    description_html?: string | null;
    contact_person?: Record<string, unknown> | null;
    attributes?: unknown[];
    date_published?: string | null;
    apply_url?: string | null;
    [key: string]: unknown;
}

export interface IndeedJobsResponse {
    success?: boolean;
    data?: {
        jobs?: IndeedJob[];
        pagination?: Record<string, unknown>;
        metadata?: Record<string, unknown>;
    };
    jobs?: IndeedJob[];
    pagination?: Record<string, unknown>;
    metadata?: Record<string, unknown>;
    [key: string]: unknown;
}

export function getIndeedJobs(response: IndeedJobsResponse): IndeedJob[] {
    const payload = response.data ?? response;

    if (Array.isArray(payload.jobs)) {
        return payload.jobs;
    }

    console.debug('Unexpected Indeed Jobs response shape: expected "data.jobs" or "jobs" array.');
    return [];
}

export function getIndeedPagination(response: IndeedJobsResponse): Record<string, unknown> | undefined {
    return response.data?.pagination ?? response.pagination;
}

export function getIndeedMetadata(response: IndeedJobsResponse): Record<string, unknown> | undefined {
    return response.data?.metadata ?? response.metadata;
}

export function getCompanyName(company: IndeedJob['company']): string | undefined {
    if (typeof company === 'string') {
        return company;
    }

    if (company && typeof company === 'object' && typeof company.name === 'string') {
        return company.name;
    }

    return undefined;
}

export function getFormattedLocation(location: IndeedJob['location']): string | undefined {
    if (typeof location === 'string') {
        return location;
    }

    if (!location || typeof location !== 'object') {
        return undefined;
    }

    if (typeof location.formatted === 'string' && location.formatted.trim()) {
        return location.formatted;
    }

    const parts = [location.city, location.state, location.country]
        .filter((part): part is string => typeof part === 'string' && part.trim() !== '');

    return parts.length > 0 ? parts.join(', ') : undefined;
}

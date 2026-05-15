export interface StepstoneCompany {
    id?: string | number | null;
    name?: string | null;
    logo_url?: string | null;
    url?: string | null;
    [key: string]: unknown;
}

export interface StepstoneLocation {
    formatted?: string | null;
    city?: string | null;
    region?: string | null;
    country?: string | null;
    [key: string]: unknown;
}

export interface StepstoneJob {
    id?: string | null;
    title?: string | null;
    url?: string | null;
    company?: StepstoneCompany | string | null;
    location?: StepstoneLocation | string | null;
    salary?: Record<string, unknown> | null;
    date_posted?: string | null;
    description?: string | null;
    contact_person?: Record<string, unknown> | null;
    skills?: string[];
    labels?: string[];
    work_from_home?: boolean;
    is_highlighted?: boolean;
    is_sponsored?: boolean;
    [key: string]: unknown;
}

export interface StepstoneDatasetJob extends StepstoneJob {
    company_name: string | null;
    company_url: string | null;
    location_formatted: string | null;
    location_city: string | null;
    location_region: string | null;
    location_country: string | null;
}

export interface StepstoneJobsResponse {
    success?: boolean;
    data?: {
        jobs?: StepstoneJob[];
        pagination?: Record<string, unknown>;
        metadata?: Record<string, unknown>;
    };
    jobs?: StepstoneJob[];
    pagination?: Record<string, unknown>;
    metadata?: Record<string, unknown>;
    [key: string]: unknown;
}

export function getStepstoneJobs(response: StepstoneJobsResponse): StepstoneJob[] {
    if (Array.isArray(response.data?.jobs)) {
        return response.data.jobs;
    }

    if (Array.isArray(response.jobs)) {
        return response.jobs;
    }

    console.debug('Unexpected Stepstone Jobs response shape: expected "data.jobs" or "jobs" array.');
    return [];
}

export function getStepstonePagination(response: StepstoneJobsResponse): Record<string, unknown> | undefined {
    return response.data?.pagination ?? response.pagination;
}

export function getStepstoneMetadata(response: StepstoneJobsResponse): Record<string, unknown> | undefined {
    return response.data?.metadata ?? response.metadata;
}

export function toStepstoneDatasetJob(job: StepstoneJob): StepstoneDatasetJob {
    const location = getLocationParts(job.location);

    return {
        ...job,
        company_name: getCompanyName(job.company) ?? null,
        company_url: getCompanyUrl(job.company) ?? null,
        location_formatted: getFormattedLocation(job.location) ?? null,
        location_city: location.city ?? null,
        location_region: location.region ?? null,
        location_country: location.country ?? null,
    };
}

export function getCompanyName(company: StepstoneJob['company']): string | undefined {
    if (typeof company === 'string') {
        return company;
    }

    if (company && typeof company === 'object' && typeof company.name === 'string') {
        return company.name;
    }

    return undefined;
}

export function getCompanyUrl(company: StepstoneJob['company']): string | undefined {
    if (company && typeof company === 'object' && typeof company.url === 'string') {
        return company.url;
    }

    return undefined;
}

export function getFormattedLocation(location: StepstoneJob['location']): string | undefined {
    if (typeof location === 'string') {
        return location;
    }

    if (!location || typeof location !== 'object') {
        return undefined;
    }

    if (typeof location.formatted === 'string' && location.formatted.trim()) {
        return location.formatted;
    }

    const parts = [location.city, location.region, location.country]
        .filter((part): part is string => typeof part === 'string' && part.trim() !== '');

    return parts.length > 0 ? parts.join(', ') : undefined;
}

function getLocationParts(location: StepstoneJob['location']): StepstoneLocation {
    if (!location || typeof location !== 'object') {
        return {};
    }

    return location;
}

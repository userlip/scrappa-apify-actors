export interface KununuCompany {
    id?: string | number | null;
    uuid?: string | null;
    name?: string | null;
    slug?: string | null;
    url?: string | null;
    website?: string | null;
    logo?: string | null;
    logo_url?: string | null;
    rating?: number | null;
    score?: number | null;
    kununu_score?: number | null;
    is_top_company?: boolean | null;
    isTopCompany?: boolean | null;
    top_company?: boolean | null;
    industry?: string | null;
    [key: string]: unknown;
}

export interface KununuLocation {
    formatted?: string | null;
    city?: string | null;
    region?: string | null;
    country?: string | null;
    postal_code?: string | null;
    [key: string]: unknown;
}

export interface KununuJob {
    id?: string | number | null;
    uuid?: string | null;
    title?: string | null;
    url?: string | null;
    link?: string | null;
    company?: KununuCompany | string | null;
    company_name?: string | null;
    location?: KununuLocation | string | null;
    workplace?: string | null;
    workplace_model?: string | null;
    employment_type?: string | null;
    employment_types?: string[] | null;
    employmentTypes?: string[] | null;
    career_level?: string | number | null;
    benefits?: string[] | null;
    salary?: Record<string, unknown> | string | null;
    date_posted?: string | null;
    posted_at?: string | null;
    postedAt?: string | null;
    city?: string | null;
    region?: string | null;
    countryCode?: string | null;
    stateCode?: string | null;
    description?: string | null;
    snippet?: string | null;
    [key: string]: unknown;
}

export interface KununuDatasetJob extends Omit<KununuJob, 'company' | 'employment_types' | 'date_posted' | 'posted_at'> {
    job_id: string | number | null;
    job_url: string | null;
    company: KununuCompany | string | null;
    company_name: string | null;
    company_slug: string | null;
    company_url: string | null;
    company_score: number | null;
    company_is_top_company: boolean | null;
    location_formatted: string | null;
    location_city: string | null;
    location_region: string | null;
    location_country: string | null;
    employment_types: string[] | null;
    date_posted: string | null;
    posted_at: string | null;
    raw_job?: KununuJob;
}

export interface KununuJobsResponse {
    success?: boolean;
    data?: {
        jobs?: KununuJob[];
        results?: KununuJob[];
        pagination?: Record<string, unknown>;
        metadata?: Record<string, unknown>;
    } | KununuJob[];
    jobs?: KununuJob[];
    results?: KununuJob[];
    pagination?: Record<string, unknown>;
    metadata?: Record<string, unknown>;
    meta?: {
        pagination?: Record<string, unknown>;
        [key: string]: unknown;
    };
    message?: string;
    error?: string | Record<string, unknown>;
    [key: string]: unknown;
}

export function getKununuJobs(response: KununuJobsResponse): KununuJob[] {
    if (response.success === false) {
        const message = getString(response.message) ?? getString(response.error) ?? 'Scrappa returned success=false for Kununu Jobs.';
        throw new Error(message);
    }

    if (Array.isArray(response.data)) {
        return response.data;
    }

    if (Array.isArray(response.data?.jobs)) {
        return response.data.jobs;
    }

    if (Array.isArray(response.data?.results)) {
        return response.data.results;
    }

    if (Array.isArray(response.jobs)) {
        return response.jobs;
    }

    if (Array.isArray(response.results)) {
        return response.results;
    }

    console.debug('Unexpected Kununu Jobs response shape: expected "data.jobs", "data.results", "jobs", or "results" array.');
    return [];
}

export function getKununuPagination(response: KununuJobsResponse): Record<string, unknown> | undefined {
    if (Array.isArray(response.data)) {
        return response.pagination ?? response.meta?.pagination;
    }

    return response.data?.pagination ?? response.pagination ?? response.meta?.pagination;
}

export function getKununuMetadata(response: KununuJobsResponse): Record<string, unknown> | undefined {
    if (Array.isArray(response.data)) {
        return response.metadata;
    }

    return response.data?.metadata ?? response.metadata;
}

export function toKununuDatasetJob(job: KununuJob, options: { includeRawJob?: boolean } = {}): KununuDatasetJob {
    const location = getLocationParts(job.location, job);
    const company = getCompanyParts(job.company, job);
    const jobUrl = getJobUrl(job);
    const datePosted = getString(job.date_posted ?? job.posted_at ?? job.postedAt) ?? null;
    const datasetJob: KununuDatasetJob = {
        ...job,
        job_id: job.id ?? job.uuid ?? null,
        job_url: jobUrl ?? null,
        company: job.company ?? null,
        company_name: company.name ?? null,
        company_slug: company.slug ?? null,
        company_url: company.url ?? null,
        company_score: company.score ?? null,
        company_is_top_company: company.isTopCompany ?? null,
        location_formatted: getFormattedLocation(job.location, job) ?? null,
        location_city: location.city ?? null,
        location_region: location.region ?? null,
        location_country: location.country ?? null,
        employment_types: job.employment_types ?? job.employmentTypes ?? null,
        date_posted: datePosted,
        posted_at: getString(job.posted_at ?? job.postedAt ?? job.date_posted) ?? null,
    };

    if (options.includeRawJob) {
        datasetJob.raw_job = job;
    }

    return datasetJob;
}

export function getCompanyName(company: KununuJob['company'], job?: KununuJob): string | undefined {
    return getCompanyParts(company, job).name;
}

export function getFormattedLocation(location: KununuJob['location'], job?: KununuJob): string | undefined {
    if (typeof location === 'string') {
        return location;
    }

    if (!location || typeof location !== 'object') {
        const fallbackParts = [job?.city, job?.region, job?.countryCode]
            .filter((part): part is string => typeof part === 'string' && part.trim() !== '');

        return fallbackParts.length > 0 ? fallbackParts.join(', ') : undefined;
    }

    if (typeof location.formatted === 'string' && location.formatted.trim()) {
        return location.formatted;
    }

    const locationParts = getLocationParts(location, job);
    const parts = [locationParts.city, locationParts.region, locationParts.country]
        .filter((part): part is string => typeof part === 'string' && part.trim() !== '');

    return parts.length > 0 ? parts.join(', ') : undefined;
}

function getCompanyParts(company: KununuJob['company'], job?: KununuJob): {
    name?: string;
    slug?: string;
    url?: string;
    score?: number;
    isTopCompany?: boolean;
} {
    if (typeof company === 'string') {
        return {
            name: company,
            score: getNumber(job?.company_score ?? job?.kununu_score),
            isTopCompany: getBoolean(job?.is_top_company ?? job?.isTopCompany ?? job?.top_company),
        };
    }

    if (!company || typeof company !== 'object') {
        return {
            name: getString(job?.company_name),
            score: getNumber(job?.company_score ?? job?.kununu_score),
            isTopCompany: getBoolean(job?.is_top_company ?? job?.isTopCompany ?? job?.top_company),
        };
    }

    return {
        name: getString(company.name ?? job?.company_name),
        slug: getString(company.slug),
        url: getString(company.url ?? company.website),
        score: getNumber(company.kununu_score ?? company.score ?? company.rating ?? job?.company_score ?? job?.kununu_score),
        isTopCompany: getBoolean(company.is_top_company ?? company.isTopCompany ?? company.top_company ?? job?.is_top_company ?? job?.isTopCompany ?? job?.top_company),
    };
}

function getLocationParts(location: KununuJob['location'], job?: KununuJob): KununuLocation {
    if (!location || typeof location !== 'object') {
        return {
            city: getString(job?.city),
            region: getString(job?.region),
            country: getString(job?.countryCode),
        };
    }

    return {
        ...location,
        city: getString(location.city) ?? getString(job?.city) ?? null,
        region: getString(location.region) ?? getString(job?.region) ?? null,
        country: getString(location.country) ?? getString(job?.countryCode) ?? null,
    };
}

function getJobUrl(job: KununuJob): string | undefined {
    return getString(job.url ?? job.link);
}

function getString(value: unknown): string | undefined {
    return typeof value === 'string' && value.trim() !== '' ? value : undefined;
}

function getNumber(value: unknown): number | undefined {
    return typeof value === 'number' && Number.isFinite(value) ? value : undefined;
}

function getBoolean(value: unknown): boolean | undefined {
    return typeof value === 'boolean' ? value : undefined;
}

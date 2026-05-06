import type { GoogleJob, GoogleJobsResponse } from './jobs-response.js';
import type { GoogleJobsInput } from './jobs-params.js';

interface IndeedCompany {
    name?: string;
    logo?: string;
    website?: string;
    [key: string]: unknown;
}

interface IndeedLocation {
    formatted?: string;
    city?: string;
    state?: string;
    country?: string;
    country_name?: string;
    is_remote?: boolean;
    [key: string]: unknown;
}

interface IndeedJob {
    id?: string;
    title?: string;
    company?: string | IndeedCompany;
    location?: string | IndeedLocation;
    salary?: unknown;
    description?: string;
    description_html?: string;
    attributes?: unknown[];
    date_published?: string;
    apply_url?: string;
    [key: string]: unknown;
}

interface IndeedJobsResponse {
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

export function buildIndeedFallbackParams(input: GoogleJobsInput): Record<string, unknown> {
    const parsed = parseJobsQuery(input.q ?? '');
    const params: Record<string, unknown> = {
        query: parsed.query || input.q,
        limit: 10,
    };

    if (parsed.location) {
        params.location = parsed.location;
    }

    if (input.gl) {
        params.country = input.gl.toUpperCase();
        params.gl = input.gl;
    }

    if (input.hl) {
        params.hl = input.hl;
    }

    return params;
}

export function transformIndeedFallbackResponse(
    response: unknown,
    input: GoogleJobsInput,
    originalError: string
): GoogleJobsResponse {
    const fallbackResponse = isIndeedJobsResponse(response) ? response : {};
    const payload = fallbackResponse.data ?? fallbackResponse;
    const jobs = Array.isArray(payload.jobs) ? payload.jobs : [];
    const transformedJobs = jobs.map((job, index) => transformIndeedJob(job, index));

    return {
        jobs_results: transformedJobs,
        jobs: transformedJobs,
        filters: [],
        search_information: {
            query_displayed: input.q,
            total_results: transformedJobs.length,
        },
        pagination: payload.pagination,
        metadata: payload.metadata,
        service_used: 'indeed',
        fallback_from: 'google_jobs',
        fallback_reason: originalError,
    };
}

function isIndeedJobsResponse(response: unknown): response is IndeedJobsResponse {
    return response !== null
        && typeof response === 'object'
        && ('success' in response || 'jobs' in response || 'data' in response);
}

function parseJobsQuery(query: string): { query: string; location?: string } {
    const normalized = query.trim().replace(/\s+/g, ' ');
    const match = /^(?<query>.+?)\s+jobs?\s+in\s+(?<location>.+)$/i.exec(normalized);

    if (!match?.groups) {
        return { query: normalized };
    }

    return {
        query: match.groups.query.trim(),
        location: match.groups.location.trim(),
    };
}

function transformIndeedJob(job: IndeedJob, index: number): GoogleJob {
    const companyName = getCompanyName(job.company);
    const location = getLocation(job.location);
    const jobId = job.id ? String(job.id) : undefined;
    const link = typeof job.apply_url === 'string' ? job.apply_url : undefined;

    const transformed: GoogleJob = {
        position: index + 1,
        title: job.title ?? 'Unknown Title',
        company: companyName,
        company_name: companyName,
        location,
        via: 'Indeed',
        description: getDescription(job),
        link,
        share_link: link,
        job_id: jobId,
        extensions: buildExtensions(job),
        detected_extensions: {
            source: 'indeed_fallback',
        },
    };

    if (job.salary !== undefined) {
        transformed.salary = job.salary;
    }

    return transformed;
}

function getCompanyName(company: IndeedJob['company']): string {
    if (typeof company === 'string' && company.trim()) {
        return company;
    }

    if (company && typeof company === 'object' && typeof company.name === 'string' && company.name.trim()) {
        return company.name;
    }

    return 'Unknown Company';
}

function getLocation(location: IndeedJob['location']): string {
    if (typeof location === 'string') {
        return location;
    }

    if (!location || typeof location !== 'object') {
        return '';
    }

    if (typeof location.formatted === 'string' && location.formatted.trim()) {
        return location.formatted;
    }

    return [location.city, location.state, location.country]
        .filter((part): part is string => typeof part === 'string' && part.trim() !== '')
        .join(', ');
}

function getDescription(job: IndeedJob): string {
    if (typeof job.description === 'string' && job.description.trim()) {
        return job.description;
    }

    if (typeof job.description_html !== 'string') {
        return '';
    }

    return job.description_html
        .replace(/<[^>]+>/g, ' ')
        .replace(/&nbsp;/g, ' ')
        .replace(/&amp;/g, '&')
        .replace(/&lt;/g, '<')
        .replace(/&gt;/g, '>')
        .replace(/&#039;|&apos;/g, "'")
        .replace(/&quot;/g, '"')
        .replace(/\s+/g, ' ')
        .trim();
}

function buildExtensions(job: IndeedJob): string[] {
    const extensions = [];

    if (typeof job.date_published === 'string' && job.date_published.trim()) {
        extensions.push(job.date_published);
    }

    if (Array.isArray(job.attributes)) {
        for (const attribute of job.attributes) {
            if (typeof attribute === 'string' && attribute.trim()) {
                extensions.push(attribute);
            }
        }
    }

    return extensions;
}

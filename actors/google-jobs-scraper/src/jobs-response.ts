export interface GoogleJob {
    title: string;
    company?: string;
    company_name?: string;
    location?: string;
    via?: string;
    description?: string;
    extensions?: string[];
    detected_extensions?: Record<string, unknown>;
    job_id?: string;
    thumbnail?: string;
    related_links?: unknown[];
    [key: string]: unknown;
}

export interface GoogleJobsResponse {
    jobs?: GoogleJob[];
    jobs_results?: GoogleJob[];
    filters?: unknown[];
    next_page_token?: string;
    search_information?: {
        query_displayed?: string;
        total_results?: number;
    };
    pagination?: {
        next_page_token?: string;
        [key: string]: unknown;
    };
    [key: string]: unknown;
}

export function getJobs(response: GoogleJobsResponse): GoogleJob[] {
    if (Array.isArray(response.jobs) && response.jobs.length > 0) {
        return response.jobs;
    }

    if (Array.isArray(response.jobs_results)) {
        return response.jobs_results;
    }

    return Array.isArray(response.jobs) ? response.jobs : [];
}

export function getNextPageToken(response: GoogleJobsResponse): string | undefined {
    return response.next_page_token ?? response.pagination?.next_page_token;
}

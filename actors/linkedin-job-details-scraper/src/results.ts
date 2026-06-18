import { ScrappaApiError } from './shared/index.js';

export interface LinkedInJobDetailsResponse {
    success?: boolean;
    title?: string | null;
    job_title?: string | null;
    company?: string | null;
    company_name?: string | null;
    location?: string | null;
    employment_type?: string | null;
    seniority_level?: string | null;
    posted_date?: string | null;
    date_posted?: string | null;
    applicants?: string | number | null;
    applicant_count?: string | number | null;
    apply_url?: string | null;
    application_url?: string | null;
    url?: string | null;
    cached?: boolean;
    cached_at?: string;
    message?: string;
    status_code?: number;
    data?: Record<string, unknown>;
    [key: string]: unknown;
}

export type LinkedInJobDetailsResult = Omit<LinkedInJobDetailsResponse, 'url'> & {
    input_url: string;
    normalized_url?: string;
    url?: string;
    title?: string | null;
    company?: string | null;
    posted_date?: string | null;
    applicants?: string | number | null;
    apply_url?: string | null;
    error?: string;
    error_type?: string;
};

function firstPresent<T>(...values: (T | null | undefined)[]): T | undefined {
    return values.find((value): value is T => {
        if (value === undefined || value === null) {
            return false;
        }

        return typeof value !== 'string' || value.trim() !== '';
    });
}

function optionalField<T>(key: string, value: T | undefined): Record<string, T> {
    return value === undefined ? {} : { [key]: value };
}

export function buildLinkedInJobDetailsDatasetItem(
    response: LinkedInJobDetailsResponse,
    inputUrl: string,
    normalizedUrl: string,
): LinkedInJobDetailsResult {
    const responseUrl = typeof response.url === 'string' && response.url.trim() !== ''
        ? response.url
        : normalizedUrl;

    return {
        ...response,
        success: response.success ?? true,
        ...optionalField('title', firstPresent(response.title, response.job_title)),
        ...optionalField('company', firstPresent(response.company, response.company_name)),
        ...optionalField('posted_date', firstPresent(response.posted_date, response.date_posted)),
        ...optionalField('applicants', firstPresent(response.applicants, response.applicant_count)),
        ...optionalField('apply_url', firstPresent(response.apply_url, response.application_url)),
        url: responseUrl,
        input_url: inputUrl,
        normalized_url: normalizedUrl,
    };
}

export function buildLinkedInJobDetailsFailureItem(
    error: unknown,
    inputUrl: string,
    normalizedUrl?: string,
): LinkedInJobDetailsResult {
    const statusCode = error instanceof ScrappaApiError ? error.status : undefined;
    const message = error instanceof Error ? error.message : String(error);

    return {
        success: false,
        input_url: inputUrl,
        normalized_url: normalizedUrl,
        url: normalizedUrl,
        error: message,
        error_type: error instanceof ScrappaApiError ? 'scrappa_api_error' : 'error',
        message: statusCode === 404 ? 'Job not found' : message,
        status_code: statusCode,
    };
}

export function buildLinkedInJobDetailsOutput(result: LinkedInJobDetailsResult): LinkedInJobDetailsResponse {
    const {
        input_url: _inputUrl,
        normalized_url: _normalizedUrl,
        url: _url,
        error: _error,
        error_type: _errorType,
        ...output
    } = result;

    return output as LinkedInJobDetailsResponse;
}

export function isRecoverableLinkedInJobDetailsError(error: unknown): boolean {
    return error instanceof ScrappaApiError && error.status === 404;
}

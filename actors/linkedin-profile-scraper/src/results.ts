import { ScrappaApiError } from './shared/index.js';

interface Experience {
    company?: string;
    url?: string;
    start_date?: string;
    end_date?: string;
    [key: string]: unknown;
}

interface Education {
    school?: string;
    url?: string;
    start_date?: string;
    end_date?: string;
    [key: string]: unknown;
}

interface Article {
    title?: string;
    url?: string;
    published_date?: string;
    [key: string]: unknown;
}

interface Activity {
    type?: string;
    text?: string;
    date?: string;
    [key: string]: unknown;
}

interface Publication {
    title?: string;
    publisher?: string;
    date?: string;
    [key: string]: unknown;
}

interface Project {
    title?: string;
    description?: string;
    date?: string;
    [key: string]: unknown;
}

interface Recommendation {
    name?: string;
    title?: string;
    text?: string;
    [key: string]: unknown;
}

interface SimilarProfile {
    name?: string;
    url?: string;
    title?: string;
    [key: string]: unknown;
}

export interface LinkedInProfileResponse {
    success: boolean;
    name?: string;
    image?: string;
    location?: string;
    followers?: number;
    connections?: number;
    about?: string;
    job_titles?: string[];
    experience?: Experience[];
    education?: Education[];
    skills?: string[];
    articles?: Article[];
    activity?: Activity[];
    publications?: Publication[];
    projects?: Project[];
    recommendations?: Recommendation[];
    similar_profiles?: SimilarProfile[];
    cached?: boolean;
    cached_at?: string;
    message?: string;
    status_code?: number;
    [key: string]: unknown;
}

export type LinkedInProfileResult = Omit<LinkedInProfileResponse, 'url'> & {
    input_url: string;
    normalized_url?: string;
    url?: string;
    error?: string;
    error_type?: string;
};

export function buildLinkedInProfileDatasetItem(
    response: LinkedInProfileResponse,
    inputUrl: string,
    normalizedUrl: string,
): LinkedInProfileResult {
    const responseUrl = typeof response.url === 'string' && response.url.trim()
        ? response.url
        : normalizedUrl;

    return {
        ...response,
        url: responseUrl,
        input_url: inputUrl,
        normalized_url: normalizedUrl,
    };
}

export function buildLinkedInProfileFailureItem(
    error: unknown,
    inputUrl: string,
    normalizedUrl?: string,
): LinkedInProfileResult {
    const statusCode = error instanceof ScrappaApiError ? error.status : undefined;
    const message = error instanceof Error ? error.message : String(error);

    return {
        success: false,
        input_url: inputUrl,
        normalized_url: normalizedUrl,
        url: normalizedUrl,
        error: message,
        error_type: error instanceof ScrappaApiError ? 'scrappa_api_error' : 'error',
        message: statusCode === 404 ? 'Profile not found' : message,
        status_code: statusCode,
    };
}

export function isRecoverableLinkedInProfileError(error: unknown): boolean {
    // Keep only true per-profile misses recoverable. Auth, rate limit, and server errors
    // should fail the run so Scrappa or Apify reliability issues are visible.
    return error instanceof ScrappaApiError && error.status === 404;
}

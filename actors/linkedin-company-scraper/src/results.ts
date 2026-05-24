import { ScrappaApiError } from './shared/index.js';

export interface Employee {
    name: string;
    title: string;
    profile_url: string;
}

export interface Post {
    text: string;
    date: string;
    likes: number;
    comments: number;
}

export interface SimilarPage {
    name: string;
    url: string;
}

export interface Funding {
    round: string;
    amount: string;
    date: string;
}

export interface Address {
    street?: string;
    city?: string;
    state?: string;
    country?: string;
    postal_code?: string;
}

export interface LinkedInCompanyResponse {
    success: boolean;
    name?: string;
    description?: string;
    logo?: string;
    website?: string;
    employee_count?: number;
    address?: Address[];
    posts?: Post[];
    followers?: number;
    similar_pages?: SimilarPage[];
    specialties?: string[];
    employees?: Employee[];
    funding?: Funding | null;
    industry?: string;
    size?: string;
    type?: string;
    cached?: boolean;
    cached_at?: string;
    message?: string;
    status_code?: number;
    [key: string]: unknown;
}

export type LinkedInCompanyResult = LinkedInCompanyResponse & {
    input_url: string;
    normalized_url?: string;
    url?: string;
    error?: string;
    error_type?: string;
};

export function buildLinkedInCompanyDatasetItem(
    response: LinkedInCompanyResponse,
    inputUrl: string,
    normalizedUrl: string,
): LinkedInCompanyResult {
    return {
        ...response,
        url: response.url as string | undefined ?? normalizedUrl,
        input_url: inputUrl,
        normalized_url: normalizedUrl,
    };
}

export function buildLinkedInCompanyFailureItem(
    error: unknown,
    inputUrl: string,
    normalizedUrl?: string,
): LinkedInCompanyResult {
    const statusCode = error instanceof ScrappaApiError ? error.status : undefined;
    const message = error instanceof Error ? error.message : String(error);

    return {
        success: false,
        input_url: inputUrl,
        normalized_url: normalizedUrl,
        url: normalizedUrl,
        error: message,
        error_type: error instanceof ScrappaApiError ? 'scrappa_api_error' : 'error',
        message: statusCode === 404 ? 'Company not found' : message,
        status_code: statusCode,
    };
}

export function isRecoverableLinkedInCompanyError(error: unknown): boolean {
    return error instanceof ScrappaApiError && error.status === 404;
}

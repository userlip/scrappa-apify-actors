import { ScrappaHttpError, ScrappaTimeoutError } from './shared/index.js';

export interface DomainAvailabilityResponse {
    domain?: string;
    available?: boolean | null;
    registered?: boolean | null;
    status?: string;
    confidence?: string;
    source?: string;
    rdap_url?: string;
    rdap_status_code?: number;
    rdap_events?: unknown[];
    nameservers?: string[];
    message?: string;
    [key: string]: unknown;
}

export interface DomainAvailabilityDatasetItem extends Record<string, unknown> {
    success: boolean;
    input_domain: string;
    domain: string | null;
    available: boolean | null;
    registered: boolean | null;
    status: string | null;
    confidence: string | null;
    source: string | null;
    rdap_url: string | null;
    rdap_status_code: number | null;
    rdap_events: unknown[];
    nameservers: string[];
    message?: string;
    error?: string;
    status_code?: number;
}

export function buildDomainAvailabilityDatasetItem(
    response: DomainAvailabilityResponse,
    inputDomain: string,
    requestedDomain: string,
): DomainAvailabilityDatasetItem {
    return {
        success: true,
        input_domain: inputDomain,
        domain: typeof response.domain === 'string' ? response.domain : requestedDomain,
        available: typeof response.available === 'boolean' ? response.available : null,
        registered: typeof response.registered === 'boolean' ? response.registered : null,
        status: typeof response.status === 'string' ? response.status : null,
        confidence: typeof response.confidence === 'string' ? response.confidence : null,
        source: typeof response.source === 'string' ? response.source : null,
        rdap_url: typeof response.rdap_url === 'string' ? response.rdap_url : null,
        rdap_status_code: typeof response.rdap_status_code === 'number' ? response.rdap_status_code : null,
        rdap_events: Array.isArray(response.rdap_events) ? response.rdap_events : [],
        nameservers: Array.isArray(response.nameservers) ? response.nameservers : [],
        message: typeof response.message === 'string' ? response.message : undefined,
    };
}

export function buildDomainAvailabilityFailureItem(
    error: unknown,
    inputDomain: string,
    domain?: string,
): DomainAvailabilityDatasetItem {
    const message = error instanceof Error ? error.message : String(error);
    const statusCode = error instanceof ScrappaHttpError ? error.status : undefined;

    return {
        success: false,
        input_domain: inputDomain,
        domain: domain ?? null,
        available: null,
        registered: null,
        status: 'error',
        confidence: null,
        source: 'scrappa',
        rdap_url: null,
        rdap_status_code: null,
        rdap_events: [],
        nameservers: [],
        error: message,
        status_code: statusCode,
    };
}

export function isPerDomainAvailabilityFailure(error: unknown): boolean {
    if (error instanceof ScrappaTimeoutError) {
        return true;
    }

    if (error instanceof ScrappaHttpError) {
        return [400, 404, 408, 422, 429, 500, 502, 503, 504].includes(error.status);
    }

    return false;
}

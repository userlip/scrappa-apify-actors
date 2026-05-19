export type TrustpilotSearchType = 'company_search' | 'category';

export interface TrustpilotBusinessSearchInput {
    search_type?: unknown;
    query?: unknown;
    category?: unknown;
    locale?: unknown;
    country?: unknown;
    page?: unknown;
    max_pages?: unknown;
    per_page?: unknown;
    min_rating?: unknown;
    min_review_count?: unknown;
    sort?: unknown;
    claimed?: unknown;
    limit?: unknown;
    trustscore?: unknown;
}

export interface TrustpilotBusinessSearchPlan {
    searchType: TrustpilotSearchType;
    endpoint: '/trustpilot/company-search' | '/trustpilot/businesses';
    baseParams: Record<string, unknown>;
    startPage: number;
    maxPages: number;
}

const LOCALES = [
    'da-DK',
    'de-AT',
    'de-CH',
    'de-DE',
    'en-AU',
    'en-CA',
    'en-GB',
    'en-IE',
    'en-NZ',
    'en-US',
    'es-ES',
    'fi-FI',
    'fr-BE',
    'nl-BE',
    'fr-FR',
    'it-IT',
    'ja-JP',
    'nb-NO',
    'nl-NL',
    'pl-PL',
    'pt-BR',
    'pt-PT',
    'sv-SE',
] as const;
const SEARCH_TYPES = ['company_search', 'category'] as const;
const SORT_VALUES = ['reviews_count', 'latest_review'] as const;
const DEFAULT_LOCALE = 'en-US';
const DEFAULT_COMPANY_PER_PAGE = 20;
const DEFAULT_CATEGORY_LIMIT = 20;
const DEFAULT_MAX_PAGES = 1;
const MAX_PAGES_PER_RUN = 10;
const MAX_PAGE = 999;

function decodeInputString(value: string): string {
    if (!value.includes('%')) {
        return value;
    }

    try {
        return decodeURIComponent(value);
    } catch {
        return value;
    }
}

function cleanString(value: unknown, field: string, maxLength: number): string | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    if (typeof value !== 'string') {
        throw new Error(`${field} must be a string`);
    }

    const trimmed = decodeInputString(value).trim();
    if (trimmed === '') {
        return undefined;
    }

    if (trimmed.length > maxLength) {
        throw new Error(`${field} must be ${maxLength} characters or fewer`);
    }

    return trimmed;
}

function cleanRequiredString(value: unknown, field: string, minLength: number, maxLength: number): string {
    const cleaned = cleanString(value, field, maxLength);
    if (cleaned === undefined) {
        throw new Error(`${field} is required`);
    }

    if (cleaned.length < minLength) {
        throw new Error(`${field} must be at least ${minLength} characters`);
    }

    return cleaned;
}

function cleanInteger(value: unknown, field: string, min: number, max: number): number | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    const normalized = typeof value === 'string' && value.trim() !== '' && /^\d+$/.test(value.trim())
        ? Number(value.trim())
        : value;

    if (typeof normalized !== 'number' || !Number.isInteger(normalized)) {
        throw new Error(`${field} must be an integer`);
    }

    if (normalized < min || normalized > max) {
        throw new Error(`${field} must be between ${min} and ${max}`);
    }

    return normalized;
}

function cleanNumber(value: unknown, field: string, min: number, max: number): number | undefined {
    if (value === undefined || value === null || value === '' || value === 'any') {
        return undefined;
    }

    const normalized = typeof value === 'string' && value.trim() !== ''
        ? Number(value.trim())
        : value;

    if (typeof normalized !== 'number' || !Number.isFinite(normalized)) {
        throw new Error(`${field} must be a number`);
    }

    if (normalized < min || normalized > max) {
        throw new Error(`${field} must be between ${min} and ${max}`);
    }

    return normalized;
}

function cleanBoolean(value: unknown, field: string): boolean | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    if (typeof value === 'boolean') {
        return value;
    }

    if (typeof value === 'string') {
        const normalized = value.trim().toLowerCase();
        if (['true', '1', 'yes'].includes(normalized)) return true;
        if (['false', '0', 'no'].includes(normalized)) return false;
    }

    throw new Error(`${field} must be a boolean`);
}

function cleanCountry(value: unknown): string | undefined {
    const country = cleanString(value, 'country', 10);
    if (country === undefined) {
        return undefined;
    }

    const normalized = country.toUpperCase();
    if (!/^[A-Z]{2}$/.test(normalized)) {
        throw new Error('country must be an ISO-2 country code, for example US or GB');
    }

    return normalized;
}

function cleanEnum<T extends readonly string[]>(
    value: unknown,
    field: string,
    allowedValues: T,
): T[number] | undefined {
    const cleaned = cleanString(value, field, 100);
    if (cleaned === undefined) {
        return undefined;
    }

    if (!allowedValues.includes(cleaned)) {
        throw new Error(`${field} must be one of: ${allowedValues.join(', ')}`);
    }

    return cleaned;
}

function inferSearchType(input: TrustpilotBusinessSearchInput): TrustpilotSearchType {
    const explicit = cleanEnum(input.search_type, 'search_type', SEARCH_TYPES);
    if (explicit) {
        return explicit;
    }

    if (input.category !== undefined && input.category !== null && input.category !== '') {
        return 'category';
    }

    return 'company_search';
}

export function buildTrustpilotBusinessSearchPlan(input: TrustpilotBusinessSearchInput): TrustpilotBusinessSearchPlan {
    const searchType = inferSearchType(input);
    const startPage = cleanInteger(input.page, 'page', 1, MAX_PAGE) ?? 1;
    const maxPages = cleanInteger(input.max_pages, 'max_pages', 1, MAX_PAGES_PER_RUN) ?? DEFAULT_MAX_PAGES;

    if (startPage + maxPages - 1 > MAX_PAGE) {
        throw new Error('page plus max_pages cannot exceed page 999');
    }

    if (searchType === 'category') {
        const baseParams: Record<string, unknown> = {
            category: cleanRequiredString(input.category, 'category', 2, 120),
            limit: cleanInteger(input.limit, 'limit', 1, 50) ?? DEFAULT_CATEGORY_LIMIT,
        };

        const country = cleanCountry(input.country);
        if (country !== undefined) baseParams.country = country;

        const sort = cleanEnum(input.sort, 'sort', SORT_VALUES);
        if (sort !== undefined) baseParams.sort = sort;

        const claimed = cleanBoolean(input.claimed, 'claimed');
        if (claimed !== undefined) baseParams.claimed = claimed ? 1 : 0;

        const trustscore = cleanNumber(input.trustscore, 'trustscore', 0, 5);
        if (trustscore !== undefined) baseParams.trustscore = trustscore;

        return {
            searchType,
            endpoint: '/trustpilot/businesses',
            baseParams,
            startPage,
            maxPages,
        };
    }

    const baseParams: Record<string, unknown> = {
        query: cleanRequiredString(input.query, 'query', 2, 200),
        per_page: cleanInteger(input.per_page, 'per_page', 1, 50) ?? DEFAULT_COMPANY_PER_PAGE,
        locale: cleanEnum(input.locale, 'locale', LOCALES) ?? DEFAULT_LOCALE,
    };

    const country = cleanCountry(input.country);
    if (country !== undefined) baseParams.country = country;

    const minRating = cleanNumber(input.min_rating, 'min_rating', 0, 5);
    if (minRating !== undefined) baseParams.min_rating = minRating;

    const minReviewCount = cleanNumber(input.min_review_count, 'min_review_count', 0, 100000000);
    if (minReviewCount !== undefined) baseParams.min_review_count = minReviewCount;

    return {
        searchType,
        endpoint: '/trustpilot/company-search',
        baseParams,
        startPage,
        maxPages,
    };
}

export function buildPageParams(plan: TrustpilotBusinessSearchPlan, page: number): Record<string, unknown> {
    return {
        ...plan.baseParams,
        page,
    };
}

export function describeTrustpilotBusinessSearchRequest(plan: TrustpilotBusinessSearchPlan): string {
    const lastPage = plan.startPage + plan.maxPages - 1;
    const pages = plan.maxPages === 1 ? `page ${plan.startPage}` : `pages ${plan.startPage}-${lastPage}`;

    if (plan.searchType === 'category') {
        return `category "${String(plan.baseParams.category)}" (${pages})`;
    }

    return `company query "${String(plan.baseParams.query)}" (${pages})`;
}

export interface TrustpilotCompanyReviewsInput {
    company_domain?: unknown;
    locale?: unknown;
    page?: unknown;
    max_pages?: unknown;
    per_page?: unknown;
    sort?: unknown;
    rating?: unknown;
    verified?: unknown;
    with_replies?: unknown;
    query?: unknown;
    date_posted?: unknown;
    fields?: unknown;
}

export interface TrustpilotReviewsRequestPlan {
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

const SORT_VALUES = ['relevance', 'recency'] as const;
const DATE_POSTED_VALUES = ['any', 'last_12_months', 'last_6_months', 'last_3_months', 'last_30_days'] as const;
const DEFAULT_LOCALE = 'en-US';
const DEFAULT_PER_PAGE = 20;
const DEFAULT_SORT = 'recency';
const DEFAULT_DATE_POSTED = 'any';

function cleanDomain(value: unknown): string {
    if (typeof value !== 'string') {
        throw new Error('company_domain is required and must be a string');
    }

    let domain = value.trim().toLowerCase();
    domain = domain.replace(/^https?:\/\//i, '').replace(/^www\./i, '');
    domain = domain.split('/')[0] ?? domain;

    if (!/^[a-z0-9.-]+\.[a-z]{2,}$/.test(domain)) {
        throw new Error('company_domain must be a valid domain name, for example amazon.com');
    }

    if (domain.length > 255) {
        throw new Error('company_domain must be 255 characters or fewer');
    }

    return domain;
}

function cleanInteger(value: unknown, field: string, min: number, max: number): number | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    if (typeof value !== 'number' || !Number.isInteger(value)) {
        throw new Error(`${field} must be an integer`);
    }

    if (value < min || value > max) {
        throw new Error(`${field} must be between ${min} and ${max}`);
    }

    return value;
}

function cleanOptionalString(value: unknown, field: string, maxLength: number): string | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    if (typeof value !== 'string') {
        throw new Error(`${field} must be a string`);
    }

    const trimmed = value.trim();
    if (trimmed === '') {
        return undefined;
    }

    if (trimmed.length > maxLength) {
        throw new Error(`${field} must be ${maxLength} characters or fewer`);
    }

    return trimmed;
}

function cleanEnum<T extends readonly string[]>(
    value: unknown,
    field: string,
    allowedValues: T,
): T[number] | undefined {
    const cleaned = cleanOptionalString(value, field, 100);
    if (cleaned === undefined) {
        return undefined;
    }

    if (!allowedValues.includes(cleaned)) {
        throw new Error(`${field} must be one of: ${allowedValues.join(', ')}`);
    }

    return cleaned;
}

function cleanBoolean(value: unknown, field: string): boolean | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    if (typeof value === 'boolean') {
        return value;
    }

    throw new Error(`${field} must be a boolean`);
}

function cleanRating(value: unknown): string | undefined {
    const rating = cleanOptionalString(value, 'rating', 20)?.replace(/\s+/g, '');
    if (rating === undefined) {
        return undefined;
    }

    if (!/^[1-5](,[1-5])*$/.test(rating)) {
        throw new Error('rating must be a comma-separated list of ratings from 1 to 5, for example 4,5');
    }

    return rating;
}

export function buildTrustpilotCompanyReviewsPlan(input: TrustpilotCompanyReviewsInput): TrustpilotReviewsRequestPlan {
    const companyDomain = cleanDomain(input.company_domain);
    const startPage = cleanInteger(input.page, 'page', 1, 10) ?? 1;
    const maxPages = cleanInteger(input.max_pages, 'max_pages', 1, 10) ?? 1;

    if (startPage + maxPages - 1 > 10) {
        throw new Error('page plus max_pages cannot request beyond page 10');
    }

    const baseParams: Record<string, unknown> = {
        company_domain: companyDomain,
    };

    baseParams.per_page = cleanInteger(input.per_page, 'per_page', 1, 100) ?? DEFAULT_PER_PAGE;

    baseParams.locale = cleanEnum(input.locale, 'locale', LOCALES) ?? DEFAULT_LOCALE;

    const sort = cleanEnum(input.sort, 'sort', SORT_VALUES) ?? DEFAULT_SORT;
    if (sort !== DEFAULT_SORT) baseParams.sort = sort;

    const rating = cleanRating(input.rating);
    if (rating !== undefined) baseParams.rating = rating;

    const verified = cleanBoolean(input.verified, 'verified');
    if (verified !== undefined) baseParams.verified = verified ? 1 : 0;

    const withReplies = cleanBoolean(input.with_replies, 'with_replies');
    if (withReplies !== undefined) baseParams.with_replies = withReplies ? 1 : 0;

    const query = cleanOptionalString(input.query, 'query', 200);
    if (query !== undefined) baseParams.query = query;

    const datePosted = cleanEnum(input.date_posted, 'date_posted', DATE_POSTED_VALUES) ?? DEFAULT_DATE_POSTED;
    if (datePosted !== DEFAULT_DATE_POSTED) baseParams.date_posted = datePosted;

    const fields = cleanOptionalString(input.fields, 'fields', 500);
    if (fields !== undefined) baseParams.fields = fields;

    return { baseParams, startPage, maxPages };
}

export function buildPageParams(plan: TrustpilotReviewsRequestPlan, page: number): Record<string, unknown> {
    return {
        ...plan.baseParams,
        page,
    };
}

export function describeTrustpilotCompanyReviewsRequest(plan: TrustpilotReviewsRequestPlan): string {
    const companyDomain = String(plan.baseParams.company_domain);
    const lastPage = plan.startPage + plan.maxPages - 1;
    const pages = plan.maxPages === 1 ? `page ${plan.startPage}` : `pages ${plan.startPage}-${lastPage}`;
    return `${companyDomain} (${pages})`;
}

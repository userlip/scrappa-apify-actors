export interface KununuTargetInput {
    country?: unknown;
    countryCode?: unknown;
    company_slug?: unknown;
    slug?: unknown;
    company_id?: unknown;
    uuid?: unknown;
    url?: unknown;
}

export interface KununuReviewsInput {
    targets?: unknown;
    companies?: unknown;
    company_slug?: unknown;
    slug?: unknown;
    country?: unknown;
    countryCode?: unknown;
    company_id?: unknown;
    uuid?: unknown;
    url?: unknown;
    page?: unknown;
    max_pages?: unknown;
    review_type?: unknown;
    sort?: unknown;
    score_filters?: unknown;
    recommended_filters?: unknown;
    jobstatus_filters?: unknown;
    position_filters?: unknown;
    department_filters?: unknown;
    response_filters?: unknown;
    date_filters?: unknown;
    fetch_factor_scores?: unknown;
    include_raw_review?: unknown;
    include_raw_responses?: unknown;
}

export interface KununuTarget {
    country: string;
    company_slug: string;
    company_id?: string;
    input: string;
}

export interface KununuReviewsRequestPlan {
    targets: KununuTarget[];
    baseParams: Record<string, unknown>;
    startPage: number;
    maxPages: number;
    includeRawReview: boolean;
    includeRawResponses: boolean;
}

const COUNTRIES = ['de', 'at', 'ch'] as const;
const REVIEW_TYPES = ['employees', 'candidates'] as const;
const SORT_VALUES = ['worst', 'best', 'newest', 'oldest'] as const;
const SCORE_FILTERS = ['excellent', 'good', 'satisfactory', 'subpar'] as const;
const RECOMMENDED_FILTERS = ['yes', 'no'] as const;
const JOBSTATUS_FILTERS = ['current', 'former'] as const;
const POSITION_FILTERS = ['employee', 'manager', 'apprentice', 'student', 'intern', 'freelancer', 'contractor'] as const;
const DEPARTMENT_FILTERS = [
    'administration',
    'sales',
    'legal',
    'operations',
    'recruiting',
    'communication',
    'product',
    'logistic',
    'it',
    'management',
    'research',
    'controlling',
    'design',
    'procurement',
] as const;
const RESPONSE_FILTERS = ['yes', 'no'] as const;
const DATE_FILTERS = ['24months', '12months', '6months', '30days'] as const;

const DEFAULT_COUNTRY = 'de';
const DEFAULT_REVIEW_TYPE = 'employees';

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

function cleanBoolean(value: unknown, field: string): boolean | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    if (typeof value !== 'boolean') {
        throw new Error(`${field} must be a boolean`);
    }

    return value;
}

function cleanEnum<T extends readonly string[]>(
    value: unknown,
    field: string,
    allowedValues: T,
): T[number] | undefined {
    const cleaned = cleanOptionalString(value, field, 100)?.toLowerCase();
    if (cleaned === undefined) {
        return undefined;
    }

    if (!allowedValues.includes(cleaned)) {
        throw new Error(`${field} must be one of: ${allowedValues.join(', ')}`);
    }

    return cleaned;
}

function cleanStringArray<T extends readonly string[]>(
    value: unknown,
    field: string,
    allowedValues: T,
): T[number][] | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    const values = Array.isArray(value) ? value : [value];
    const cleaned = values
        .map((item) => cleanOptionalString(item, field, 100)?.toLowerCase())
        .filter((item): item is string => item !== undefined);

    for (const item of cleaned) {
        if (!allowedValues.includes(item)) {
            throw new Error(`${field} values must be one of: ${allowedValues.join(', ')}`);
        }
    }

    return cleaned as T[number][];
}

function cleanUuid(value: unknown, field: string): string | undefined {
    const cleaned = cleanOptionalString(value, field, 100);
    if (cleaned === undefined) {
        return undefined;
    }

    if (!/^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(cleaned)) {
        throw new Error(`${field} must be a valid UUID`);
    }

    return cleaned;
}

function parseKununuUrl(value: string): Partial<KununuTarget> | undefined {
    const rawValue = value.trim();
    const withProtocol = /^https?:\/\//i.test(rawValue) ? rawValue : `https://${rawValue}`;

    try {
        const url = new URL(withProtocol);
        if (!/(^|\.)kununu\.com$/i.test(url.hostname)) {
            return undefined;
        }

        const [country, slug] = url.pathname.split('/').filter(Boolean);
        if (!country || !slug) {
            return undefined;
        }

        return {
            country: country.toLowerCase(),
            company_slug: slug.toLowerCase(),
        };
    } catch {
        return undefined;
    }
}

function cleanSlug(value: unknown, field: string): string | undefined {
    const cleaned = cleanOptionalString(value, field, 255)?.toLowerCase();
    if (cleaned === undefined) {
        return undefined;
    }

    const urlParts = parseKununuUrl(cleaned);
    const slug = urlParts?.company_slug ?? cleaned.replace(/^\/+|\/+$/g, '').split('/').pop();
    if (!slug || !/^[a-z0-9][a-z0-9-]*$/.test(slug)) {
        throw new Error(`${field} must be a Kununu company slug, for example bmwgroup`);
    }

    return slug;
}

function normalizeTarget(target: unknown, defaultCountry: string): KununuTarget {
    if (typeof target === 'string') {
        const trimmed = target.trim();
        const urlParts = parseKununuUrl(trimmed);
        if (urlParts?.country && urlParts.company_slug) {
            const country = cleanEnum(urlParts.country, 'country', COUNTRIES) ?? defaultCountry;
            return {
                country,
                company_slug: cleanSlug(urlParts.company_slug, 'company_slug') as string,
                input: trimmed,
            };
        }

        const countrySlugMatch = trimmed.match(/^(de|at|ch)\/([a-z0-9][a-z0-9-]*)$/i);
        if (countrySlugMatch) {
            return {
                country: countrySlugMatch[1].toLowerCase(),
                company_slug: countrySlugMatch[2].toLowerCase(),
                input: trimmed,
            };
        }

        if (trimmed.includes('/')) {
            throw new Error('targets with a slash must use a supported country/slug pair such as de/bmwgroup, at/example, or ch/example');
        }

        return {
            country: defaultCountry,
            company_slug: cleanSlug(trimmed, 'company_slug') as string,
            input: trimmed,
        };
    }

    if (!target || typeof target !== 'object' || Array.isArray(target)) {
        throw new Error('Each target must be a string, Kununu URL, or object with country and company_slug');
    }

    const objectTarget = target as KununuTargetInput;
    const urlValue = cleanOptionalString(objectTarget.url, 'url', 500);
    const urlParts = urlValue ? parseKununuUrl(urlValue) : undefined;
    const country = cleanEnum(objectTarget.country ?? objectTarget.countryCode ?? urlParts?.country ?? defaultCountry, 'country', COUNTRIES) ?? defaultCountry;
    const slug = cleanSlug(objectTarget.company_slug ?? objectTarget.slug ?? urlParts?.company_slug, 'company_slug');
    if (!slug) {
        throw new Error('Each target object must include company_slug, slug, or a Kununu company URL');
    }

    const companyId = cleanUuid(objectTarget.company_id ?? objectTarget.uuid, 'company_id');
    return {
        country,
        company_slug: slug,
        ...(companyId ? { company_id: companyId } : {}),
        input: urlValue ?? `${country}/${slug}`,
    };
}

function buildTargets(input: KununuReviewsInput): KununuTarget[] {
    const defaultCountry = cleanEnum(input.country ?? input.countryCode ?? DEFAULT_COUNTRY, 'country', COUNTRIES) ?? DEFAULT_COUNTRY;
    const batchInput = input.targets ?? input.companies;
    const rawTargets = batchInput === undefined || batchInput === null || batchInput === ''
        ? [input.url ?? input.company_slug ?? input.slug]
        : Array.isArray(batchInput) ? batchInput : [batchInput];

    const targets = rawTargets
        .filter((target) => target !== undefined && target !== null && target !== '')
        .map((target) => normalizeTarget(target, defaultCountry));

    if (targets.length === 0) {
        throw new Error('Provide at least one Kununu company target in targets, companies, company_slug, slug, or url');
    }

    if (targets.length > 25) {
        throw new Error('targets cannot contain more than 25 companies per run');
    }

    return targets;
}

export function buildKununuReviewsPlan(input: KununuReviewsInput): KununuReviewsRequestPlan {
    const targets = buildTargets(input);
    const startPage = cleanInteger(input.page, 'page', 1, 100) ?? 1;
    const maxPages = cleanInteger(input.max_pages, 'max_pages', 1, 25) ?? 1;
    const includeRawReview = cleanBoolean(input.include_raw_review, 'include_raw_review') ?? false;
    const includeRawResponses = cleanBoolean(input.include_raw_responses, 'include_raw_responses') ?? false;

    const baseParams: Record<string, unknown> = {};

    const reviewType = cleanEnum(input.review_type, 'review_type', REVIEW_TYPES) ?? DEFAULT_REVIEW_TYPE;
    if (reviewType !== DEFAULT_REVIEW_TYPE) baseParams.review_type = reviewType;

    const sort = cleanEnum(input.sort, 'sort', SORT_VALUES);
    if (sort !== undefined) baseParams.sort = sort;

    const fetchFactorScores = cleanBoolean(input.fetch_factor_scores, 'fetch_factor_scores');
    if (fetchFactorScores !== undefined) baseParams.fetch_factor_scores = fetchFactorScores ? 1 : 0;

    const arrayFilters = {
        score_filters: cleanStringArray(input.score_filters, 'score_filters', SCORE_FILTERS),
        recommended_filters: cleanStringArray(input.recommended_filters, 'recommended_filters', RECOMMENDED_FILTERS),
        jobstatus_filters: cleanStringArray(input.jobstatus_filters, 'jobstatus_filters', JOBSTATUS_FILTERS),
        position_filters: cleanStringArray(input.position_filters, 'position_filters', POSITION_FILTERS),
        department_filters: cleanStringArray(input.department_filters, 'department_filters', DEPARTMENT_FILTERS),
        response_filters: cleanStringArray(input.response_filters, 'response_filters', RESPONSE_FILTERS),
        date_filters: cleanStringArray(input.date_filters, 'date_filters', DATE_FILTERS),
    };

    for (const [key, value] of Object.entries(arrayFilters)) {
        if (value !== undefined && value.length > 0) {
            baseParams[key] = value;
        }
    }

    return { targets, baseParams, startPage, maxPages, includeRawReview, includeRawResponses };
}

export function buildPageParams(
    plan: KununuReviewsRequestPlan,
    target: KununuTarget,
    page: number,
): Record<string, unknown> {
    return {
        ...plan.baseParams,
        country: target.country,
        company_slug: target.company_slug,
        ...(target.company_id ? { company_id: target.company_id } : {}),
        page,
    };
}

export function describeKununuReviewsRequest(plan: KununuReviewsRequestPlan): string {
    const lastPage = plan.startPage + plan.maxPages - 1;
    const pages = plan.maxPages === 1 ? `page ${plan.startPage}` : `pages ${plan.startPage}-${lastPage}`;
    const targets = plan.targets.map((target) => `${target.country}/${target.company_slug}`).join(', ');
    return `${targets} (${pages})`;
}

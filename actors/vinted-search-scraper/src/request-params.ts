export interface VintedSearchInput {
    query?: unknown;
    country?: unknown;
    page?: unknown;
    per_page?: unknown;
    max_pages?: unknown;
    order?: unknown;
    brand_ids?: unknown;
    catalog_ids?: unknown;
    color_ids?: unknown;
    size_ids?: unknown;
    material_ids?: unknown;
    status_ids?: unknown;
    price_from?: unknown;
    price_to?: unknown;
}

export interface VintedSearchPlan {
    baseParams: Record<string, unknown>;
    startPage: number;
    perPage: number;
    maxPages: number;
}

const VALID_COUNTRIES = ['FR', 'DE', 'ES', 'IT', 'NL', 'BE', 'AT', 'PL', 'CZ', 'LT', 'LU', 'SK', 'HU', 'RO', 'PT', 'SE', 'DK', 'FI', 'US'] as const;
const VALID_ORDERS = ['relevance', 'newest_first', 'price_low_to_high', 'price_high_to_low'] as const;
const FILTER_FIELDS = ['brand_ids', 'catalog_ids', 'color_ids', 'size_ids', 'material_ids', 'status_ids'] as const;
const MAX_PAGE = 999;
const DEFAULT_PER_PAGE = 24;
const DEFAULT_MAX_PAGES = 1;
const MAX_PAGES_PER_RUN = 20;

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

function cleanNumber(value: unknown, field: string, min: number): number | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    const normalized = typeof value === 'string' && value.trim() !== ''
        ? Number(value.trim())
        : value;

    if (typeof normalized !== 'number' || !Number.isFinite(normalized)) {
        throw new Error(`${field} must be a number`);
    }

    if (normalized < min) {
        throw new Error(`${field} must be at least ${min}`);
    }

    return normalized;
}

function cleanCountry(value: unknown): string {
    const country = cleanString(value, 'country', 2);
    if (country === undefined) {
        return 'FR';
    }

    const normalized = country.toUpperCase();
    if (!VALID_COUNTRIES.includes(normalized as typeof VALID_COUNTRIES[number])) {
        throw new Error(`country must be one of: ${VALID_COUNTRIES.join(', ')}`);
    }

    return normalized;
}

function cleanOrder(value: unknown): string | undefined {
    const order = cleanString(value, 'order', 30);
    if (order === undefined) {
        return undefined;
    }

    if (!VALID_ORDERS.includes(order as typeof VALID_ORDERS[number])) {
        throw new Error(`order must be one of: ${VALID_ORDERS.join(', ')}`);
    }

    return order;
}

function cleanIdList(value: unknown, field: string): string | undefined {
    const raw = cleanString(value, field, field === 'brand_ids' || field === 'catalog_ids' ? 500 : 200);
    if (raw === undefined) {
        return undefined;
    }

    const ids = raw.split(',')
        .map((id) => id.trim())
        .filter(Boolean);

    if (ids.length === 0) {
        return undefined;
    }

    if (!ids.every((id) => /^\d+$/.test(id))) {
        throw new Error(`${field} must be a comma-separated list of numeric IDs`);
    }

    return ids.join(',');
}

export function buildVintedSearchPlan(input: VintedSearchInput): VintedSearchPlan {
    const baseParams: Record<string, unknown> = {
        country: cleanCountry(input.country),
        per_page: cleanInteger(input.per_page, 'per_page', 1, 100) ?? DEFAULT_PER_PAGE,
    };

    const query = cleanString(input.query, 'query', 500);
    if (query !== undefined) {
        baseParams.query = query;
    }

    const order = cleanOrder(input.order);
    if (order !== undefined) {
        baseParams.order = order;
    }

    for (const field of FILTER_FIELDS) {
        const cleaned = cleanIdList(input[field], field);
        if (cleaned !== undefined) {
            baseParams[field] = cleaned;
        }
    }

    const priceFrom = cleanNumber(input.price_from, 'price_from', 0);
    const priceTo = cleanNumber(input.price_to, 'price_to', 0);
    if (priceFrom !== undefined) {
        baseParams.price_from = priceFrom;
    }
    if (priceTo !== undefined) {
        baseParams.price_to = priceTo;
    }
    if (priceFrom !== undefined && priceTo !== undefined && priceFrom > priceTo) {
        throw new Error('price_from cannot be greater than price_to');
    }

    const startPage = cleanInteger(input.page, 'page', 1, MAX_PAGE) ?? 1;
    const maxPages = cleanInteger(input.max_pages, 'max_pages', 1, MAX_PAGES_PER_RUN) ?? DEFAULT_MAX_PAGES;
    if (startPage + maxPages - 1 > MAX_PAGE) {
        throw new Error('page plus max_pages cannot exceed page 999');
    }

    return {
        baseParams,
        startPage,
        perPage: Number(baseParams.per_page),
        maxPages,
    };
}

export function buildPageParams(plan: VintedSearchPlan, page: number): Record<string, unknown> {
    return {
        ...plan.baseParams,
        page,
    };
}

export function describeVintedSearchRequest(plan: VintedSearchPlan): string {
    const query = plan.baseParams.query ? `"${String(plan.baseParams.query)}"` : 'all listings';
    const pageDescription = plan.maxPages === 1
        ? `page ${plan.startPage}`
        : `pages ${plan.startPage}-${plan.startPage + plan.maxPages - 1}`;

    return `${query} in ${String(plan.baseParams.country)} (${pageDescription}, ${plan.perPage}/page)`;
}

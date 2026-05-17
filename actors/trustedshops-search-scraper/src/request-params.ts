export interface TrustedShopsSearchInput {
    q?: unknown;
    market?: unknown;
    page?: unknown;
    max_pages?: unknown;
}

export interface TrustedShopsSearchPlan {
    baseParams: Record<string, unknown>;
    startPage: number;
    maxPages: number;
}

const VALID_MARKETS = ['DEU', 'GBR', 'AUT', 'CHE', 'NLD', 'ESP', 'ITA', 'FRA', 'BEL', 'POL', 'PRT'] as const;
const MAX_PAGE = 100;
const DEFAULT_MAX_PAGES = 1;
const MAX_PAGES_PER_RUN = 10;

function cleanString(value: unknown, field: string, maxLength: number): string | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    if (typeof value !== 'string') {
        throw new Error(`${field} must be a string`);
    }

    const trimmed = decodeURIComponent(value).trim();
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

function cleanMarket(value: unknown): string | undefined {
    const market = cleanString(value, 'market', 3);
    if (market === undefined) {
        return undefined;
    }

    const normalized = market.toUpperCase();
    if (!VALID_MARKETS.includes(normalized as typeof VALID_MARKETS[number])) {
        throw new Error(`market must be one of: ${VALID_MARKETS.join(', ')}`);
    }

    return normalized;
}

export function buildTrustedShopsSearchPlan(input: TrustedShopsSearchInput): TrustedShopsSearchPlan {
    const baseParams: Record<string, unknown> = {
        q: cleanRequiredString(input.q, 'q', 2, 200).replace(/\s*&\s*/g, ' '),
        market: cleanMarket(input.market) ?? 'DEU',
    };

    const startPage = cleanInteger(input.page, 'page', 0, MAX_PAGE) ?? 0;
    const maxPages = cleanInteger(input.max_pages, 'max_pages', 1, MAX_PAGES_PER_RUN) ?? DEFAULT_MAX_PAGES;

    if (startPage + maxPages - 1 > MAX_PAGE) {
        throw new Error('page plus max_pages cannot exceed page 100');
    }

    return { baseParams, startPage, maxPages };
}

export function buildPageParams(plan: TrustedShopsSearchPlan, page: number): Record<string, unknown> {
    return {
        ...plan.baseParams,
        page,
    };
}

export function describeTrustedShopsSearchRequest(plan: TrustedShopsSearchPlan): string {
    const pageDescription = plan.maxPages === 1
        ? `page ${plan.startPage}`
        : `pages ${plan.startPage}-${plan.startPage + plan.maxPages - 1}`;

    return `"${String(plan.baseParams.q)}" in ${String(plan.baseParams.market)} (${pageDescription})`;
}

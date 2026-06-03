export interface JamedaSearchInput {
    q?: unknown;
    loc?: unknown;
    searches?: unknown;
    page?: unknown;
    per_page?: unknown;
    max_pages?: unknown;
}

export interface JamedaSearchLookup {
    q: string;
    loc?: string;
}

export interface JamedaSearchPlan {
    baseParams: Record<string, unknown>;
    startPage: number;
    perPage: number;
    maxPages: number;
}

const MAX_PAGE = 500;
const DEFAULT_PAGE = 1;
const DEFAULT_PER_PAGE = 28;
const MAX_PER_PAGE = 28;
const DEFAULT_MAX_PAGES = 1;
const MAX_PAGES_PER_RUN = 2;
const MAX_SEARCHES_PER_RUN = 10;

interface JamedaPagination {
    startPage: number;
    perPage: number;
    maxPages: number;
}

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

export function buildJamedaSearchPlan(input: JamedaSearchInput): JamedaSearchPlan {
    return buildJamedaSearchPlanFromLookup({
        q: cleanRequiredString(input.q, 'q', 2, 255),
        loc: cleanString(input.loc, 'loc', 100),
    }, buildJamedaPagination(input));
}

function buildJamedaPagination(input: JamedaSearchInput): JamedaPagination {
    const startPage = cleanInteger(input.page, 'page', 1, MAX_PAGE) ?? DEFAULT_PAGE;
    const perPage = cleanInteger(input.per_page, 'per_page', 1, MAX_PER_PAGE) ?? DEFAULT_PER_PAGE;
    const maxPages = cleanInteger(input.max_pages, 'max_pages', 1, MAX_PAGES_PER_RUN) ?? DEFAULT_MAX_PAGES;

    if (startPage + maxPages - 1 > MAX_PAGE) {
        throw new Error('page plus max_pages cannot exceed page 500');
    }

    return { startPage, perPage, maxPages };
}

function buildJamedaSearchPlanFromLookup(
    lookup: JamedaSearchLookup,
    pagination: JamedaPagination,
): JamedaSearchPlan {
    return {
        baseParams: {
            q: lookup.q,
            loc: lookup.loc,
            per_page: pagination.perPage,
        },
        startPage: pagination.startPage,
        perPage: pagination.perPage,
        maxPages: pagination.maxPages,
    };
}

export function buildJamedaSearchPlans(input: JamedaSearchInput): JamedaSearchPlan[] {
    const lookups = getJamedaSearchLookups(input);
    const pagination = buildJamedaPagination(input);

    return lookups.map((lookup) => buildJamedaSearchPlanFromLookup(lookup, pagination));
}

function getJamedaSearchLookups(input: JamedaSearchInput): JamedaSearchLookup[] {
    const rawLookups: Array<{ q?: unknown; loc?: unknown }> = [];

    if (input.q !== undefined && input.q !== null && input.q !== '') {
        rawLookups.push({
            q: input.q,
            loc: input.loc,
        });
    }

    if (input.searches !== undefined && input.searches !== null && input.searches !== '') {
        if (!Array.isArray(input.searches)) {
            throw new Error('searches must be an array');
        }

        if (input.searches.length > MAX_SEARCHES_PER_RUN) {
            throw new Error(`searches must contain ${MAX_SEARCHES_PER_RUN} items or fewer`);
        }

        for (const search of input.searches) {
            if (search === null || typeof search !== 'object' || Array.isArray(search)) {
                throw new Error('Each searches item must be an object with q and optional loc');
            }

            const lookup = search as JamedaSearchLookup;
            rawLookups.push({
                q: lookup.q,
                loc: lookup.loc,
            });
        }
    }

    if (rawLookups.length === 0) {
        throw new Error('Provide q or searches');
    }

    const seen = new Set<string>();
    const deduped: JamedaSearchLookup[] = [];
    for (const lookup of rawLookups) {
        const q = cleanRequiredString(lookup.q, 'q', 2, 255);
        const loc = cleanString(lookup.loc, 'loc', 100);
        const key = `${q}\u0000${loc ?? ''}`;
        if (seen.has(key)) {
            continue;
        }

        seen.add(key);
        deduped.push({ q, loc });
    }

    return deduped;
}

export function buildPageParams(plan: JamedaSearchPlan, page: number): Record<string, unknown> {
    return {
        ...plan.baseParams,
        page,
    };
}

export function describeJamedaSearchRequest(plan: JamedaSearchPlan): string {
    const pageDescription = plan.maxPages === 1
        ? `page ${plan.startPage}`
        : `pages ${plan.startPage}-${plan.startPage + plan.maxPages - 1}`;
    const location = plan.baseParams.loc ? ` in ${String(plan.baseParams.loc)}` : '';

    return `"${String(plan.baseParams.q)}"${location} (${pageDescription}, ${plan.perPage} per page)`;
}

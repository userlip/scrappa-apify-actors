export interface KleinanzeigenSearchItemInput {
    query?: unknown;
    page?: unknown;
    location?: unknown;
    category?: unknown;
    price_min?: unknown;
    price_max?: unknown;
}

export interface KleinanzeigenSearchInput extends KleinanzeigenSearchItemInput {
    searches?: unknown;
}

export interface KleinanzeigenSearchPlanItem {
    params: Record<string, unknown>;
    index: number;
}

export interface KleinanzeigenSearchPlan {
    searches: KleinanzeigenSearchPlanItem[];
}

const MAX_QUERY_LENGTH = 500;
const MAX_FILTER_LENGTH = 100;
const MAX_PAGE = 100;
const MAX_BATCH_SEARCHES = 25;

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

function cleanRequiredString(value: unknown, field: string, maxLength: number): string {
    const cleaned = cleanString(value, field, maxLength);
    if (cleaned === undefined) {
        throw new Error(`${field} is required`);
    }

    return cleaned;
}

function cleanInteger(value: unknown, field: string, min: number, max: number): number | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    const normalized = typeof value === 'string' && value.trim() !== '' && /^-?\d+$/.test(value.trim())
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

function buildSingleSearchParams(input: KleinanzeigenSearchItemInput): Record<string, unknown> {
    const priceMin = cleanInteger(input.price_min, 'price_min', 0, Number.MAX_SAFE_INTEGER);
    const priceMax = cleanInteger(input.price_max, 'price_max', 0, Number.MAX_SAFE_INTEGER);

    if (priceMin !== undefined && priceMax !== undefined && priceMax < priceMin) {
        throw new Error('price_max cannot be less than price_min');
    }

    const params: Record<string, unknown> = {
        query: cleanRequiredString(input.query, 'query', MAX_QUERY_LENGTH),
        page: cleanInteger(input.page, 'page', 1, MAX_PAGE) ?? 1,
    };

    const location = cleanString(input.location, 'location', MAX_FILTER_LENGTH);
    if (location !== undefined) {
        params.location = location;
    }

    const category = cleanString(input.category, 'category', MAX_FILTER_LENGTH);
    if (category !== undefined) {
        params.category = category;
    }

    if (priceMin !== undefined) {
        params.price_min = priceMin;
    }

    if (priceMax !== undefined) {
        params.price_max = priceMax;
    }

    return params;
}

function getRawSearches(input: KleinanzeigenSearchInput): KleinanzeigenSearchItemInput[] {
    if (input.searches === undefined || input.searches === null) {
        return [input];
    }

    if (!Array.isArray(input.searches)) {
        throw new Error('searches must be an array');
    }

    if (input.searches.length === 0) {
        throw new Error('searches must contain at least one search');
    }

    if (input.searches.length > MAX_BATCH_SEARCHES) {
        throw new Error(`searches cannot contain more than ${MAX_BATCH_SEARCHES} search objects`);
    }

    return input.searches.map((search, index) => {
        if (!search || typeof search !== 'object' || Array.isArray(search)) {
            throw new Error(`searches[${index}] must be an object`);
        }

        return search as KleinanzeigenSearchItemInput;
    });
}

export function buildKleinanzeigenSearchPlan(input: KleinanzeigenSearchInput = {}): KleinanzeigenSearchPlan {
    return {
        searches: getRawSearches(input).map((search, index) => ({
            params: buildSingleSearchParams(search),
            index,
        })),
    };
}

export function describeKleinanzeigenSearchRequest(plan: KleinanzeigenSearchPlan): string {
    if (plan.searches.length === 1) {
        const params = plan.searches[0]?.params ?? {};
        const location = params.location ? ` in ${String(params.location)}` : '';
        const category = params.category ? `, category ${String(params.category)}` : '';
        return `"${String(params.query)}"${location} (page ${String(params.page)}${category})`;
    }

    return `${plan.searches.length} Kleinanzeigen searches`;
}

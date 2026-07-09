export interface KununuJobsInput {
    query?: unknown;
    location?: unknown;
    country?: unknown;
    page?: unknown;
    max_pages?: unknown;
    radius?: unknown;
    sort?: unknown;
    workplace?: unknown;
    employment_types?: unknown;
    career_level?: unknown;
    kununu_score?: unknown;
    industry?: unknown;
    discipline?: unknown;
    benefits?: unknown;
    is_top_company?: unknown;
    include_raw_job?: unknown;
}

export interface KununuJobsSearchPlan {
    params: Record<string, unknown>;
    startPage: number;
    maxPages: number;
    includeRawJob: boolean;
}

const DEFAULT_QUERY = 'Software Engineer';
const DEFAULT_LOCATION = 'Berlin';
const DEFAULT_COUNTRY = 'de';

const COUNTRIES = ['de', 'at', 'ch'] as const;
const RADIUS_VALUES = [10, 20, 30, 50, 100, 200] as const;
const ALLOWED_RADII = new Set<number>(RADIUS_VALUES);
const SORT_VALUES = ['newest', 'kununuScore'] as const;
const WORKPLACE_VALUES = ['FULL_REMOTE', 'PARTLY_REMOTE', 'NON_REMOTE'] as const;
const EMPLOYMENT_TYPE_VALUES = ['FULL_TIME', 'PART_TIME', 'INTERN', 'TEMPORARY', 'CONTRACTOR', 'SEASONAL', 'VOLUNTARY'] as const;
const CAREER_LEVEL_VALUES = ['1', '2', '3', '4', '5', '6', '99'] as const;
const KUNUNU_SCORE_VALUES = ['4-5', '3-4', '2-3', '1-2'] as const;
const BENEFIT_VALUES = [
    'flexWorkingHours',
    'pensionPlan',
    'coaching',
    'mobilePhone',
    'internet',
    'healthProgram',
    'reachability',
    'events',
    'discounts',
    'parking',
    'car',
    'meals',
    'dogs',
    'daycare',
    'cantine',
    'stockOptions',
    'doctor',
    'accessibility',
    'material',
    'clothes',
    'transportation',
] as const;

export function buildKununuJobsSearchPlan(input: KununuJobsInput | null | undefined): KununuJobsSearchPlan {
    const source = input ?? {};
    const startPage = cleanInteger(source.page, 'page', 1, 100) ?? 1;
    const maxPages = cleanInteger(source.max_pages, 'max_pages', 1, 10) ?? 1;
    const params: Record<string, unknown> = {
        query: cleanOptionalString(source.query, 'query', 120) ?? DEFAULT_QUERY,
        location: cleanOptionalString(source.location, 'location', 120) ?? DEFAULT_LOCATION,
        country: cleanEnum(source.country, 'country', COUNTRIES, normalizeLowercase) ?? DEFAULT_COUNTRY,
        page: startPage,
    };

    const radius = cleanInteger(source.radius, 'radius', 10, 200);
    if (radius !== undefined) {
        if (!ALLOWED_RADII.has(radius)) {
            throw new Error(`radius must be one of: ${RADIUS_VALUES.join(', ')}`);
        }
        params.radius = radius;
    }

    const sort = cleanEnum(source.sort, 'sort', SORT_VALUES);
    if (sort !== undefined) params.sort = sort;

    addArrayParam(params, 'workplace', cleanEnumArray(source.workplace, 'workplace', WORKPLACE_VALUES, normalizeUppercase));
    addArrayParam(params, 'employment_types', cleanEnumArray(source.employment_types, 'employment_types', EMPLOYMENT_TYPE_VALUES, normalizeUppercase));
    addArrayParam(params, 'career_level', cleanEnumArray(source.career_level, 'career_level', CAREER_LEVEL_VALUES, String));
    addArrayParam(params, 'kununu_score', cleanEnumArray(source.kununu_score, 'kununu_score', KUNUNU_SCORE_VALUES));
    addArrayParam(params, 'industry', cleanIntegerArray(source.industry, 'industry', 1, 44));
    addArrayParam(params, 'discipline', cleanIntegerArray(source.discipline, 'discipline', 1001, 1022));
    addArrayParam(params, 'benefits', cleanEnumArray(source.benefits, 'benefits', BENEFIT_VALUES));

    const isTopCompany = cleanBoolean(source.is_top_company, 'is_top_company');
    if (isTopCompany !== undefined) params.is_top_company = isTopCompany;

    return {
        params,
        startPage,
        maxPages,
        includeRawJob: cleanBoolean(source.include_raw_job, 'include_raw_job') ?? false,
    };
}

function addArrayParam(params: Record<string, unknown>, key: string, values: unknown[] | undefined): void {
    if (values !== undefined && values.length > 0) {
        params[key] = values;
    }
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

function cleanInteger(value: unknown, field: string, min: number, max: number): number | undefined {
    if (value === undefined || value === null) {
        return undefined;
    }

    if (typeof value === 'string') {
        const trimmed = value.trim();
        if (trimmed === '') {
            return undefined;
        }
        value = Number(trimmed);
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
    normalize: (value: string) => string = (item) => item,
): T[number] | undefined {
    const cleaned = cleanOptionalString(value, field, 100);
    if (cleaned === undefined) {
        return undefined;
    }

    const normalized = normalize(cleaned);
    if (!allowedValues.includes(normalized)) {
        throw new Error(`${field} must be one of: ${allowedValues.join(', ')}`);
    }

    return normalized;
}

function cleanEnumArray<T extends readonly string[]>(
    value: unknown,
    field: string,
    allowedValues: T,
    normalize: (value: string) => string = (item) => item,
): T[number][] | undefined {
    const values = cleanArrayValues(value);
    if (values === undefined) {
        return undefined;
    }

    const cleaned = values
        .map((item) => cleanEnumArrayItem(item, field, allowedValues, normalize))
        .filter((item): item is T[number] => item !== undefined);

    return [...new Set(cleaned)];
}

function cleanIntegerArray(value: unknown, field: string, min: number, max: number): number[] | undefined {
    const values = cleanArrayValues(value);
    if (values === undefined) {
        return undefined;
    }

    const cleaned = values
        .map((item) => cleanInteger(item, field, min, max))
        .filter((item): item is number => item !== undefined);

    return [...new Set(cleaned)];
}

function cleanArrayValues(value: unknown): unknown[] | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    return Array.isArray(value) ? value : [value];
}

function cleanEnumArrayItem<T extends readonly string[]>(
    value: unknown,
    field: string,
    allowedValues: T,
    normalize: (value: string) => string,
): T[number] | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    const stringValue = typeof value === 'number' ? String(value) : value;
    return cleanEnum(stringValue, field, allowedValues, normalize);
}

function normalizeLowercase(value: string): string {
    return value.toLowerCase();
}

function normalizeUppercase(value: string): string {
    return value.toUpperCase();
}

export function describeKununuJobsRequest(params: Record<string, unknown>): string {
    const query = typeof params.query === 'string' ? params.query : DEFAULT_QUERY;
    const location = typeof params.location === 'string' ? params.location : DEFAULT_LOCATION;
    const country = typeof params.country === 'string' ? params.country : DEFAULT_COUNTRY;
    return `"${query}" in ${location}, ${country.toUpperCase()}`;
}

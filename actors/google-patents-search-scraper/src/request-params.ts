export interface GooglePatentsSearchInput {
    q?: unknown;
    page?: unknown;
    num?: unknown;
    sort?: unknown;
    before?: unknown;
    after?: unknown;
    country?: unknown;
    language?: unknown;
    status?: unknown;
    type?: unknown;
    inventor?: unknown;
    assignee?: unknown;
}

const SORT_VALUES = ['new', 'old'] as const;
const STATUS_VALUES = ['GRANT', 'APPLICATION'] as const;
const TYPE_VALUES = ['PATENT', 'DESIGN'] as const;
const DATE_FILTER_TYPES = ['filing', 'publication', 'priority'] as const;

function cleanString(value: unknown, field: string, maxLength: number): string | undefined {
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

    if (typeof value !== 'number' || !Number.isInteger(value)) {
        throw new Error(`${field} must be an integer`);
    }

    if (value < min || value > max) {
        throw new Error(`${field} must be between ${min} and ${max}`);
    }

    return value;
}

function cleanEnum<T extends readonly string[]>(
    value: unknown,
    field: string,
    values: T,
    normalize: 'lower' | 'upper' = 'lower',
): T[number] | undefined {
    const cleaned = cleanString(value, field, 40);
    if (cleaned === undefined) {
        return undefined;
    }

    const normalized = normalize === 'upper' ? cleaned.toUpperCase() : cleaned.toLowerCase();
    if (!values.includes(normalized as T[number])) {
        throw new Error(`${field} must be one of: ${values.join(', ')}`);
    }

    return normalized as T[number];
}

function cleanDateFilter(value: unknown, field: string): string | undefined {
    const cleaned = cleanString(value, field, 30);
    if (cleaned === undefined) {
        return undefined;
    }

    const match = /^(filing|publication|priority):(\d{8})$/i.exec(cleaned);
    if (!match) {
        throw new Error(`${field} must use format filing:YYYYMMDD, publication:YYYYMMDD, or priority:YYYYMMDD`);
    }

    const [, rawType, date] = match;
    const parsed = new Date(`${date.slice(0, 4)}-${date.slice(4, 6)}-${date.slice(6, 8)}T00:00:00Z`);
    if (Number.isNaN(parsed.getTime()) || parsed.toISOString().slice(0, 10).replace(/-/g, '') !== date) {
        throw new Error(`${field} must include a valid calendar date`);
    }

    const type = rawType.toLowerCase();
    if (!DATE_FILTER_TYPES.includes(type as typeof DATE_FILTER_TYPES[number])) {
        throw new Error(`${field} must start with filing, publication, or priority`);
    }

    return `${type}:${date}`;
}

function cleanCsvString(value: unknown, field: string, maxLength: number): string | undefined {
    const cleaned = cleanString(value, field, maxLength);
    if (cleaned === undefined) {
        return undefined;
    }

    const parts = cleaned
        .split(',')
        .map((part) => part.trim())
        .filter(Boolean);

    if (parts.length === 0) {
        return undefined;
    }

    return parts.join(',');
}

function addIfDefined(params: Record<string, unknown>, key: string, value: unknown): void {
    if (value !== undefined) {
        params[key] = value;
    }
}

export function buildGooglePatentsSearchParams(input: GooglePatentsSearchInput): Record<string, unknown> {
    const params: Record<string, unknown> = {
        q: cleanRequiredString(input.q, 'q', 500),
    };

    addIfDefined(params, 'page', cleanInteger(input.page, 'page', 1, 100));
    addIfDefined(params, 'num', cleanInteger(input.num, 'num', 1, 100));
    addIfDefined(params, 'sort', cleanEnum(input.sort, 'sort', SORT_VALUES));
    addIfDefined(params, 'before', cleanDateFilter(input.before, 'before'));
    addIfDefined(params, 'after', cleanDateFilter(input.after, 'after'));
    addIfDefined(params, 'country', cleanCsvString(input.country, 'country', 200));
    addIfDefined(params, 'language', cleanCsvString(input.language, 'language', 200));
    addIfDefined(params, 'status', cleanEnum(input.status, 'status', STATUS_VALUES, 'upper'));
    addIfDefined(params, 'type', cleanEnum(input.type, 'type', TYPE_VALUES, 'upper'));
    addIfDefined(params, 'inventor', cleanCsvString(input.inventor, 'inventor', 300));
    addIfDefined(params, 'assignee', cleanCsvString(input.assignee, 'assignee', 300));

    return params;
}

export function describeGooglePatentsSearchRequest(params: Record<string, unknown>): string {
    const filters = ['country', 'language', 'status', 'type', 'before', 'after', 'inventor', 'assignee']
        .filter((field) => params[field] !== undefined)
        .map((field) => `${field}=${String(params[field])}`);
    const page = typeof params.page === 'number' ? ` page ${params.page}` : '';
    const num = typeof params.num === 'number' ? ` (${params.num} results per page)` : '';
    const sort = typeof params.sort === 'string' ? ` sorted by ${params.sort}` : '';
    const filterSuffix = filters.length > 0 ? ` with ${filters.join(', ')}` : '';

    return `query "${String(params.q)}"${page}${num}${sort}${filterSuffix}`;
}

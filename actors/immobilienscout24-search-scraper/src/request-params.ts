export interface Immobilienscout24SearchInput {
    location?: unknown;
    type?: unknown;
    price_min?: unknown;
    price_max?: unknown;
    rooms_min?: unknown;
    rooms_max?: unknown;
    size_min?: unknown;
    size_max?: unknown;
    per_page?: unknown;
    property_type?: unknown;
    page?: unknown;
    limit?: unknown;
}

export const DEFAULT_IMMOBILIENSCOUT24_SEARCH_INPUT = {
    location: 'Berlin',
    type: 'apartment-rent',
    page: 1,
    per_page: 20,
} as const;

const MAX_LOCATION_LENGTH = 120;
const MAX_PER_PAGE = 50;
const IMMOBILIENSCOUT24_PROPERTY_TYPES = ['apartment-rent', 'apartment-buy', 'house-rent', 'house-buy'] as const;

export function normalizeImmobilienscout24SearchInput(
    input?: Immobilienscout24SearchInput | null,
): Immobilienscout24SearchInput {
    if (!input) {
        return { ...DEFAULT_IMMOBILIENSCOUT24_SEARCH_INPUT };
    }

    const normalized: Immobilienscout24SearchInput = {};

    copyKnownInputValue(input, normalized, 'location', 'location');
    copyKnownInputValue(input, normalized, 'type', 'type');
    copyKnownInputValue(input, normalized, 'price_min', 'price_min');
    copyKnownInputValue(input, normalized, 'price_max', 'price_max');
    copyKnownInputValue(input, normalized, 'rooms_min', 'rooms_min');
    copyKnownInputValue(input, normalized, 'rooms_max', 'rooms_max');
    copyKnownInputValue(input, normalized, 'size_min', 'size_min');
    copyKnownInputValue(input, normalized, 'size_max', 'size_max');
    copyKnownInputValue(input, normalized, 'per_page', 'per_page');
    copyKnownInputValue(input, normalized, 'page', 'page');

    if (!Object.hasOwn(input, 'type')) {
        copyKnownInputValue(input, normalized, 'property_type', 'type');
    }

    if (!Object.hasOwn(input, 'per_page')) {
        copyKnownInputValue(input, normalized, 'limit', 'per_page');
    }

    const hasKnownInput = Object.keys(normalized).length > 0;
    if (!hasKnownInput) {
        return { ...DEFAULT_IMMOBILIENSCOUT24_SEARCH_INPUT };
    }

    return {
        ...DEFAULT_IMMOBILIENSCOUT24_SEARCH_INPUT,
        ...normalized,
    };
}

export function buildImmobilienscout24SearchParams(
    input: Immobilienscout24SearchInput,
): Record<string, unknown> {
    const params: Record<string, unknown> = {
        location: cleanRequiredString(input.location, 'location', MAX_LOCATION_LENGTH),
        type: cleanEnumString(input.type, 'type', IMMOBILIENSCOUT24_PROPERTY_TYPES),
        page: cleanInteger(input.page, 'page', 1, 10000),
        per_page: cleanInteger(input.per_page, 'per_page', 1, MAX_PER_PAGE),
    };

    copyOptionalIntegerParam(input, params, 'price_min', 0, 100000000);
    copyOptionalIntegerParam(input, params, 'price_max', 0, 100000000);
    copyOptionalNumberParam(input, params, 'rooms_min', 0, 100);
    copyOptionalNumberParam(input, params, 'rooms_max', 0, 100);
    copyOptionalIntegerParam(input, params, 'size_min', 0, 1000000);
    copyOptionalIntegerParam(input, params, 'size_max', 0, 1000000);
    validateMinMax(params, 'price_min', 'price_max');
    validateMinMax(params, 'rooms_min', 'rooms_max');
    validateMinMax(params, 'size_min', 'size_max');

    return params;
}

export function describeImmobilienscout24SearchRequest(params: Record<string, unknown>): string {
    const filters = [
        describeRange(params, 'price_min', 'price_max', 'price'),
        describeRange(params, 'rooms_min', 'rooms_max', 'rooms'),
        describeRange(params, 'size_min', 'size_max', 'size'),
    ].filter(Boolean);

    const filterText = filters.length > 0 ? `, ${filters.join(', ')}` : '';
    return `${params.type} properties in ${params.location} (page ${params.page}, per_page ${params.per_page}${filterText})`;
}

function cleanRequiredString(value: unknown, field: string, maxLength: number): string {
    if (typeof value !== 'string') {
        throw new Error(`${field} must be a string`);
    }

    const trimmed = value.trim();
    if (!trimmed) {
        throw new Error(`${field} is required`);
    }

    if (trimmed.length > maxLength) {
        throw new Error(`${field} must be ${maxLength} characters or fewer`);
    }

    return trimmed;
}

function copyKnownInputValue(
    input: Immobilienscout24SearchInput,
    normalized: Immobilienscout24SearchInput,
    sourceKey: keyof Immobilienscout24SearchInput,
    targetKey: keyof Immobilienscout24SearchInput,
): void {
    if (!Object.hasOwn(input, sourceKey)) {
        return;
    }

    const value = input[sourceKey];
    if (typeof value === 'string') {
        normalized[targetKey] = value.trim() as never;
    } else if (value !== undefined) {
        normalized[targetKey] = value as never;
    }
}

function cleanInteger(value: unknown, field: string, min: number, max: number): number {
    const normalized = typeof value === 'string' && /^-?\d+$/.test(value.trim())
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

function cleanNumber(value: unknown, field: string, min: number, max: number): number {
    const normalized = typeof value === 'string' && /^-?\d+(?:\.\d+)?$/.test(value.trim())
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

function cleanEnumString<T extends readonly string[]>(value: unknown, field: string, allowedValues: T): T[number] {
    const trimmed = cleanRequiredString(value, field, 40);

    if (!allowedValues.includes(trimmed)) {
        throw new Error(`${field} must be one of: ${allowedValues.join(', ')}`);
    }

    return trimmed;
}

function copyOptionalIntegerParam(
    input: Immobilienscout24SearchInput,
    params: Record<string, unknown>,
    field: keyof Immobilienscout24SearchInput,
    min: number,
    max: number,
): void {
    const value = input[field];
    if (value === undefined || value === null || value === '') {
        return;
    }

    params[field] = cleanInteger(value, field, min, max);
}

function copyOptionalNumberParam(
    input: Immobilienscout24SearchInput,
    params: Record<string, unknown>,
    field: keyof Immobilienscout24SearchInput,
    min: number,
    max: number,
): void {
    const value = input[field];
    if (value === undefined || value === null || value === '') {
        return;
    }

    params[field] = cleanNumber(value, field, min, max);
}

function validateMinMax(params: Record<string, unknown>, minField: string, maxField: string): void {
    const minValue = params[minField];
    const maxValue = params[maxField];

    if (
        typeof minValue === 'number'
        && typeof maxValue === 'number'
        && minValue > maxValue
    ) {
        throw new Error(`${minField} must be less than or equal to ${maxField}`);
    }
}

function describeRange(
    params: Record<string, unknown>,
    minField: string,
    maxField: string,
    label: string,
): string | null {
    const minValue = params[minField];
    const maxValue = params[maxField];

    if (minValue === undefined && maxValue === undefined) {
        return null;
    }

    if (minValue !== undefined && maxValue !== undefined) {
        return `${label} ${minValue}-${maxValue}`;
    }

    if (minValue !== undefined) {
        return `${label} >= ${minValue}`;
    }

    return `${label} <= ${maxValue}`;
}

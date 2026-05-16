export interface ImmoweltPropertySearchInput {
    location?: unknown;
    type?: unknown;
    per_page?: unknown;
    property_type?: unknown;
    page?: unknown;
    limit?: unknown;
}

export const DEFAULT_IMMOWELT_PROPERTY_SEARCH_INPUT = {
    location: 'Berlin',
    type: 'apartment-rent',
    page: 1,
    per_page: 20,
} as const;

const MAX_LOCATION_LENGTH = 120;
const MAX_PER_PAGE = 50;
const IMMOWELT_PROPERTY_TYPES = ['apartment-rent', 'apartment-buy', 'house-rent', 'house-buy'] as const;
const KNOWN_INPUT_KEYS = new Set<keyof ImmoweltPropertySearchInput>([
    'location',
    'type',
    'per_page',
    'property_type',
    'page',
    'limit',
]);

export function normalizeImmoweltPropertySearchInput(
    input?: ImmoweltPropertySearchInput | null,
): ImmoweltPropertySearchInput {
    if (!input) {
        return { ...DEFAULT_IMMOWELT_PROPERTY_SEARCH_INPUT };
    }

    const normalized: ImmoweltPropertySearchInput = {};

    for (const [key, value] of Object.entries(input) as [keyof ImmoweltPropertySearchInput, unknown][]) {
        if (!KNOWN_INPUT_KEYS.has(key)) {
            continue;
        }

        const normalizedKey = key === 'property_type'
            ? 'type'
            : key === 'limit'
                ? 'per_page'
                : key;

        if (typeof value === 'string') {
            normalized[normalizedKey] = value.trim() as never;
        } else if (value !== undefined) {
            normalized[normalizedKey] = value as never;
        }
    }

    const hasKnownInput = Object.keys(normalized).length > 0;
    if (!hasKnownInput) {
        return { ...DEFAULT_IMMOWELT_PROPERTY_SEARCH_INPUT };
    }

    return {
        ...DEFAULT_IMMOWELT_PROPERTY_SEARCH_INPUT,
        ...normalized,
    };
}

export function buildImmoweltPropertySearchParams(
    input: ImmoweltPropertySearchInput,
): Record<string, unknown> {
    return {
        location: cleanRequiredString(input.location, 'location', MAX_LOCATION_LENGTH),
        type: cleanEnumString(input.type, 'type', IMMOWELT_PROPERTY_TYPES),
        page: cleanInteger(input.page, 'page', 1, 10000),
        per_page: cleanInteger(input.per_page, 'per_page', 1, MAX_PER_PAGE),
    };
}

export function describeImmoweltPropertySearchRequest(params: Record<string, unknown>): string {
    return `${params.type} properties in ${params.location} (page ${params.page}, per_page ${params.per_page})`;
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

function cleanEnumString<T extends readonly string[]>(value: unknown, field: string, allowedValues: T): T[number] {
    const trimmed = cleanRequiredString(value, field, 40);

    if (!allowedValues.includes(trimmed)) {
        throw new Error(`${field} must be one of: ${allowedValues.join(', ')}`);
    }

    return trimmed;
}

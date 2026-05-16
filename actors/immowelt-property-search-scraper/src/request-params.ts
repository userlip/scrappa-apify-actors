export interface ImmoweltPropertySearchInput {
    location?: unknown;
    property_type?: unknown;
    page?: unknown;
    limit?: unknown;
}

export const DEFAULT_IMMOWELT_PROPERTY_SEARCH_INPUT = {
    location: 'Berlin',
    property_type: 'apartment',
    page: 1,
    limit: 20,
} as const;

const MAX_LOCATION_LENGTH = 120;
const MAX_LIMIT = 100;
const KNOWN_INPUT_KEYS = new Set<keyof ImmoweltPropertySearchInput>([
    'location',
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

        if (typeof value === 'string') {
            const trimmed = value.trim();
            if (trimmed !== '') {
                normalized[key] = trimmed as never;
            }
        } else if (value !== undefined && value !== null) {
            normalized[key] = value as never;
        }
    }

    const hasKnownInput = Object.values(normalized).some((value) => value !== undefined && value !== '');
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
        property_type: cleanRequiredString(input.property_type, 'property_type', 40),
        page: cleanInteger(input.page, 'page', 1, 10000),
        limit: cleanInteger(input.limit, 'limit', 1, MAX_LIMIT),
    };
}

export function describeImmoweltPropertySearchRequest(params: Record<string, unknown>): string {
    return `${params.property_type} properties in ${params.location} (page ${params.page}, limit ${params.limit})`;
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

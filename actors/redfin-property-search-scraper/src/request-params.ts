export interface RedfinPropertySearchParamsInput {
    region_id?: unknown;
    region_type?: unknown;
    market?: unknown;
    min_price?: unknown;
    max_price?: unknown;
    num_beds?: unknown;
    num_baths?: unknown;
    property_types?: unknown;
    status?: unknown;
    sold_within_days?: unknown;
    num_homes?: unknown;
    page?: unknown;
}

export interface RedfinPropertySearchInput extends RedfinPropertySearchParamsInput {
    searches?: unknown;
}

export interface RedfinPropertySearchRequest {
    params: Record<string, unknown>;
    index: number;
}

const MAX_SEARCHES_PER_RUN = 25;
const MAX_NUM_HOMES = 450;
const VALID_REGION_TYPES = [1, 2, 4, 5, 6] as const;
const VALID_STATUSES = [1, 9, 130, 131] as const;

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

function cleanInteger(value: unknown, field: string, min: number, max?: number): number | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    const normalized = typeof value === 'string' && /^-?\d+$/.test(value.trim())
        ? Number(value.trim())
        : value;

    if (typeof normalized !== 'number' || !Number.isInteger(normalized)) {
        throw new Error(`${field} must be an integer`);
    }

    if (normalized < min || (max !== undefined && normalized > max)) {
        const maxMessage = max === undefined ? `at least ${min}` : `between ${min} and ${max}`;
        throw new Error(`${field} must be ${maxMessage}`);
    }

    return normalized;
}

function cleanRequiredInteger(value: unknown, field: string, min: number, max?: number): number {
    const cleaned = cleanInteger(value, field, min, max);
    if (cleaned === undefined) {
        throw new Error(`${field} is required`);
    }

    return cleaned;
}

function cleanNumber(value: unknown, field: string, min: number, max: number): number | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    const normalized = typeof value === 'string' && value.trim() !== ''
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

function cleanMarket(value: unknown, prefix = ''): string {
    const market = cleanRequiredString(value, `${prefix}market`, 30).toLowerCase();
    if (!/^[a-z0-9]+$/.test(market)) {
        throw new Error(`${prefix}market must contain only lowercase letters and numbers`);
    }

    return market;
}

function cleanCommaSeparatedPropertyTypes(value: unknown, field: string): string | undefined {
    const propertyTypes = cleanString(value, field, 20);
    if (propertyTypes === undefined) {
        return undefined;
    }

    if (!/^([1-8](,[1-8])*)?$/.test(propertyTypes)) {
        throw new Error(`${field} must be comma-separated numbers 1-8`);
    }

    return propertyTypes;
}

function addIfDefined(params: Record<string, unknown>, key: string, value: unknown): void {
    if (value !== undefined) {
        params[key] = value;
    }
}

function cleanEnumInteger<T extends readonly number[]>(value: unknown, field: string, values: T): T[number] | undefined {
    const cleaned = cleanInteger(value, field, 0);
    if (cleaned === undefined) {
        return undefined;
    }

    if (!values.includes(cleaned)) {
        throw new Error(`${field} must be one of: ${values.join(', ')}`);
    }

    return cleaned;
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function buildSingleRedfinPropertySearchParams(
    input: RedfinPropertySearchParamsInput,
    prefix = '',
): Record<string, unknown> {
    const regionId = cleanRequiredInteger(input.region_id, `${prefix}region_id`, 1);
    const regionType = cleanEnumInteger(input.region_type, `${prefix}region_type`, VALID_REGION_TYPES);
    if (regionType === undefined) {
        throw new Error(`${prefix}region_type is required`);
    }

    const params: Record<string, unknown> = {
        region_id: regionId,
        region_type: regionType,
        market: cleanMarket(input.market, prefix),
    };

    const minPrice = cleanInteger(input.min_price, `${prefix}min_price`, 0);
    const maxPrice = cleanInteger(input.max_price, `${prefix}max_price`, 0);
    if (minPrice !== undefined && maxPrice !== undefined && maxPrice < minPrice) {
        throw new Error(`${prefix}max_price must be greater than or equal to min_price`);
    }

    addIfDefined(params, 'min_price', minPrice);
    addIfDefined(params, 'max_price', maxPrice);
    addIfDefined(params, 'num_beds', cleanInteger(input.num_beds, `${prefix}num_beds`, 0, 10));
    addIfDefined(params, 'num_baths', cleanNumber(input.num_baths, `${prefix}num_baths`, 0, 10));
    addIfDefined(params, 'property_types', cleanCommaSeparatedPropertyTypes(input.property_types, `${prefix}property_types`));
    addIfDefined(params, 'status', cleanEnumInteger(input.status, `${prefix}status`, VALID_STATUSES));
    addIfDefined(params, 'sold_within_days', cleanInteger(input.sold_within_days, `${prefix}sold_within_days`, 1, 365));
    addIfDefined(params, 'num_homes', cleanInteger(input.num_homes, `${prefix}num_homes`, 1, MAX_NUM_HOMES));
    addIfDefined(params, 'page', cleanInteger(input.page, `${prefix}page`, 1));

    return params;
}

export function buildRedfinPropertySearchRequests(input: RedfinPropertySearchInput): RedfinPropertySearchRequest[] {
    if (Array.isArray(input.searches)) {
        if (input.searches.length === 0) {
            throw new Error('searches must include at least one search');
        }
        if (input.searches.length > MAX_SEARCHES_PER_RUN) {
            throw new Error(`searches cannot include more than ${MAX_SEARCHES_PER_RUN} searches per run`);
        }

        return input.searches.map((search, index) => {
            if (!isRecord(search)) {
                throw new Error(`searches[${index}] must be an object`);
            }

            return {
                params: buildSingleRedfinPropertySearchParams(search, `searches[${index}].`),
                index,
            };
        });
    }

    if (input.searches !== undefined) {
        throw new Error('searches must be an array of search objects');
    }

    return [{
        params: buildSingleRedfinPropertySearchParams(input),
        index: 0,
    }];
}

export function describeRedfinPropertySearchRequest(params: Record<string, unknown>): string {
    const required = `region ${String(params.region_id)} (${String(params.market)}, type ${String(params.region_type)})`;
    const excludedFields = new Set(['region_id', 'region_type', 'market']);
    const filters = Object.keys(params)
        .filter((field) => !excludedFields.has(field))
        .sort()
        .map((field) => `${field}=${String(params[field])}`);

    return filters.length > 0 ? `${required} (${filters.join(', ')})` : required;
}

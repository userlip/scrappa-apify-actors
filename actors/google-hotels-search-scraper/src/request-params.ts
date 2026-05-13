export interface GoogleHotelsSearchInput {
    q?: unknown;
    check_in_date?: unknown;
    check_out_date?: unknown;
    adults?: unknown;
    children?: unknown;
    children_ages?: unknown;
    currency?: unknown;
    gl?: unknown;
    hl?: unknown;
    sort_by?: unknown;
    min_price?: unknown;
    max_price?: unknown;
    hotel_class?: unknown;
    rating?: unknown;
    free_cancellation?: unknown;
    amenities?: unknown;
    vacation_rentals?: unknown;
    eco_certified?: unknown;
    special_offers?: unknown;
    property_types?: unknown;
    brands?: unknown;
    bedrooms?: unknown;
    bathrooms?: unknown;
    next_page_token?: unknown;
    property_token?: unknown;
}

const SORT_BY_VALUES = [3, 8, 13] as const;
const HOTEL_CLASS_VALUES = [2, 3, 4, 5] as const;
const RATING_VALUES = [7, 8, 9] as const;
const MAX_PRICE = 5000;

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

function cleanDate(value: unknown, field: string, options: { futureOrToday?: boolean } = {}): string {
    const date = cleanRequiredString(value, field, 10);
    if (!/^\d{4}-\d{2}-\d{2}$/.test(date)) {
        throw new Error(`${field} must use YYYY-MM-DD format`);
    }

    const parsed = new Date(`${date}T00:00:00.000Z`);
    if (Number.isNaN(parsed.getTime()) || parsed.toISOString().slice(0, 10) !== date) {
        throw new Error(`${field} must be a valid calendar date`);
    }

    if (options.futureOrToday) {
        const today = new Date().toISOString().slice(0, 10);
        if (date < today) {
            throw new Error(`${field} must be today or a future date`);
        }
    }

    return date;
}

function cleanInteger(value: unknown, field: string, min: number, max?: number): number | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    if (typeof value !== 'number' || !Number.isInteger(value)) {
        throw new Error(`${field} must be an integer`);
    }

    if (value < min || (max !== undefined && value > max)) {
        throw new Error(max === undefined
            ? `${field} must be greater than or equal to ${min}`
            : `${field} must be between ${min} and ${max}`);
    }

    return value;
}

function cleanBoolean(value: unknown, field: string): boolean | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    if (typeof value !== 'boolean') {
        throw new Error(`${field} must be true or false`);
    }

    return value;
}

function cleanCode(value: unknown, field: string, length: number): string | undefined {
    const cleaned = cleanString(value, field, length);
    if (cleaned === undefined) {
        return undefined;
    }

    if (cleaned.length !== length || !/^[a-z]+$/i.test(cleaned)) {
        throw new Error(`${field} must be a ${length}-letter code`);
    }

    return length === 3 ? cleaned.toUpperCase() : cleaned.toLowerCase();
}

function cleanEnumNumber<T extends readonly number[]>(
    value: unknown,
    field: string,
    allowedValues: T,
): T[number] | undefined {
    const normalized = typeof value === 'string' && value.trim() !== '' && /^-?\d+$/.test(value.trim())
        ? Number(value.trim())
        : value;
    const cleaned = cleanInteger(normalized, field, Math.min(...allowedValues), Math.max(...allowedValues));
    if (cleaned === undefined) {
        return undefined;
    }

    if (!allowedValues.includes(cleaned)) {
        throw new Error(`${field} must be one of: ${allowedValues.join(', ')}`);
    }

    return cleaned as T[number];
}

function cleanIntegerArray(value: unknown, field: string, min?: number, max?: number): number[] | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    let values: unknown[];
    if (Array.isArray(value)) {
        values = value;
    } else if (typeof value === 'string') {
        const parts = value.split(',').map((part) => part.trim());
        values = parts.map((part, index) => {
            if (part === '' || !/^-?\d+$/.test(part)) {
                throw new Error(`${field}[${index}] must be an integer`);
            }

            return Number(part);
        });
    } else {
        throw new Error(`${field} must be an array of integers or a comma-separated string`);
    }

    const integers = values.map((entry, index) => {
        if (typeof entry !== 'number' || !Number.isInteger(entry)) {
            throw new Error(`${field}[${index}] must be an integer`);
        }
        if (min !== undefined && entry < min) {
            throw new Error(`${field}[${index}] must be greater than or equal to ${min}`);
        }
        if (max !== undefined && entry > max) {
            throw new Error(`${field}[${index}] must be less than or equal to ${max}`);
        }
        return entry;
    });

    return integers.length > 0 ? integers : undefined;
}

function setParam(params: Record<string, unknown>, field: string, value: unknown): void {
    if (value === undefined || value === false) {
        return;
    }

    params[field] = Array.isArray(value) ? value.join(',') : value;
}

export function buildGoogleHotelsSearchParams(input: GoogleHotelsSearchInput): Record<string, unknown> {
    const params: Record<string, unknown> = {
        q: cleanRequiredString(input.q, 'q', 200),
        check_in_date: cleanDate(input.check_in_date, 'check_in_date', { futureOrToday: true }),
        check_out_date: cleanDate(input.check_out_date, 'check_out_date'),
    };

    if (String(params.check_out_date) <= String(params.check_in_date)) {
        throw new Error('check_out_date must be after check_in_date');
    }

    const adults = cleanInteger(input.adults, 'adults', 1, 10);
    const children = cleanInteger(input.children, 'children', 0, 6);
    const childrenAges = cleanIntegerArray(input.children_ages, 'children_ages', 1, 17);
    if ((children ?? 0) === 0 && childrenAges !== undefined) {
        throw new Error('children_ages requires children to be greater than 0');
    }
    if (children !== undefined && children > 0 && (childrenAges?.length ?? 0) !== children) {
        throw new Error('children_ages length must match children');
    }

    const minPrice = cleanInteger(input.min_price, 'min_price', 0);
    const maxPrice = cleanInteger(input.max_price, 'max_price', 1, MAX_PRICE);
    if (minPrice !== undefined && maxPrice !== undefined) {
        if (maxPrice <= minPrice) {
            throw new Error('max_price must be greater than min_price');
        }
        if (maxPrice - minPrice > 5000) {
            throw new Error('max_price cannot be more than 5000 above min_price');
        }
    }

    const vacationRentals = cleanBoolean(input.vacation_rentals, 'vacation_rentals');
    const booleans = {
        free_cancellation: cleanBoolean(input.free_cancellation, 'free_cancellation'),
        eco_certified: cleanBoolean(input.eco_certified, 'eco_certified'),
        special_offers: cleanBoolean(input.special_offers, 'special_offers'),
    };
    const hasBooleanFilter = Object.values(booleans).some((value) => value === true);

    const filterParams = {
        min_price: minPrice,
        max_price: maxPrice,
        hotel_class: cleanEnumNumber(input.hotel_class, 'hotel_class', HOTEL_CLASS_VALUES),
        rating: cleanEnumNumber(input.rating, 'rating', RATING_VALUES),
        amenities: cleanIntegerArray(input.amenities, 'amenities'),
        property_types: cleanIntegerArray(input.property_types, 'property_types'),
        brands: cleanIntegerArray(input.brands, 'brands', 1),
    };
    const activeOtherFilters = Object.entries(filterParams)
        .filter(([, value]) => value !== undefined)
        .map(([field]) => field);

    if (hasBooleanFilter && activeOtherFilters.length > 0) {
        throw new Error(`Boolean filters cannot be combined with other filters: ${activeOtherFilters.join(', ')}`);
    }

    if (vacationRentals === true) {
        if (booleans.free_cancellation === true || booleans.eco_certified === true || booleans.special_offers === true) {
            throw new Error('free_cancellation, eco_certified, and special_offers are not available for vacation_rentals');
        }
        if (filterParams.hotel_class !== undefined || filterParams.brands !== undefined) {
            throw new Error('hotel_class and brands are not available for vacation_rentals');
        }
    } else if (input.bedrooms !== undefined || input.bathrooms !== undefined) {
        throw new Error('bedrooms and bathrooms are only available when vacation_rentals is true');
    }

    setParam(params, 'adults', adults);
    setParam(params, 'children', children);
    setParam(params, 'children_ages', childrenAges);
    setParam(params, 'currency', cleanCode(input.currency, 'currency', 3));
    setParam(params, 'gl', cleanCode(input.gl, 'gl', 2));
    setParam(params, 'hl', cleanCode(input.hl, 'hl', 2));
    setParam(params, 'sort_by', cleanEnumNumber(input.sort_by, 'sort_by', SORT_BY_VALUES));
    setParam(params, 'vacation_rentals', vacationRentals);
    setParam(params, 'bedrooms', cleanInteger(input.bedrooms, 'bedrooms', 1, 20));
    setParam(params, 'bathrooms', cleanInteger(input.bathrooms, 'bathrooms', 1, 20));
    setParam(params, 'next_page_token', cleanString(input.next_page_token, 'next_page_token', 3000));
    setParam(params, 'property_token', cleanString(input.property_token, 'property_token', 3000));

    for (const [field, value] of Object.entries(booleans)) {
        setParam(params, field, value);
    }
    for (const [field, value] of Object.entries(filterParams)) {
        setParam(params, field, value);
    }

    return params;
}

export function describeGoogleHotelsSearchRequest(params: Record<string, unknown>): string {
    const parts = [
        `"${String(params.q ?? 'unknown location')}"`,
        `${String(params.check_in_date)} to ${String(params.check_out_date)}`,
    ];
    const excludedFields = new Set(['q', 'check_in_date', 'check_out_date']);
    const filters = Object.keys(params)
        .filter((field) => !excludedFields.has(field) && params[field] !== undefined)
        .sort()
        .map((field) => `${field}=${String(params[field])}`);

    return filters.length > 0 ? `${parts.join(' ')} (${filters.join(', ')})` : parts.join(' ');
}

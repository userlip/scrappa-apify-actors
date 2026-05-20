export interface BookingSearchParamsInput {
    ss?: unknown;
    checkin?: unknown;
    checkout?: unknown;
    group_adults?: unknown;
    group_children?: unknown;
    no_rooms?: unknown;
    lang?: unknown;
    currency?: unknown;
}

export interface BookingSearchInput extends BookingSearchParamsInput {
    searches?: unknown;
}

export interface BookingSearchRequest {
    params: Record<string, unknown>;
    index: number;
}

const MAX_SEARCHES_PER_RUN = 25;

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

function cleanDate(value: unknown, field: string): string | undefined {
    const date = cleanString(value, field, 10);
    if (date === undefined) {
        return undefined;
    }

    if (!/^\d{4}-\d{2}-\d{2}$/.test(date)) {
        throw new Error(`${field} must use YYYY-MM-DD format`);
    }

    const parsed = new Date(`${date}T00:00:00.000Z`);
    if (Number.isNaN(parsed.getTime()) || parsed.toISOString().slice(0, 10) !== date) {
        throw new Error(`${field} must be a valid calendar date`);
    }

    const today = new Date().toISOString().slice(0, 10);
    if (date < today) {
        throw new Error(`${field} must be today or a future date`);
    }

    return date;
}

function cleanInteger(value: unknown, field: string, min: number, max: number): number | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

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

function cleanLanguage(value: unknown): string | undefined {
    const lang = cleanString(value, 'lang', 10);
    if (lang === undefined) {
        return undefined;
    }

    const normalized = lang.toLowerCase();
    if (!/^[a-z]{2}(-[a-z]{2})?$/.test(normalized)) {
        throw new Error('lang must be a valid language code such as en, en-us, de, or fr');
    }

    return normalized;
}

function cleanCurrency(value: unknown): string | undefined {
    const currency = cleanString(value, 'currency', 20);
    if (currency === undefined) {
        return undefined;
    }

    const normalized = currency.toUpperCase();
    if (!/^[A-Z]{3}$/.test(normalized)) {
        throw new Error('currency must be a 3-letter currency code such as USD, EUR, or GBP');
    }

    return normalized;
}

function addIfDefined(params: Record<string, unknown>, key: string, value: unknown): void {
    if (value !== undefined) {
        params[key] = value;
    }
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function buildSingleBookingSearchParams(input: BookingSearchParamsInput, prefix = ''): Record<string, unknown> {
    const params: Record<string, unknown> = {
        ss: cleanRequiredString(input.ss, `${prefix}ss`, 200),
    };

    const checkin = cleanDate(input.checkin, `${prefix}checkin`);
    const checkout = cleanDate(input.checkout, `${prefix}checkout`);
    if ((checkin === undefined) !== (checkout === undefined)) {
        throw new Error(`${prefix}checkin and ${prefix}checkout must be provided together`);
    }
    if (checkin !== undefined && checkout !== undefined && checkout <= checkin) {
        throw new Error(`${prefix}checkout must be after ${prefix}checkin`);
    }

    addIfDefined(params, 'checkin', checkin);
    addIfDefined(params, 'checkout', checkout);
    addIfDefined(params, 'group_adults', cleanInteger(input.group_adults, `${prefix}group_adults`, 1, 30));
    addIfDefined(params, 'group_children', cleanInteger(input.group_children, `${prefix}group_children`, 0, 20));
    addIfDefined(params, 'no_rooms', cleanInteger(input.no_rooms, `${prefix}no_rooms`, 1, 30));
    addIfDefined(params, 'lang', cleanLanguage(input.lang));
    addIfDefined(params, 'currency', cleanCurrency(input.currency));

    return params;
}

export function buildBookingSearchRequests(input: BookingSearchInput): BookingSearchRequest[] {
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
                params: buildSingleBookingSearchParams(search, `searches[${index}].`),
                index,
            };
        });
    }

    if (input.searches !== undefined) {
        throw new Error('searches must be an array of search objects');
    }

    return [{
        params: buildSingleBookingSearchParams(input),
        index: 0,
    }];
}

export function describeBookingSearchRequest(params: Record<string, unknown>): string {
    const core = `"${String(params.ss ?? 'unknown destination')}"`;
    const dates = params.checkin && params.checkout
        ? ` ${String(params.checkin)} to ${String(params.checkout)}`
        : '';
    const excludedFields = new Set(['ss', 'checkin', 'checkout']);
    const filters = Object.keys(params)
        .filter((field) => !excludedFields.has(field))
        .sort()
        .map((field) => `${field}=${String(params[field])}`);

    return filters.length > 0 ? `${core}${dates} (${filters.join(', ')})` : `${core}${dates}`;
}

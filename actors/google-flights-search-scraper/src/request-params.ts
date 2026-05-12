export interface GoogleFlightsSearchInput {
    trip_type?: unknown;
    origin?: unknown;
    destination?: unknown;
    departure_date?: unknown;
    return_date?: unknown;
    adults?: unknown;
    children?: unknown;
    infants_in_seat?: unknown;
    infants_on_lap?: unknown;
    cabin_class?: unknown;
    exclude_basic?: unknown;
    max_stops?: unknown;
    sort_by?: unknown;
    airlines?: unknown;
    include_baggage?: unknown;
    hl?: unknown;
    gl?: unknown;
    currency?: unknown;
    departure_time_min?: unknown;
    departure_time_max?: unknown;
    arrival_time_min?: unknown;
    arrival_time_max?: unknown;
    max_duration_minutes?: unknown;
    max_price?: unknown;
}

export type TripType = 'one_way' | 'round_trip';

export interface GoogleFlightsRequest {
    endpoint: '/flights/one-way' | '/flights/round-trip';
    tripType: TripType;
    params: Record<string, unknown>;
}

const TRIP_TYPES = ['one_way', 'round_trip'] as const;
const CABIN_CLASSES = ['economy', 'premium_economy', 'business', 'first'] as const;
const MAX_STOPS_VALUES = ['any', 'nonstop', 'one_or_fewer', 'two_or_fewer'] as const;
const SORT_BY_VALUES = ['top_flights', 'cheapest', 'departure_time', 'arrival_time', 'duration'] as const;

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

function cleanEnum<T extends readonly string[]>(
    value: unknown,
    field: string,
    values: T,
): T[number] | undefined {
    const cleaned = cleanString(value, field, 40);
    if (cleaned === undefined) {
        return undefined;
    }

    const normalized = cleaned.toLowerCase();
    if (!values.includes(normalized as T[number])) {
        throw new Error(`${field} must be one of: ${values.join(', ')}`);
    }

    return normalized as T[number];
}

function cleanTripType(value: unknown): TripType {
    return cleanEnum(value ?? 'one_way', 'trip_type', TRIP_TYPES) ?? 'one_way';
}

function cleanAirportCode(value: unknown, field: string): string {
    const code = cleanRequiredString(value, field, 3).toUpperCase();
    if (!/^[A-Z]{3}$/.test(code)) {
        throw new Error(`${field} must be a 3-letter IATA airport code`);
    }

    return code;
}

function cleanDate(value: unknown, field: string): string {
    const date = cleanRequiredString(value, field, 10);
    if (!/^\d{4}-\d{2}-\d{2}$/.test(date)) {
        throw new Error(`${field} must be in YYYY-MM-DD format`);
    }

    const parsed = new Date(`${date}T00:00:00Z`);
    if (Number.isNaN(parsed.getTime()) || parsed.toISOString().slice(0, 10) !== date) {
        throw new Error(`${field} must be a valid calendar date`);
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
        throw new Error(`${field} must be a boolean`);
    }

    return value;
}

function cleanLanguageCode(value: unknown): string | undefined {
    const hl = cleanString(value, 'hl', 10);
    if (hl === undefined) {
        return undefined;
    }

    const normalized = hl.toLowerCase();
    if (!/^[a-z]{2}(-[a-z]{2})?$/.test(normalized)) {
        throw new Error('hl must be a valid language code such as en, de, or en-us');
    }

    return normalized;
}

function cleanCountryCode(value: unknown): string | undefined {
    const gl = cleanString(value, 'gl', 2);
    if (gl === undefined) {
        return undefined;
    }

    if (!/^[a-z]{2}$/i.test(gl)) {
        throw new Error('gl must be a two-letter country code');
    }

    return gl.toLowerCase();
}

function cleanCurrency(value: unknown): string | undefined {
    const currency = cleanString(value, 'currency', 3);
    if (currency === undefined) {
        return undefined;
    }

    const normalized = currency.toUpperCase();
    if (!/^[A-Z]{3}$/.test(normalized)) {
        throw new Error('currency must be a 3-letter currency code such as USD, EUR, or GBP');
    }

    return normalized;
}

function cleanAirlines(value: unknown): string | undefined {
    if (value === undefined || value === null || value === '') {
        return undefined;
    }

    const codes = Array.isArray(value)
        ? value
        : typeof value === 'string'
            ? value.split(',')
            : undefined;

    if (!codes) {
        throw new Error('airlines must be an array of airline codes or a comma-separated string');
    }

    const cleaned = codes
        .map((code) => {
            if (typeof code !== 'string') {
                throw new Error('airlines must contain only strings');
            }

            const normalized = code.trim().toUpperCase();
            if (!/^[A-Z0-9]{2}$/.test(normalized)) {
                throw new Error('airlines must contain 2-character IATA airline codes');
            }

            return normalized;
        })
        .filter(Boolean);

    return cleaned.length > 0 ? cleaned.join(',') : undefined;
}

function addIfDefined(params: Record<string, unknown>, key: string, value: unknown): void {
    if (value !== undefined) {
        params[key] = value;
    }
}

export function buildGoogleFlightsRequest(input: GoogleFlightsSearchInput): GoogleFlightsRequest {
    const tripType = cleanTripType(input.trip_type);
    const origin = cleanAirportCode(input.origin, 'origin');
    const destination = cleanAirportCode(input.destination, 'destination');
    if (origin === destination) {
        throw new Error('destination must be different from origin');
    }

    const departureDate = cleanDate(input.departure_date, 'departure_date');
    const returnDate = tripType === 'round_trip' ? cleanDate(input.return_date, 'return_date') : undefined;
    if (returnDate !== undefined && returnDate <= departureDate) {
        throw new Error('return_date must be after departure_date');
    }

    const params: Record<string, unknown> = {
        origin,
        destination,
        departure_date: departureDate,
    };

    addIfDefined(params, 'return_date', returnDate);
    addIfDefined(params, 'adults', cleanInteger(input.adults, 'adults', 1, 9));
    addIfDefined(params, 'children', cleanInteger(input.children, 'children', 0, 9));
    addIfDefined(params, 'infants_in_seat', cleanInteger(input.infants_in_seat, 'infants_in_seat', 0, 9));
    addIfDefined(params, 'infants_on_lap', cleanInteger(input.infants_on_lap, 'infants_on_lap', 0, 9));
    addIfDefined(params, 'cabin_class', cleanEnum(input.cabin_class, 'cabin_class', CABIN_CLASSES));
    addIfDefined(params, 'exclude_basic', cleanBoolean(input.exclude_basic, 'exclude_basic'));
    addIfDefined(params, 'max_stops', cleanEnum(input.max_stops, 'max_stops', MAX_STOPS_VALUES));
    addIfDefined(params, 'sort_by', cleanEnum(input.sort_by, 'sort_by', SORT_BY_VALUES));
    addIfDefined(params, 'airlines', cleanAirlines(input.airlines));
    addIfDefined(params, 'include_baggage', cleanBoolean(input.include_baggage, 'include_baggage'));
    addIfDefined(params, 'hl', cleanLanguageCode(input.hl));
    addIfDefined(params, 'gl', cleanCountryCode(input.gl));
    addIfDefined(params, 'currency', cleanCurrency(input.currency));
    addIfDefined(params, 'departure_time_min', cleanInteger(input.departure_time_min, 'departure_time_min', 0, 23));
    addIfDefined(params, 'departure_time_max', cleanInteger(input.departure_time_max, 'departure_time_max', 0, 23));
    addIfDefined(params, 'arrival_time_min', cleanInteger(input.arrival_time_min, 'arrival_time_min', 0, 23));
    addIfDefined(params, 'arrival_time_max', cleanInteger(input.arrival_time_max, 'arrival_time_max', 0, 23));
    addIfDefined(params, 'max_duration_minutes', cleanInteger(input.max_duration_minutes, 'max_duration_minutes', 1));
    addIfDefined(params, 'max_price', cleanInteger(input.max_price, 'max_price', 1));

    if (typeof params.departure_time_min === 'number'
        && typeof params.departure_time_max === 'number'
        && params.departure_time_max < params.departure_time_min) {
        throw new Error('departure_time_max must be greater than or equal to departure_time_min');
    }

    if (typeof params.arrival_time_min === 'number'
        && typeof params.arrival_time_max === 'number'
        && params.arrival_time_max < params.arrival_time_min) {
        throw new Error('arrival_time_max must be greater than or equal to arrival_time_min');
    }

    return {
        endpoint: tripType === 'round_trip' ? '/flights/round-trip' : '/flights/one-way',
        tripType,
        params,
    };
}

export function describeGoogleFlightsRequest(request: GoogleFlightsRequest): string {
    const tripLabel = request.tripType === 'round_trip' ? 'round-trip' : 'one-way';
    const parts = [
        `${request.params.origin} to ${request.params.destination}`,
        `departing ${request.params.departure_date}`,
    ];

    if (request.params.return_date) {
        parts.push(`returning ${request.params.return_date}`);
    }

    return `${tripLabel} flight search (${parts.join(', ')})`;
}

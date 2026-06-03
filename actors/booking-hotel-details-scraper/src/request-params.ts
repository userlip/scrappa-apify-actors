export interface BookingHotelInput {
    url?: unknown;
    urls?: unknown;
    country?: unknown;
    slug?: unknown;
    hotels?: unknown;
}

export interface BookingHotelRequest {
    params: Record<string, unknown>;
    index: number;
    inputType: 'url' | 'country_slug';
}

const MAX_HOTELS_PER_RUN = 10;
const BOOKING_URL_REGEX = /^https?:\/\/(?:[a-z0-9-]+\.)*booking\.com\/hotel\//i;
const SLUG_REGEX = /^[a-z0-9._-]+(?:\.html)?$/i;

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === 'object' && value !== null && !Array.isArray(value);
}

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

function cleanBookingUrl(value: unknown, field: string): string | undefined {
    const url = cleanString(value, field, 2048);
    if (url === undefined) {
        return undefined;
    }

    let parsed: URL;
    try {
        parsed = new URL(url);
    } catch {
        throw new Error(`${field} must be a valid URL`);
    }

    if (!BOOKING_URL_REGEX.test(parsed.toString())) {
        throw new Error(`${field} must be a Booking.com hotel URL`);
    }

    return parsed.toString();
}

function cleanCountry(value: unknown, field: string): string | undefined {
    const country = cleanString(value, field, 2);
    if (country === undefined) {
        return undefined;
    }

    if (!/^[a-z]{2}$/i.test(country)) {
        throw new Error(`${field} must be a 2-letter country code`);
    }

    return country.toLowerCase();
}

function cleanSlug(value: unknown, field: string): string | undefined {
    const slug = cleanString(value, field, 255);
    if (slug === undefined) {
        return undefined;
    }

    if (slug.length < 2) {
        throw new Error(`${field} must be at least 2 characters`);
    }

    if (!SLUG_REGEX.test(slug)) {
        throw new Error(`${field} must contain only letters, numbers, dots, underscores, and dashes, with optional .html`);
    }

    return slug;
}

function buildSingleBookingHotelRequest(input: Record<string, unknown>, index: number, prefix = ''): BookingHotelRequest {
    const url = cleanBookingUrl(input.url, `${prefix}url`);
    if (url !== undefined) {
        return {
            params: { url },
            index,
            inputType: 'url',
        };
    }

    const country = cleanCountry(input.country, `${prefix}country`);
    const slug = cleanSlug(input.slug, `${prefix}slug`);
    if (country !== undefined || slug !== undefined) {
        if (country === undefined || slug === undefined) {
            throw new Error(`${prefix}country and ${prefix}slug must be provided together`);
        }

        return {
            params: { country, slug },
            index,
            inputType: 'country_slug',
        };
    }

    throw new Error(`${prefix}url or country plus slug is required`);
}

function assertBatchSize(requests: BookingHotelRequest[]): void {
    if (requests.length === 0) {
        throw new Error('Provide at least one hotel URL or country/slug pair');
    }

    if (requests.length > MAX_HOTELS_PER_RUN) {
        throw new Error(`Hotel batches cannot include more than ${MAX_HOTELS_PER_RUN} hotels per run`);
    }
}

export function buildBookingHotelRequests(input: BookingHotelInput): BookingHotelRequest[] {
    const requests: BookingHotelRequest[] = [];

    if (input.url !== undefined || input.country !== undefined || input.slug !== undefined) {
        requests.push(buildSingleBookingHotelRequest(input as Record<string, unknown>, requests.length));
    }

    if (Array.isArray(input.urls)) {
        input.urls.forEach((url, index) => {
            requests.push(buildSingleBookingHotelRequest({ url }, requests.length, `urls[${index}].`));
        });
    } else if (input.urls !== undefined) {
        throw new Error('urls must be an array of Booking.com hotel URLs');
    }

    if (Array.isArray(input.hotels)) {
        input.hotels.forEach((hotel, index) => {
            if (!isRecord(hotel)) {
                throw new Error(`hotels[${index}] must be an object`);
            }

            requests.push(buildSingleBookingHotelRequest(hotel, requests.length, `hotels[${index}].`));
        });
    } else if (input.hotels !== undefined) {
        throw new Error('hotels must be an array of hotel request objects');
    }

    assertBatchSize(requests);

    return requests;
}

export function describeBookingHotelRequest(request: BookingHotelRequest): string {
    if (typeof request.params.url === 'string') {
        return request.params.url;
    }

    return `${String(request.params.country)}/${String(request.params.slug)}`;
}

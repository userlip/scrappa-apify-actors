export interface GoogleHotelsSearchResponse {
    properties?: GoogleHotelProperty[];
    brands?: unknown[];
    pagination?: {
        next_page_token?: string;
        next?: string;
        [key: string]: unknown;
    };
    data?: {
        hotels?: GoogleHotelProperty[];
        [key: string]: unknown;
    };
    [key: string]: unknown;
}

export interface GoogleHotelProperty {
    type?: string;
    name?: string;
    link?: string;
    property_link?: string;
    property_token?: string;
    entity_id?: string;
    place_id?: string;
    gps_coordinates?: {
        latitude?: number;
        longitude?: number;
        [key: string]: unknown;
    };
    rate_per_night?: {
        lowest?: string;
        extracted_lowest?: number;
        [key: string]: unknown;
    };
    total_rate?: {
        lowest?: string;
        extracted_lowest?: number;
        [key: string]: unknown;
    };
    overall_rating?: number;
    reviews?: number;
    hotel_class?: string;
    extracted_hotel_class?: number;
    thumbnail?: string;
    prices?: unknown[];
    amenities?: unknown[];
    [key: string]: unknown;
}

export function getHotelProperties(response: GoogleHotelsSearchResponse): GoogleHotelProperty[] {
    if (Array.isArray(response.properties)) {
        return response.properties;
    }

    if (Array.isArray(response.data?.hotels)) {
        return response.data.hotels;
    }

    return [];
}

export function buildHotelDatasetItem(
    hotel: GoogleHotelProperty,
    params: Record<string, unknown>,
): Record<string, unknown> {
    return {
        ...hotel,
        name: hotel.name ?? null,
        property_token: hotel.property_token ?? hotel.entity_id ?? null,
        entity_id: hotel.entity_id ?? null,
        place_id: hotel.place_id ?? null,
        latitude: hotel.gps_coordinates?.latitude ?? null,
        longitude: hotel.gps_coordinates?.longitude ?? null,
        rate_per_night_lowest: hotel.rate_per_night?.lowest ?? null,
        rate_per_night_extracted_lowest: hotel.rate_per_night?.extracted_lowest ?? null,
        total_rate_lowest: hotel.total_rate?.lowest ?? null,
        total_rate_extracted_lowest: hotel.total_rate?.extracted_lowest ?? null,
        booking_link: hotel.property_link ?? hotel.link ?? null,
        price_sources_count: Array.isArray(hotel.prices) ? hotel.prices.length : 0,
        amenities_count: Array.isArray(hotel.amenities) ? hotel.amenities.length : 0,
        request_q: params.q ?? null,
        request_check_in_date: params.check_in_date ?? null,
        request_check_out_date: params.check_out_date ?? null,
        request_adults: params.adults ?? null,
        request_children: params.children ?? null,
        request_currency: params.currency ?? null,
        request_gl: params.gl ?? null,
        request_hl: params.hl ?? null,
        request_sort_by: params.sort_by ?? null,
        request_next_page_token: params.next_page_token ?? null,
        request_property_token: params.property_token ?? null,
    };
}

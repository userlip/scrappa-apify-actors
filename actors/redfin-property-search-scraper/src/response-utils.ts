export interface RedfinPropertyListing {
    property_id?: number | string | null;
    listing_id?: number | string | null;
    address?: string | null;
    city?: string | null;
    state?: string | null;
    zip?: string | null;
    price?: number | string | null;
    beds?: number | string | null;
    baths?: number | string | null;
    sqft?: number | string | null;
    lot_size?: number | string | null;
    year_built?: number | string | null;
    property_type?: number | string | null;
    status?: string | number | null;
    latitude?: number | string | null;
    longitude?: number | string | null;
    url?: string | null;
    mls_number?: string | number | null;
    [key: string]: unknown;
}

export interface RedfinPropertySearchResponse {
    data?: {
        properties?: RedfinPropertyListing[];
        count?: number | string;
        region_id?: number | string;
        region_type?: number | string;
        [key: string]: unknown;
    };
    properties?: RedfinPropertyListing[];
    count?: number | string;
    region_id?: number | string;
    region_type?: number | string;
    [key: string]: unknown;
}

function firstString(...values: unknown[]): string | null {
    for (const value of values) {
        if (typeof value === 'string' && value.trim() !== '') {
            return value;
        }
        if (typeof value === 'number' && Number.isFinite(value)) {
            return String(value);
        }
    }

    return null;
}

function firstNumber(...values: unknown[]): number | null {
    for (const value of values) {
        if (typeof value === 'number' && Number.isFinite(value)) {
            return value;
        }
        if (typeof value === 'string' && value.trim() !== '') {
            const number = Number(value);
            if (Number.isFinite(number)) {
                return number;
            }
        }
    }

    return null;
}

export function getRedfinPropertyListings(response: RedfinPropertySearchResponse): RedfinPropertyListing[] {
    if (Array.isArray(response.data?.properties)) {
        return response.data.properties;
    }

    if (Array.isArray(response.properties)) {
        return response.properties;
    }

    return [];
}

export function getRedfinSearchCount(response: RedfinPropertySearchResponse): number | null {
    return firstNumber(response.data?.count, response.count);
}

export function buildRedfinDatasetItem(
    property: RedfinPropertyListing,
    params: Record<string, unknown>,
    searchIndex: number,
): Record<string, unknown> {
    return {
        ...property,
        property_id: firstNumber(property.property_id),
        listing_id: firstNumber(property.listing_id),
        address: firstString(property.address),
        city: firstString(property.city),
        state: firstString(property.state),
        zip: firstString(property.zip),
        price: firstNumber(property.price),
        beds: firstNumber(property.beds),
        baths: firstNumber(property.baths),
        sqft: firstNumber(property.sqft),
        lot_size: firstNumber(property.lot_size),
        year_built: firstNumber(property.year_built),
        property_type: firstNumber(property.property_type),
        status: firstString(property.status),
        latitude: firstNumber(property.latitude),
        longitude: firstNumber(property.longitude),
        url: firstString(property.url),
        mls_number: firstString(property.mls_number),
        request_search_index: searchIndex,
        request_region_id: params.region_id ?? null,
        request_region_type: params.region_type ?? null,
        request_market: params.market ?? null,
        request_min_price: params.min_price ?? null,
        request_max_price: params.max_price ?? null,
        request_num_beds: params.num_beds ?? null,
        request_num_baths: params.num_baths ?? null,
        request_property_types: params.property_types ?? null,
        request_status: params.status ?? null,
        request_sold_within_days: params.sold_within_days ?? null,
        request_num_homes: params.num_homes ?? null,
        request_page: params.page ?? null,
    };
}

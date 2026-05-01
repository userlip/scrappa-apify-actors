export interface PhotoSample {
    photo_id?: string;
    photo_url?: string;
    photo_url_large?: string;
    video_thumbnail_url?: string;
    latitude?: number;
    longitude?: number;
    type?: string;
}

export interface OpeningHour {
    day?: string;
    hours?: string;
    date?: string;
    special_day?: boolean;
}

export interface GoogleMapsSearchResult {
    name?: string;
    type?: string;
    subtypes?: string[];
    rating?: number;
    review_count?: number;
    price_level?: string;
    price_level_text?: string;
    full_address?: string;
    address?: string;
    district?: string;
    timezone?: string;
    latitude?: number;
    longitude?: number;
    phone_numbers?: string[];
    phone?: string;
    website?: string;
    domain?: string;
    business_id?: string;
    place_id?: string;
    google_mid?: string;
    owner_id?: string;
    owner_name?: string;
    owner_link?: string;
    order_link?: string;
    short_description?: string;
    full_description?: string;
    current_status?: string;
    photos_sample?: PhotoSample[];
    opening_hours?: OpeningHour[];
    [key: string]: unknown;
}

export interface GoogleMapsSearchResponse {
    items?: GoogleMapsSearchResult[];
    [key: string]: unknown;
}

export function addContactAliases(item: GoogleMapsSearchResult): GoogleMapsSearchResult {
    const aliasedItem: GoogleMapsSearchResult = { ...item };

    if (aliasedItem.address === undefined && typeof aliasedItem.full_address === 'string') {
        aliasedItem.address = aliasedItem.full_address;
    }

    if (aliasedItem.phone === undefined && Array.isArray(aliasedItem.phone_numbers)) {
        const phoneNumbers = aliasedItem.phone_numbers.filter((phoneNumber) => phoneNumber.trim() !== '');
        if (phoneNumbers.length > 0) {
            aliasedItem.phone = phoneNumbers.join(', ');
        }
    }

    return aliasedItem;
}

export function addSearchResponseAliases(response: GoogleMapsSearchResponse): GoogleMapsSearchResponse {
    if (!Array.isArray(response.items)) {
        return response;
    }

    return {
        ...response,
        items: response.items.map(addContactAliases),
    };
}

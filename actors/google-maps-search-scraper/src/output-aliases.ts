export interface GoogleMapsSearchResult {
    name?: string;
    price_level?: number | string | null;
    price_level_text?: string | null;
    review_count?: number | null;
    rating?: number | null;
    website?: string | null;
    domain?: string | null;
    latitude?: number | null;
    longitude?: number | null;
    business_id?: string | null;
    subtypes?: string[];
    district?: string | null;
    full_address?: string | null;
    address?: string;
    timezone?: string | null;
    short_description?: string | null;
    full_description?: string | null;
    owner_id?: string | null;
    owner_name?: string | null;
    owner_link?: string | null;
    order_link?: string | null;
    google_mid?: string | null;
    type?: string | null;
    phone_numbers?: string[];
    phone?: string;
    place_id?: string | null;
    photos_sample?: GoogleMapsPhotoSample[];
    opening_hours?: GoogleMapsOpeningHours[];
    current_status?: string | null;
    [key: string]: unknown;
}

export interface GoogleMapsPhotoSample {
    photo_id?: string;
    photo_url?: string;
    photo_url_large?: string;
    video_thumbnail_url?: string | null;
    latitude?: number | null;
    longitude?: number | null;
    type?: string | null;
    [key: string]: unknown;
}

export interface GoogleMapsOpeningHours {
    day?: string;
    hours?: string;
    date?: string;
    special_day?: boolean;
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

export interface GoogleMapsSearchResult {
    full_address?: string;
    address?: string;
    phone_numbers?: string[];
    phone?: string;
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

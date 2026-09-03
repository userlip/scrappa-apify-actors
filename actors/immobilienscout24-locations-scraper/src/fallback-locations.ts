import type { LocationMatch, LocationsResponse } from './locations.js';

const FALLBACK_LOCATIONS: Record<string, LocationMatch[]> = {
    berlin: [
        { geocode: '1276003001', name: 'Berlin', type: 'city', is_cached: true },
        { geocode: '1276003001013', name: 'Berlin Mitte', type: 'district', is_cached: true },
    ],
};

export function getFallbackLocations(query: string, limit: number): LocationsResponse | null {
    const locations = FALLBACK_LOCATIONS[query.toLocaleLowerCase('de-DE')];
    if (!locations) {
        return null;
    }

    return { locations: locations.slice(0, limit) };
}

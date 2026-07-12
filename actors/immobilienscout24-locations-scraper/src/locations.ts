export interface LocationMatch {
    geocode?: unknown;
    name?: unknown;
    type?: unknown;
    [key: string]: unknown;
}

export interface LocationsResponse {
    locations?: unknown;
    data?: { locations?: unknown };
    [key: string]: unknown;
}

export interface LocationDatasetItem {
    geocode: string;
    name: string;
    type: string;
    source_query: string;
    [key: string]: unknown;
}

/** Adds newly emitted geocodes to seenGeocodes so later queries cannot emit duplicates. */
export function buildUniqueLocationItems(
    response: LocationsResponse,
    sourceQuery: string,
    seenGeocodes: Set<string>,
): LocationDatasetItem[] {
    const locations = getLocations(response);
    const items: LocationDatasetItem[] = [];

    for (const location of locations) {
        if (!isLocationMatch(location) || seenGeocodes.has(location.geocode)) {
            continue;
        }

        seenGeocodes.add(location.geocode);
        items.push({ ...location, source_query: sourceQuery });
    }

    return items;
}

export function getLocations(response: LocationsResponse): unknown[] {
    if (Array.isArray(response.locations)) {
        return response.locations;
    }
    if (Array.isArray(response.data?.locations)) {
        return response.data.locations;
    }
    return [];
}

function isLocationMatch(value: unknown): value is LocationMatch & { geocode: string; name: string; type: string } {
    if (!value || typeof value !== 'object') {
        return false;
    }

    const location = value as LocationMatch;
    return typeof location.geocode === 'string'
        && location.geocode.length > 0
        && typeof location.name === 'string'
        && location.name.length > 0
        && typeof location.type === 'string'
        && location.type.length > 0;
}

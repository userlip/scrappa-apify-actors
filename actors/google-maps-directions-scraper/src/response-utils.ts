import type { DirectionsRequest } from './request-params.js';

export interface DirectionsResponse {
    success?: boolean;
    status?: unknown;
    message?: unknown;
    error?: unknown;
    directions?: unknown;
    routes?: unknown;
    data?: unknown;
    search_parameters?: unknown;
    [key: string]: unknown;
}

export type DirectionsAlternative = Record<string, unknown>;

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function nonEmptyString(value: unknown): string | null {
    if (typeof value !== 'string') {
        return null;
    }

    const trimmed = value.trim();
    return trimmed ? trimmed : null;
}

function responseMessage(response: DirectionsResponse): string {
    const message = nonEmptyString(response.message) ?? nonEmptyString(response.error);
    return message ?? 'Scrappa response reported a directions failure';
}

function findAlternativeArray(response: DirectionsResponse): unknown[] | null {
    const candidates: unknown[] = [response.directions, response.routes];
    if (isRecord(response.data)) {
        candidates.push(response.data.directions, response.data.routes, response.data.routes_data);
    } else {
        candidates.push(response.data);
    }

    return candidates.find((candidate) => Array.isArray(candidate)) as unknown[] | null ?? null;
}

export function extractRouteAlternatives(response: DirectionsResponse): DirectionsAlternative[] {
    if (!isRecord(response)) {
        throw new Error('Scrappa response was not an object');
    }
    if (response.success === false) {
        throw new Error(responseMessage(response));
    }

    const status = nonEmptyString(response.status);
    if (status && status.toUpperCase() !== 'OK' && status.toUpperCase() !== 'SUCCESS') {
        throw new Error(`${responseMessage(response)} (status: ${status})`);
    }

    const alternatives = findAlternativeArray(response);
    if (!alternatives) {
        throw new Error('Scrappa response did not include route alternatives');
    }

    const records = alternatives.filter(isRecord);
    if (records.length === 0) {
        throw new Error('Scrappa response included no route alternatives');
    }

    if (records.length !== alternatives.length) {
        throw new Error('Scrappa response included a malformed route alternative');
    }

    return records;
}

function getStepCoordinates(alternative: DirectionsAlternative): unknown[] | undefined {
    const coordinates: unknown[] = [];
    const trips = alternative.trips;
    if (!Array.isArray(trips)) {
        return undefined;
    }

    for (const trip of trips) {
        if (!isRecord(trip) || !Array.isArray(trip.details)) {
            continue;
        }
        for (const detail of trip.details) {
            if (isRecord(detail) && isRecord(detail.gps_coordinates)) {
                coordinates.push(detail.gps_coordinates);
            }
        }
    }

    return coordinates.length > 0 ? coordinates : undefined;
}

function getSearchParameterMode(response: DirectionsResponse): unknown {
    return isRecord(response.search_parameters) ? response.search_parameters.travel_mode : undefined;
}

export function buildDirectionsDatasetRows(
    response: DirectionsResponse,
    request: DirectionsRequest,
): Record<string, unknown>[] {
    return extractRouteAlternatives(response).map((alternative, alternativeIndex) => {
        const row: Record<string, unknown> = {
            ...alternative,
            alternative_index: alternativeIndex,
            request_index: request.index,
            request_origin: request.origin,
            request_destination: request.destination,
            request_mode: request.mode,
            request_hl: request.hl,
        };

        if (request.gl !== undefined) {
            row.request_gl = request.gl;
        }

        if (alternative.travel_mode === undefined) {
            const responseMode = getSearchParameterMode(response);
            if (responseMode !== undefined) {
                row.travel_mode = responseMode;
            }
        }

        const stepCoordinates = getStepCoordinates(alternative);
        if (stepCoordinates !== undefined) {
            row.step_coordinates = stepCoordinates;
        }

        return row;
    });
}

import type { TripType } from './request-params.js';

export interface GoogleFlightsSearchResponse {
    flights?: FlightResult[];
    search_metadata?: Record<string, unknown>;
    baggage_info?: unknown;
    [key: string]: unknown;
}

export interface FlightLeg {
    departure_airport?: string | null;
    arrival_airport?: string | null;
    departure_time?: string | null;
    arrival_time?: string | null;
    duration_minutes?: number | null;
    airline?: string | null;
    airline_name?: string | null;
    flight_number?: string | null;
    stops?: number | null;
    aircraft?: string | null;
    [key: string]: unknown;
}

export interface FlightResult {
    price?: number | string | null;
    currency?: string | null;
    total_duration_minutes?: number | null;
    legs?: FlightLeg[];
    booking_token?: string | null;
    [key: string]: unknown;
}

function firstString(...values: unknown[]): string | null {
    for (const value of values) {
        if (typeof value === 'string' && value.trim() !== '') {
            return value;
        }
    }

    return null;
}

function firstNumber(...values: unknown[]): number | null {
    for (const value of values) {
        if (typeof value === 'number' && Number.isFinite(value)) {
            return value;
        }

        if (typeof value === 'string') {
            const parsed = Number(value.replace(/,/g, '').trim());
            if (Number.isFinite(parsed)) {
                return parsed;
            }
        }
    }

    return null;
}

function countStops(legs: FlightLeg[]): number | null {
    const stopCounts = legs
        .map((leg) => firstNumber(leg.stops))
        .filter((value): value is number => value !== null);

    if (stopCounts.length > 0) {
        return Math.max(...stopCounts);
    }

    return legs.length > 0 ? Math.max(0, legs.length - 1) : null;
}

function legFlightNumbers(legs: FlightLeg[]): string[] {
    return legs
        .map((leg) => firstString(leg.flight_number))
        .filter((value): value is string => value !== null);
}

function flightAirlines(flight: FlightResult, legs: FlightLeg[]): string[] {
    const topLevelAirline = firstString(flight.airline_name, flight.airline);

    return [...new Set([
        topLevelAirline,
        ...legs
        .map((leg) => firstString(leg.airline_name, topLevelAirline ? undefined : leg.airline))
    ].filter((value): value is string => value !== null))];
}

export function getFlights(response: GoogleFlightsSearchResponse): FlightResult[] {
    if (Array.isArray(response.flights)) {
        return response.flights;
    }

    console.debug('Unexpected Google Flights response shape: expected "flights" array.');
    return [];
}

export function buildFlightDatasetItems(
    response: GoogleFlightsSearchResponse,
    params: Record<string, unknown>,
    tripType: TripType,
): Record<string, unknown>[] {
    const metadata = response.search_metadata ?? {};

    return getFlights(response).map((flight, index) => {
        const legs = Array.isArray(flight.legs) ? flight.legs : [];
        const firstLeg = legs[0] ?? {};
        const lastLeg = legs[legs.length - 1] ?? {};

        return {
            position: index + 1,
            trip_type: tripType,
            price: firstNumber(flight.price),
            currency: firstString(flight.currency, params.currency),
            total_duration_minutes: firstNumber(flight.total_duration_minutes),
            stops: countStops(legs),
            airline_names: flightAirlines(flight, legs),
            flight_numbers: legFlightNumbers(legs),
            departure_airport: firstString(firstLeg.departure_airport, params.origin),
            arrival_airport: firstString(lastLeg.arrival_airport, params.destination),
            departure_time: firstString(firstLeg.departure_time),
            arrival_time: firstString(lastLeg.arrival_time),
            booking_token: firstString(flight.booking_token),
            legs,
            raw_flight: flight,
            search_metadata: metadata,
            request_origin: params.origin,
            request_destination: params.destination,
            request_departure_date: params.departure_date,
            request_return_date: params.return_date ?? null,
            request_cabin_class: params.cabin_class ?? null,
            request_max_stops: params.max_stops ?? null,
            request_sort_by: params.sort_by ?? null,
            request_airlines: params.airlines ?? null,
            request_hl: params.hl ?? null,
            request_gl: params.gl ?? null,
        };
    });
}

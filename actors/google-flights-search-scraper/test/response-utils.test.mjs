import assert from 'node:assert/strict';
import test from 'node:test';

import { buildFlightDatasetItems, getFlights } from '../dist/response-utils.js';

test('extracts flights from the Scrappa response', () => {
    const flights = [{ price: 123 }];
    assert.equal(getFlights({ flights }), flights);
    assert.deepEqual(getFlights({}), []);
});

test('builds one dataset item per flight', () => {
    const items = buildFlightDatasetItems(
        {
            flights: [
                {
                    price: '326',
                    currency: 'USD',
                    total_duration_minutes: 374,
                    airline_name: 'Delta',
                    booking_token: 'token-1',
                    legs: [
                        {
                            departure_airport: 'JFK',
                            arrival_airport: 'LAX',
                            departure_time: '2026-09-15T08:00:00',
                            arrival_time: '2026-09-15T11:14:00',
                            airline: 'DL',
                            flight_number: 'DL123',
                            stops: 0,
                        },
                    ],
                },
            ],
            search_metadata: {
                response_time_ms: 1200,
            },
        },
        {
            origin: 'JFK',
            destination: 'LAX',
            departure_date: '2026-09-15',
            return_date: '2026-09-22',
            cabin_class: 'economy',
            max_stops: 'nonstop',
            sort_by: 'cheapest',
        },
        'round_trip',
    );

    assert.equal(items.length, 1);
    assert.equal(items[0].position, 1);
    assert.equal(items[0].trip_type, 'round_trip');
    assert.equal(items[0].price, 326);
    assert.equal(items[0].stops, 0);
    assert.deepEqual(items[0].airline_names, ['Delta']);
    assert.deepEqual(items[0].flight_numbers, ['DL123']);
    assert.equal(items[0].departure_airport, 'JFK');
    assert.equal(items[0].arrival_airport, 'LAX');
    assert.equal(items[0].booking_token, 'token-1');
    assert.deepEqual(items[0].search_metadata, { response_time_ms: 1200 });
    assert.equal(items[0].request_return_date, '2026-09-22');
    assert.equal(Object.hasOwn(items[0], 'raw_flight'), false);
});

test('preserves missing numeric strings as null', () => {
    const items = buildFlightDatasetItems(
        {
            flights: [
                {
                    price: '   ',
                    total_duration_minutes: '',
                    legs: [
                        {
                            stops: '',
                        },
                    ],
                },
            ],
        },
        {
            origin: 'JFK',
            destination: 'LAX',
            departure_date: '2026-09-15',
        },
        'one_way',
    );

    assert.equal(items[0].price, null);
    assert.equal(items[0].total_duration_minutes, null);
    assert.equal(items[0].stops, 0);
});

test('uses outbound legs for round-trip display route fields', () => {
    const items = buildFlightDatasetItems(
        {
            flights: [
                {
                    price: 450,
                    legs: [
                        {
                            departure_airport: 'JFK',
                            arrival_airport: 'LAX',
                            departure_time: '2026-09-15T08:00:00',
                            arrival_time: '2026-09-15T11:00:00',
                        },
                        {
                            departure_airport: 'LAX',
                            arrival_airport: 'JFK',
                            departure_time: '2026-09-22T15:00:00',
                            arrival_time: '2026-09-22T23:00:00',
                        },
                    ],
                    outbound_legs: [
                        {
                            departure_airport: 'JFK',
                            arrival_airport: 'LAX',
                            departure_time: '2026-09-15T08:00:00',
                            arrival_time: '2026-09-15T11:00:00',
                        },
                    ],
                    return_legs: [
                        {
                            departure_airport: 'LAX',
                            arrival_airport: 'JFK',
                            departure_time: '2026-09-22T15:00:00',
                            arrival_time: '2026-09-22T23:00:00',
                        },
                    ],
                },
            ],
        },
        {
            origin: 'JFK',
            destination: 'LAX',
            departure_date: '2026-09-15',
            return_date: '2026-09-22',
        },
        'round_trip',
    );

    assert.equal(items[0].departure_airport, 'JFK');
    assert.equal(items[0].arrival_airport, 'LAX');
    assert.equal(items[0].departure_time, '2026-09-15T08:00:00');
    assert.equal(items[0].arrival_time, '2026-09-15T11:00:00');
    assert.equal(items[0].stops, 0);
    assert.deepEqual(items[0].return_legs, [
        {
            departure_airport: 'LAX',
            arrival_airport: 'JFK',
            departure_time: '2026-09-22T15:00:00',
            arrival_time: '2026-09-22T23:00:00',
        },
    ]);
});

test('derives leg fields from segmented round-trip legs when flat legs are missing', () => {
    const items = buildFlightDatasetItems(
        {
            flights: [
                {
                    price: 475,
                    airline_name: 'Delta',
                    outbound_legs: [
                        {
                            departure_airport: 'JFK',
                            arrival_airport: 'ATL',
                            airline: 'DL',
                            flight_number: 'DL100',
                        },
                        {
                            departure_airport: 'ATL',
                            arrival_airport: 'LAX',
                            airline: 'DL',
                            flight_number: 'DL200',
                        },
                    ],
                    return_legs: [
                        {
                            departure_airport: 'LAX',
                            arrival_airport: 'JFK',
                            airline: 'DL',
                            flight_number: 'DL300',
                        },
                    ],
                },
            ],
        },
        {
            origin: 'JFK',
            destination: 'LAX',
            departure_date: '2026-09-15',
            return_date: '2026-09-22',
        },
        'round_trip',
    );

    assert.deepEqual(items[0].airline_names, ['Delta']);
    assert.deepEqual(items[0].flight_numbers, ['DL100', 'DL200', 'DL300']);
    assert.equal(items[0].stops, 1);
});

test('falls back to request route when leg airports are missing', () => {
    const items = buildFlightDatasetItems(
        {
            flights: [
                {
                    price: 500,
                    legs: [{}],
                },
            ],
        },
        {
            origin: 'SFO',
            destination: 'CDG',
            departure_date: '2026-10-01',
        },
        'one_way',
    );

    assert.equal(items[0].departure_airport, 'SFO');
    assert.equal(items[0].arrival_airport, 'CDG');
    assert.equal(items[0].request_return_date, null);
});

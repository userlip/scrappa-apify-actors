import assert from 'node:assert/strict';
import test from 'node:test';

import { buildGoogleFlightsRequest, describeGoogleFlightsRequest } from '../dist/request-params.js';

test('builds params for a complete round-trip search', () => {
    assert.deepEqual(
        buildGoogleFlightsRequest({
            trip_type: 'round_trip',
            origin: ' jfk ',
            destination: ' lax ',
            departure_date: '2026-09-15',
            return_date: '2026-09-22',
            adults: 2,
            children: 1,
            cabin_class: 'BUSINESS',
            exclude_basic: true,
            max_stops: 'one_or_fewer',
            sort_by: 'cheapest',
            airlines: [' dl ', 'AA'],
            include_baggage: true,
            currency: 'usd',
            hl: 'EN',
            gl: 'US',
            departure_time_min: 6,
            departure_time_max: 18,
            arrival_time_min: 9,
            arrival_time_max: 22,
            max_duration_minutes: 600,
            max_price: 800,
        }),
        {
            endpoint: '/flights/round-trip',
            tripType: 'round_trip',
            params: {
                origin: 'JFK',
                destination: 'LAX',
                departure_date: '2026-09-15',
                return_date: '2026-09-22',
                adults: 2,
                children: 1,
                cabin_class: 'business',
                exclude_basic: true,
                max_stops: 'one_or_fewer',
                sort_by: 'cheapest',
                airlines: 'DL,AA',
                include_baggage: true,
                hl: 'en',
                gl: 'us',
                currency: 'USD',
                departure_time_min: 6,
                departure_time_max: 18,
                arrival_time_min: 9,
                arrival_time_max: 22,
                max_duration_minutes: 600,
                max_price: 800,
            },
        },
    );
});

test('builds params for a default one-way search', () => {
    assert.deepEqual(
        buildGoogleFlightsRequest({
            origin: 'SFO',
            destination: 'CDG',
            departure_date: '2026-10-01',
            adults: 1,
        }),
        {
            endpoint: '/flights/one-way',
            tripType: 'one_way',
            params: {
                origin: 'SFO',
                destination: 'CDG',
                departure_date: '2026-10-01',
                adults: 1,
            },
        },
    );
});

test('requires valid route and date controls', () => {
    assert.throws(
        () => buildGoogleFlightsRequest({ origin: 'JF', destination: 'LAX', departure_date: '2026-09-15' }),
        /origin must be a 3-letter IATA airport code/,
    );
    assert.throws(
        () => buildGoogleFlightsRequest({ origin: 'JFK', destination: 'JFK', departure_date: '2026-09-15' }),
        /destination must be different from origin/,
    );
    assert.throws(
        () => buildGoogleFlightsRequest({ trip_type: 'round_trip', origin: 'JFK', destination: 'LAX', departure_date: '2026-09-15' }),
        /return_date is required/,
    );
    assert.throws(
        () => buildGoogleFlightsRequest({ trip_type: 'round_trip', origin: 'JFK', destination: 'LAX', departure_date: '2026-09-15', return_date: '2026-09-14' }),
        /return_date must be after departure_date/,
    );
    assert.throws(
        () => buildGoogleFlightsRequest({ origin: 'JFK', destination: 'LAX', departure_date: '2026-02-30' }),
        /departure_date must be a valid calendar date/,
    );
});

test('rejects invalid filters', () => {
    assert.throws(
        () => buildGoogleFlightsRequest({ origin: 'JFK', destination: 'LAX', departure_date: '2026-09-15', adults: 10 }),
        /adults must be between 1 and 9/,
    );
    assert.throws(
        () => buildGoogleFlightsRequest({ origin: 'JFK', destination: 'LAX', departure_date: '2026-09-15', cabin_class: 'coach' }),
        /cabin_class must be one of/,
    );
    assert.throws(
        () => buildGoogleFlightsRequest({ origin: 'JFK', destination: 'LAX', departure_date: '2026-09-15', airlines: 'AAL' }),
        /airlines must contain 2-character IATA airline codes/,
    );
    assert.throws(
        () => buildGoogleFlightsRequest({ origin: 'JFK', destination: 'LAX', departure_date: '2026-09-15', currency: 'US' }),
        /currency must be a 3-letter currency code/,
    );
    assert.throws(
        () => buildGoogleFlightsRequest({ origin: 'JFK', destination: 'LAX', departure_date: '2026-09-15', departure_time_min: 20, departure_time_max: 6 }),
        /departure_time_max must be greater than or equal/,
    );
});

test('describes requests for logs', () => {
    assert.equal(
        describeGoogleFlightsRequest(buildGoogleFlightsRequest({
            origin: 'JFK',
            destination: 'LAX',
            departure_date: '2026-09-15',
        })),
        'one-way flight search (JFK to LAX, departing 2026-09-15)',
    );
    assert.equal(
        describeGoogleFlightsRequest(buildGoogleFlightsRequest({
            trip_type: 'round_trip',
            origin: 'JFK',
            destination: 'LAX',
            departure_date: '2026-09-15',
            return_date: '2026-09-22',
        })),
        'round-trip flight search (JFK to LAX, departing 2026-09-15, returning 2026-09-22)',
    );
});

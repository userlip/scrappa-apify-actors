import assert from 'node:assert/strict';
import test from 'node:test';

import { buildGoogleHotelsSearchParams, describeGoogleHotelsSearchRequest } from '../dist/request-params.js';

test('builds params for a complete hotel search request', () => {
    assert.deepEqual(
        buildGoogleHotelsSearchParams({
            q: ' Paris, France ',
            check_in_date: '2026-06-15',
            check_out_date: '2026-06-18',
            adults: 2,
            children: 1,
            children_ages: [7],
            currency: 'eur',
            gl: 'FR',
            hl: 'EN',
            sort_by: 3,
            min_price: 150,
            max_price: 400,
            hotel_class: 4,
            rating: 8,
            amenities: [35, 9],
            property_types: [17],
            brands: [1, 2],
            property_token: ' token ',
        }),
        {
            q: 'Paris, France',
            check_in_date: '2026-06-15',
            check_out_date: '2026-06-18',
            adults: 2,
            children: 1,
            children_ages: '7',
            currency: 'EUR',
            gl: 'fr',
            hl: 'en',
            sort_by: 3,
            min_price: 150,
            max_price: 400,
            hotel_class: 4,
            rating: 8,
            amenities: '35,9',
            property_types: '17',
            brands: '1,2',
            property_token: 'token',
        },
    );
});

test('requires query and valid date range', () => {
    assert.throws(
        () => buildGoogleHotelsSearchParams({
            q: '',
            check_in_date: '2026-06-15',
            check_out_date: '2026-06-18',
        }),
        /q is required/,
    );
    assert.throws(
        () => buildGoogleHotelsSearchParams({
            q: 'Paris',
            check_in_date: '2026-06-18',
            check_out_date: '2026-06-15',
        }),
        /check_out_date must be after check_in_date/,
    );
    assert.throws(
        () => buildGoogleHotelsSearchParams({
            q: 'Paris',
            check_in_date: '2026-02-30',
            check_out_date: '2026-06-15',
        }),
        /check_in_date must be a valid calendar date/,
    );
});

test('validates guest and price constraints', () => {
    assert.throws(
        () => buildGoogleHotelsSearchParams({
            q: 'Paris',
            check_in_date: '2026-06-15',
            check_out_date: '2026-06-18',
            children: 2,
            children_ages: [7],
        }),
        /children_ages length must match children/,
    );
    assert.throws(
        () => buildGoogleHotelsSearchParams({
            q: 'Paris',
            check_in_date: '2026-06-15',
            check_out_date: '2026-06-18',
            min_price: 500,
            max_price: 200,
        }),
        /max_price must be greater than min_price/,
    );
});

test('rejects unsupported filter combinations', () => {
    assert.throws(
        () => buildGoogleHotelsSearchParams({
            q: 'Paris',
            check_in_date: '2026-06-15',
            check_out_date: '2026-06-18',
            free_cancellation: true,
            hotel_class: 4,
        }),
        /Boolean filters cannot be combined/,
    );
    assert.throws(
        () => buildGoogleHotelsSearchParams({
            q: 'Paris',
            check_in_date: '2026-06-15',
            check_out_date: '2026-06-18',
            vacation_rentals: true,
            hotel_class: 4,
        }),
        /hotel_class and brands are not available/,
    );
});

test('builds vacation rental params and describes requests', () => {
    const params = buildGoogleHotelsSearchParams({
        q: 'Aspen cabins',
        check_in_date: '2026-12-10',
        check_out_date: '2026-12-14',
        vacation_rentals: true,
        bedrooms: 2,
        bathrooms: 2,
        next_page_token: 'page-token',
    });

    assert.deepEqual(params, {
        q: 'Aspen cabins',
        check_in_date: '2026-12-10',
        check_out_date: '2026-12-14',
        vacation_rentals: true,
        bedrooms: 2,
        bathrooms: 2,
        next_page_token: 'page-token',
    });
    assert.equal(
        describeGoogleHotelsSearchRequest(params),
        '"Aspen cabins" 2026-12-10 to 2026-12-14 (vacation_rentals=true, next_page_token=page-token)',
    );
});

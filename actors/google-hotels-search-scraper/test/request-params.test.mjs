import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const requestParamsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/request-params.ts'
    : '../dist/request-params.js';
const { buildGoogleHotelsSearchParams, describeGoogleHotelsSearchRequest } = await import(requestParamsModule);

function futureDate(daysFromNow) {
    const date = new Date();
    date.setUTCDate(date.getUTCDate() + daysFromNow);
    return date.toISOString().slice(0, 10);
}

const checkInDate = futureDate(30);
const checkOutDate = futureDate(33);
const laterCheckInDate = futureDate(60);
const laterCheckOutDate = futureDate(64);

test('builds params for a complete hotel search request', () => {
    assert.deepEqual(
        buildGoogleHotelsSearchParams({
            q: ' Paris, France ',
            check_in_date: checkInDate,
            check_out_date: checkOutDate,
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
            check_in_date: checkInDate,
            check_out_date: checkOutDate,
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

test('accepts marketplace select values as numeric strings', () => {
    assert.deepEqual(
        buildGoogleHotelsSearchParams({
            q: 'Paris, France',
            check_in_date: checkInDate,
            check_out_date: checkOutDate,
            sort_by: '3',
            hotel_class: '4',
            rating: '8',
        }),
        {
            q: 'Paris, France',
            check_in_date: checkInDate,
            check_out_date: checkOutDate,
            sort_by: 3,
            hotel_class: 4,
            rating: 8,
        },
    );
});

test('input schema accepts numeric and string select values', async () => {
    const schema = JSON.parse(await readFile(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));
    for (const [field, values] of Object.entries({
        sort_by: [3, '3', 8, '8', 13, '13'],
        hotel_class: [2, '2', 3, '3', 4, '4', 5, '5'],
        rating: [7, '7', 8, '8', 9, '9'],
    })) {
        const property = schema.properties[field];
        assert.deepEqual(property.type, ['integer', 'string']);
        for (const value of values) {
            assert.ok(property.enum.includes(value), `${field} should allow ${String(value)}`);
        }
    }
});

test('requires query and valid date range', () => {
    assert.throws(
        () => buildGoogleHotelsSearchParams({
            q: '',
            check_in_date: checkInDate,
            check_out_date: checkOutDate,
        }),
        /q is required/,
    );
    assert.throws(
        () => buildGoogleHotelsSearchParams({
            q: 'Paris',
            check_in_date: checkOutDate,
            check_out_date: checkInDate,
        }),
        /check_out_date must be after check_in_date/,
    );
    assert.throws(
        () => buildGoogleHotelsSearchParams({
            q: 'Paris',
            check_in_date: '2026-02-30',
            check_out_date: checkOutDate,
        }),
        /check_in_date must be a valid calendar date/,
    );
    assert.throws(
        () => buildGoogleHotelsSearchParams({
            q: 'Paris',
            check_in_date: '2020-01-01',
            check_out_date: checkOutDate,
        }),
        /check_in_date must be today or a future date/,
    );
});

test('validates guest and price constraints', () => {
    assert.throws(
        () => buildGoogleHotelsSearchParams({
            q: 'Paris',
            check_in_date: checkInDate,
            check_out_date: checkOutDate,
            children: 2,
            children_ages: [7],
        }),
        /children_ages length must match children/,
    );
    assert.throws(
        () => buildGoogleHotelsSearchParams({
            q: 'Paris',
            check_in_date: checkInDate,
            check_out_date: checkOutDate,
            min_price: 500,
            max_price: 200,
        }),
        /max_price must be greater than min_price/,
    );
    assert.throws(
        () => buildGoogleHotelsSearchParams({
            q: 'Paris',
            check_in_date: checkInDate,
            check_out_date: checkOutDate,
            min_price: 0,
            max_price: 5001,
        }),
        /max_price must be between 1 and 5000/,
    );
    assert.deepEqual(
        buildGoogleHotelsSearchParams({
            q: 'Paris',
            check_in_date: checkInDate,
            check_out_date: checkOutDate,
            min_price: 200,
        }),
        {
            q: 'Paris',
            check_in_date: checkInDate,
            check_out_date: checkOutDate,
            min_price: 200,
        },
    );
});

test('rejects unsupported filter combinations', () => {
    assert.throws(
        () => buildGoogleHotelsSearchParams({
            q: 'Paris',
            check_in_date: checkInDate,
            check_out_date: checkOutDate,
            free_cancellation: true,
            hotel_class: 4,
        }),
        /Boolean filters cannot be combined/,
    );
    assert.throws(
        () => buildGoogleHotelsSearchParams({
            q: 'Paris',
            check_in_date: checkInDate,
            check_out_date: checkOutDate,
            vacation_rentals: true,
            hotel_class: 4,
        }),
        /hotel_class and brands are not available/,
    );
    assert.throws(
        () => buildGoogleHotelsSearchParams({
            q: 'Paris',
            check_in_date: checkInDate,
            check_out_date: checkOutDate,
            vacation_rentals: true,
            eco_certified: true,
        }),
        /free_cancellation, eco_certified, and special_offers are not available/,
    );
    assert.throws(
        () => buildGoogleHotelsSearchParams({
            q: 'Paris',
            check_in_date: checkInDate,
            check_out_date: checkOutDate,
            vacation_rentals: false,
            bedrooms: 2,
        }),
        /bedrooms and bathrooms are only available/,
    );
});

test('rejects malformed codes and comma-separated integer lists', () => {
    assert.throws(
        () => buildGoogleHotelsSearchParams({
            q: 'Paris',
            check_in_date: checkInDate,
            check_out_date: checkOutDate,
            currency: 'EURO',
        }),
        /currency must be 3 characters or fewer/,
    );
    assert.throws(
        () => buildGoogleHotelsSearchParams({
            q: 'Paris',
            check_in_date: checkInDate,
            check_out_date: checkOutDate,
            amenities: '35,bad,9',
        }),
        /amenities\[1\] must be an integer/,
    );
    assert.throws(
        () => buildGoogleHotelsSearchParams({
            q: 'Paris',
            check_in_date: checkInDate,
            check_out_date: checkOutDate,
            property_types: '17,,18',
        }),
        /property_types\[1\] must be an integer/,
    );
});

test('builds vacation rental params and describes requests', () => {
    const params = buildGoogleHotelsSearchParams({
        q: 'Aspen cabins',
        check_in_date: laterCheckInDate,
        check_out_date: laterCheckOutDate,
        vacation_rentals: true,
        bedrooms: 2,
        bathrooms: 2,
        next_page_token: 'page-token',
    });

    assert.deepEqual(params, {
        q: 'Aspen cabins',
        check_in_date: laterCheckInDate,
        check_out_date: laterCheckOutDate,
        vacation_rentals: true,
        bedrooms: 2,
        bathrooms: 2,
        next_page_token: 'page-token',
    });
    assert.equal(
        describeGoogleHotelsSearchRequest(params),
        `"Aspen cabins" ${laterCheckInDate} to ${laterCheckOutDate} (bathrooms=2, bedrooms=2, next_page_token=page-token, vacation_rentals=true)`,
    );
});

test('describes all active params including boolean filters', () => {
    assert.equal(
        describeGoogleHotelsSearchRequest({
            q: 'Paris',
            check_in_date: checkInDate,
            check_out_date: checkOutDate,
            free_cancellation: true,
            currency: 'EUR',
        }),
        `"Paris" ${checkInDate} to ${checkOutDate} (currency=EUR, free_cancellation=true)`,
    );
});

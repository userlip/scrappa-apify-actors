import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const requestParamsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/request-params.ts'
    : '../dist/request-params.js';
const { buildBookingSearchRequests, describeBookingSearchRequest } = await import(requestParamsModule);

function futureDate(daysFromNow) {
    const date = new Date();
    date.setUTCDate(date.getUTCDate() + daysFromNow);
    return date.toISOString().slice(0, 10);
}

const checkin = futureDate(30);
const checkout = futureDate(33);
const laterCheckin = futureDate(60);
const laterCheckout = futureDate(64);

test('builds a single Booking.com search request', () => {
    assert.deepEqual(
        buildBookingSearchRequests({
            ss: ' Paris ',
            checkin,
            checkout,
            group_adults: '2',
            group_children: 1,
            no_rooms: '1',
            lang: 'EN-US',
            currency: 'eur',
        }),
        [{
            index: 0,
            params: {
                ss: 'Paris',
                checkin,
                checkout,
                group_adults: 2,
                group_children: 1,
                no_rooms: 1,
                lang: 'en-us',
                currency: 'EUR',
            },
        }],
    );
});

test('builds batch Booking.com search requests', () => {
    assert.deepEqual(
        buildBookingSearchRequests({
            searches: [
                { ss: 'Paris', checkin, checkout, group_adults: 2 },
                { ss: 'Berlin', checkin: laterCheckin, checkout: laterCheckout, currency: 'EUR' },
            ],
        }),
        [
            {
                index: 0,
                params: {
                    ss: 'Paris',
                    checkin,
                    checkout,
                    group_adults: 2,
                },
            },
            {
                index: 1,
                params: {
                    ss: 'Berlin',
                    checkin: laterCheckin,
                    checkout: laterCheckout,
                    currency: 'EUR',
                },
            },
        ],
    );
});

test('input schema exposes batch searches', async () => {
    const schema = JSON.parse(await readFile(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));
    assert.equal(schema.properties.searches.type, 'array');
    assert.equal(schema.properties.searches.maxItems, 25);
    assert.deepEqual(schema.properties.searches.items.required, ['ss']);
});

test('requires destination and valid paired dates', () => {
    assert.throws(
        () => buildBookingSearchRequests({
            ss: '',
            checkin,
            checkout,
        }),
        /ss is required/,
    );
    assert.throws(
        () => buildBookingSearchRequests({
            ss: 'Paris',
            checkin,
        }),
        /checkin and checkout must be provided together/,
    );
    assert.throws(
        () => buildBookingSearchRequests({
            ss: 'Paris',
            checkin: checkout,
            checkout: checkin,
        }),
        /checkout must be after checkin/,
    );
    assert.throws(
        () => buildBookingSearchRequests({
            ss: 'Paris',
            checkin,
            checkout: checkin,
        }),
        /checkout must be after checkin/,
    );
    assert.throws(
        () => buildBookingSearchRequests({
            ss: 'Paris',
            checkin: '2026-02-30',
            checkout,
        }),
        /checkin must be a valid calendar date/,
    );
    assert.throws(
        () => buildBookingSearchRequests({
            ss: 'Paris',
            checkin: '2020-01-01',
            checkout,
        }),
        /checkin must be today or a future date/,
    );
});

test('validates numeric and locale fields', () => {
    assert.throws(
        () => buildBookingSearchRequests({ ss: 'Paris', group_adults: 31 }),
        /group_adults must be between 1 and 30/,
    );
    assert.throws(
        () => buildBookingSearchRequests({ ss: 'Paris', group_children: -1 }),
        /group_children must be between 0 and 20/,
    );
    assert.throws(
        () => buildBookingSearchRequests({ ss: 'Paris', no_rooms: 0 }),
        /no_rooms must be between 1 and 30/,
    );
    assert.throws(
        () => buildBookingSearchRequests({ ss: 'Paris', lang: 'english' }),
        /lang must be a valid language code/,
    );
    assert.throws(
        () => buildBookingSearchRequests({ ss: 'Paris', currency: 'EURO' }),
        /currency must be a 3-letter currency code/,
    );
});

test('validates batch constraints', () => {
    assert.throws(
        () => buildBookingSearchRequests({ searches: [] }),
        /searches must include at least one search/,
    );
    assert.throws(
        () => buildBookingSearchRequests({ searches: ['Paris'] }),
        /searches\[0\] must be an object/,
    );
    assert.throws(
        () => buildBookingSearchRequests({ ss: 'Paris', searches: 'Paris' }),
        /searches must be an array of search objects/,
    );
    assert.throws(
        () => buildBookingSearchRequests({ searches: Array.from({ length: 26 }, () => ({ ss: 'Paris' })) }),
        /searches cannot include more than 25 searches/,
    );
});

test('describes Booking.com search request', () => {
    assert.equal(
        describeBookingSearchRequest({
            ss: 'Paris',
            checkin,
            checkout,
            group_adults: 2,
            currency: 'EUR',
        }),
        `"Paris" ${checkin} to ${checkout} (currency=EUR, group_adults=2)`,
    );
});

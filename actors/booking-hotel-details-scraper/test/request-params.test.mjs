import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const requestParamsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/request-params.ts'
    : '../dist/request-params.js';
const { buildBookingHotelRequests, describeBookingHotelRequest } = await import(requestParamsModule);

test('builds a single Booking.com hotel request from URL', () => {
    assert.deepEqual(
        buildBookingHotelRequests({
            url: ' https://www.booking.com/hotel/fr/ritz-paris.html ',
            country: 'de',
            slug: 'ignored',
        }),
        [{
            index: 0,
            inputType: 'url',
            params: {
                url: 'https://www.booking.com/hotel/fr/ritz-paris.html',
            },
        }],
    );
});

test('builds a single Booking.com hotel request from country and slug', () => {
    assert.deepEqual(
        buildBookingHotelRequests({
            country: 'FR',
            slug: 'ritz-paris.html',
        }),
        [{
            index: 0,
            inputType: 'country_slug',
            params: {
                country: 'fr',
                slug: 'ritz-paris.html',
            },
        }],
    );
});

test('builds batch Booking.com hotel requests from urls and hotels', () => {
    assert.deepEqual(
        buildBookingHotelRequests({
            urls: [
                'https://www.booking.com/hotel/fr/ritz-paris.html',
                'https://de.booking.com/hotel/de/sample.html?lang=en-us',
            ],
            hotels: [
                { country: 'gb', slug: 'london_sample' },
                { url: 'https://www.booking.com/hotel/us/example.html', country: 'fr', slug: 'ignored' },
            ],
        }),
        [
            {
                index: 0,
                inputType: 'url',
                params: { url: 'https://www.booking.com/hotel/fr/ritz-paris.html' },
            },
            {
                index: 1,
                inputType: 'url',
                params: { url: 'https://de.booking.com/hotel/de/sample.html?lang=en-us' },
            },
            {
                index: 2,
                inputType: 'country_slug',
                params: { country: 'gb', slug: 'london_sample' },
            },
            {
                index: 3,
                inputType: 'url',
                params: { url: 'https://www.booking.com/hotel/us/example.html' },
            },
        ],
    );
});

test('input schema exposes url, country/slug, urls, and hotels', async () => {
    const schema = JSON.parse(await readFile(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));
    assert.equal(schema.properties.url.type, 'string');
    assert.equal(schema.properties.urls.type, 'array');
    assert.equal(schema.properties.urls.maxItems, 50);
    assert.equal(schema.properties.hotels.type, 'array');
    assert.equal(schema.properties.hotels.maxItems, 50);
    assert.equal(schema.properties.country.pattern, '^[A-Za-z]{2}$');
    assert.equal(schema.properties.slug.pattern, '^[A-Za-z0-9._-]+(?:\\.html)?$');
});

test('validates required input shape and field contracts', () => {
    assert.throws(
        () => buildBookingHotelRequests({}),
        /Provide at least one hotel URL or country\/slug pair/,
    );
    assert.throws(
        () => buildBookingHotelRequests({ url: 'https://example.com/hotel/foo' }),
        /url must be a Booking.com hotel URL/,
    );
    assert.throws(
        () => buildBookingHotelRequests({ country: 'fr' }),
        /country and slug must be provided together/,
    );
    assert.throws(
        () => buildBookingHotelRequests({ country: 'fra', slug: 'ritz-paris' }),
        /country must be 2 characters or fewer/,
    );
    assert.throws(
        () => buildBookingHotelRequests({ country: 'fr', slug: 'ritz paris!' }),
        /slug must contain only letters/,
    );
    assert.throws(
        () => buildBookingHotelRequests({ urls: 'https://www.booking.com/hotel/fr/ritz-paris.html' }),
        /urls must be an array/,
    );
    assert.throws(
        () => buildBookingHotelRequests({ hotels: ['https://www.booking.com/hotel/fr/ritz-paris.html'] }),
        /hotels\[0\] must be an object/,
    );
    assert.throws(
        () => buildBookingHotelRequests({
            urls: Array.from({ length: 51 }, () => 'https://www.booking.com/hotel/fr/ritz-paris.html'),
        }),
        /Hotel batches cannot include more than 50 hotels/,
    );
});

test('describes Booking.com hotel request', () => {
    assert.equal(
        describeBookingHotelRequest({
            index: 0,
            inputType: 'url',
            params: { url: 'https://www.booking.com/hotel/fr/ritz-paris.html' },
        }),
        'https://www.booking.com/hotel/fr/ritz-paris.html',
    );
    assert.equal(
        describeBookingHotelRequest({
            index: 0,
            inputType: 'country_slug',
            params: { country: 'fr', slug: 'ritz-paris' },
        }),
        'fr/ritz-paris',
    );
});

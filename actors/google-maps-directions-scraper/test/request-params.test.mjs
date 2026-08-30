import assert from 'node:assert/strict';
import test from 'node:test';

const modulePath = process.env.TEST_SOURCE === 'src'
    ? '../src/request-params.ts'
    : '../dist/request-params.js';
const { MAX_ROUTES_PER_RUN, buildDirectionsRequests, describeDirectionsRequest } = await import(modulePath);

test('normalizes singular compatibility input and omits empty optional region', () => {
    const requests = buildDirectionsRequests({
        origin: ' Berlin Hauptbahnhof ',
        destination: ' Brandenburg Gate ',
        mode: ' cycling ',
        hl: ' DE ',
        gl: ' ',
    });

    assert.deepEqual(requests, [{
        origin: 'Berlin Hauptbahnhof',
        destination: 'Brandenburg Gate',
        mode: 'bicycling',
        hl: 'de',
        gl: undefined,
        params: {
            origin: 'Berlin Hauptbahnhof',
            destination: 'Brandenburg Gate',
            mode: 'bicycling',
            hl: 'de',
        },
        index: 0,
    }]);
    assert.equal(describeDirectionsRequest(requests[0]), 'Berlin Hauptbahnhof -> Brandenburg Gate (bicycling)');
});

test('normalizes, deduplicates, and indexes a route batch in first-seen order', () => {
    const requests = buildDirectionsRequests({
        routes: [
            { origin: 'A', destination: 'B' },
            { origin: ' a ', destination: ' b ', mode: 'DRIVING', hl: 'EN' },
            { origin: 'C', destination: 'D', gl: 'DE' },
        ],
    });

    assert.deepEqual(requests.map(({ origin, destination, mode, hl, gl, index }) => ({ origin, destination, mode, hl, gl, index })), [
        { origin: 'A', destination: 'B', mode: 'driving', hl: 'en', gl: undefined, index: 0 },
        { origin: 'C', destination: 'D', mode: 'driving', hl: 'en', gl: 'de', index: 1 },
    ]);
});

test('prefers an explicit route batch over schema-injected compatibility defaults', () => {
    const requests = buildDirectionsRequests({
        routes: [
            { origin: 'Berlin Hauptbahnhof', destination: 'Brandenburg Gate', mode: 'walking' },
            { origin: 'Berlin Hauptbahnhof', destination: 'Brandenburg Gate', mode: 'driving' },
        ],
        origin: 'Times Square, New York, NY',
        destination: 'Central Park, New York, NY',
        mode: 'driving',
        hl: 'en',
    });

    assert.equal(requests.length, 2);
    assert.deepEqual(requests.map(({ origin, destination, mode }) => ({ origin, destination, mode })), [
        { origin: 'Berlin Hauptbahnhof', destination: 'Brandenburg Gate', mode: 'walking' },
        { origin: 'Berlin Hauptbahnhof', destination: 'Brandenburg Gate', mode: 'driving' },
    ]);
});

test('rejects malformed, incomplete, and over-limit input before network work', () => {
    assert.throws(() => buildDirectionsRequests({}), /Provide at least one route/);
    assert.throws(() => buildDirectionsRequests({ origin: 'A' }), /provided together/);
    assert.throws(() => buildDirectionsRequests({ routes: [{ origin: '', destination: 'B' }] }), /must not be empty/);
    assert.throws(() => buildDirectionsRequests({ routes: [{ origin: 'A', destination: 'B', mode: 'flight' }] }), /mode must be one of/);
    assert.throws(() => buildDirectionsRequests({ routes: [{ origin: 'A', destination: 'B', hl: 'english' }] }), /5 characters/);
    assert.throws(() => buildDirectionsRequests({ routes: Array.from({ length: MAX_ROUTES_PER_RUN + 1 }, (_, i) => ({ origin: `A${i}`, destination: 'B' })) }), /at most 10/);
});

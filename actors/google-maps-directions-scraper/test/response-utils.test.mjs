import assert from 'node:assert/strict';
import test from 'node:test';

const modulePath = process.env.TEST_SOURCE === 'src'
    ? '../src/response-utils.ts'
    : '../dist/response-utils.js';
const { buildDirectionsDatasetRows, extractRouteAlternatives } = await import(modulePath);

const request = {
    origin: 'A',
    destination: 'B',
    mode: 'walking',
    hl: 'en',
    gl: 'de',
    params: { origin: 'A', destination: 'B', mode: 'walking', hl: 'en', gl: 'de' },
    index: 2,
};

test('extracts and enriches multiple route alternatives with stable indexes', () => {
    const rows = buildDirectionsDatasetRows({
        status: 'OK',
        search_parameters: { travel_mode: 'Walking' },
        directions: [
            { travel_mode: 'Walking', via: 'Main road', distance: 100, duration: 20, trips: [{ details: [{ gps_coordinates: { latitude: 1, longitude: 2 } }] }] },
            { travel_mode: 'Walking', via: 'Side road', distance: 120, duration: 24, trips: [{ details: [{ title: 'Turn', gps_coordinates: { latitude: 3, longitude: 4 } }] }] },
        ],
    }, request);

    assert.equal(rows.length, 2);
    assert.equal(rows[0].alternative_index, 0);
    assert.equal(rows[1].alternative_index, 1);
    assert.equal(rows[0].request_index, 2);
    assert.equal(rows[0].request_origin, 'A');
    assert.equal(rows[0].via, 'Main road');
    assert.deepEqual(rows[0].step_coordinates, [{ latitude: 1, longitude: 2 }]);
    assert.equal(rows[1].request_gl, 'de');
});

test('preserves sparse alternatives without inventing optional fields', () => {
    const rows = buildDirectionsDatasetRows({ status: 'OK', directions: [{ distance: 10 }] }, { ...request, gl: undefined });
    assert.deepEqual(rows[0], {
        distance: 10,
        alternative_index: 0,
        request_index: 2,
        request_origin: 'A',
        request_destination: 'B',
        request_mode: 'walking',
        request_hl: 'en',
    });
});

test('rejects explicit failures, empty alternatives, and malformed alternatives', () => {
    assert.throws(() => extractRouteAlternatives({ success: false, message: 'No route' }), /No route/);
    assert.throws(() => extractRouteAlternatives({ status: 'ZERO_RESULTS', message: 'No route' }), /No route.*ZERO_RESULTS/);
    assert.throws(() => extractRouteAlternatives({ status: 'OK', directions: [] }), /no route alternatives/);
    assert.throws(() => extractRouteAlternatives({ status: 'OK', directions: [{ distance: 1 }, null] }), /malformed/);
    assert.throws(() => extractRouteAlternatives({ status: 'OK' }), /did not include route alternatives/);
});

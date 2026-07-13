import test from 'node:test';
import assert from 'node:assert/strict';

const sourceDirectory = process.env.TEST_SOURCE === 'src' ? 'src' : 'dist';
const { buildVintedUserProfileRequests } = await import(`../${sourceDirectory}/request-params.js`);

test('normalizes singular, array, CSV, whitespace, and duplicate IDs in deterministic order', () => {
    const requests = buildVintedUserProfileRequests({
        user_id: ' 255914028 ',
        user_ids: ['255914028', ' 123 ', '456, 123'],
        country: ' de ',
    });

    assert.deepEqual(requests.map((request) => request.userId), ['255914028', '123', '456']);
    assert.deepEqual(requests.map((request) => request.params), [
        { user_id: '255914028', country: 'DE' },
        { user_id: '123', country: 'DE' },
        { user_id: '456', country: 'DE' },
    ]);
});

test('accepts a safe numeric singular ID and defaults country to FR', () => {
    const [request] = buildVintedUserProfileRequests({ user_id: 255914028 });
    assert.equal(request.userId, '255914028');
    assert.equal(request.params.country, 'FR');
});

test('rejects empty, malformed, unsupported, and over-limit input', () => {
    assert.throws(() => buildVintedUserProfileRequests({}), /at least one/);
    assert.throws(() => buildVintedUserProfileRequests({ user_ids: '123,not-a-user' }), /numeric/);
    assert.throws(() => buildVintedUserProfileRequests({ user_id: '123', country: 'GB' }), /country must be one of/);
    assert.throws(() => buildVintedUserProfileRequests({ user_ids: Array.from({ length: 101 }, (_, index) => String(index + 1)) }), /at most 100/);
});

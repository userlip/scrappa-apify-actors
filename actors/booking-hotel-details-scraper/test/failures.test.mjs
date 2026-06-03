import assert from 'node:assert/strict';
import test from 'node:test';

const failuresModule = process.env.TEST_SOURCE === 'src'
    ? '../src/failures.ts'
    : '../dist/failures.js';
const { isActorLevelScrappaFailure } = await import(failuresModule);

test('treats Scrappa credential errors as actor-level failures', () => {
    assert.equal(isActorLevelScrappaFailure(new Error('Scrappa API error (401): Unauthorized')), true);
    assert.equal(isActorLevelScrappaFailure(new Error('Scrappa API error (403): Forbidden')), true);
});

test('keeps transient Scrappa errors as per-request batch failures', () => {
    for (const statusCode of [408, 429, 500, 502, 503, 504]) {
        assert.equal(
            isActorLevelScrappaFailure(new Error(`Scrappa API error (${statusCode}): temporary failure`)),
            false,
        );
    }
});

test('does not treat non-error values as actor-level failures', () => {
    assert.equal(isActorLevelScrappaFailure('Scrappa API error (401): Unauthorized'), false);
});

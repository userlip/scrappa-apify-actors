import assert from 'node:assert/strict';
import test from 'node:test';

const availabilityErrorsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/availability-errors.ts'
    : '../dist/availability-errors.js';
const {
    getTransientRedfinValuationStatus,
    isTransientRedfinValuationError,
} = await import(availabilityErrorsModule);

const sharedModule = process.env.TEST_SOURCE === 'src'
    ? '../src/shared/index.ts'
    : '../dist/shared/index.js';
const { ScrappaHttpError, ScrappaTimeoutError } = await import(sharedModule);

test('treats Scrappa server errors and timeouts as transient availability failures', () => {
    assert.equal(isTransientRedfinValuationError(new ScrappaHttpError(500, 'upstream failed')), true);
    assert.equal(isTransientRedfinValuationError(new ScrappaHttpError(503, 'unavailable')), true);
    assert.equal(isTransientRedfinValuationError(new ScrappaTimeoutError(60000)), true);
});

test('keeps client and validation errors fatal', () => {
    assert.equal(isTransientRedfinValuationError(new ScrappaHttpError(404, 'not found')), false);
    assert.equal(isTransientRedfinValuationError(new ScrappaHttpError(422, 'invalid input')), false);
    assert.equal(isTransientRedfinValuationError(new Error('invalid actor input')), false);
});

test('builds an uncharged completion status for transient upstream failures', () => {
    assert.equal(
        getTransientRedfinValuationStatus(new ScrappaHttpError(500, 'upstream failed')),
        'Scrappa upstream returned 500 after retries; no Redfin valuation result was written or charged. Try the run again later.',
    );
    assert.equal(
        getTransientRedfinValuationStatus(new ScrappaTimeoutError(60000)),
        'Scrappa API request timed out after 60000ms; no Redfin valuation result was written or charged. Try the run again later.',
    );
});

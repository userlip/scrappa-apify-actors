import assert from 'node:assert/strict';
import test from 'node:test';

import { ScrappaHttpError } from '../dist/shared/index.js';

const httpErrorsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/http-errors.ts'
    : '../dist/http-errors.js';
const { isPerPropertyScrappaHttpError } = await import(httpErrorsModule);

test('classifies property-specific Scrappa HTTP errors', () => {
    assert.equal(isPerPropertyScrappaHttpError(new ScrappaHttpError(400, 'Bad request')), true);
    assert.equal(isPerPropertyScrappaHttpError(new ScrappaHttpError(404, 'Property not found')), true);
    assert.equal(isPerPropertyScrappaHttpError(new ScrappaHttpError(422, 'Invalid property ID')), true);
});

test('does not classify auth, quota, or server HTTP errors as per-property errors', () => {
    assert.equal(isPerPropertyScrappaHttpError(new ScrappaHttpError(401, 'Unauthorized')), false);
    assert.equal(isPerPropertyScrappaHttpError(new ScrappaHttpError(403, 'Forbidden')), false);
    assert.equal(isPerPropertyScrappaHttpError(new ScrappaHttpError(429, 'Rate limited')), false);
    assert.equal(isPerPropertyScrappaHttpError(new ScrappaHttpError(500, 'Server error')), false);
    assert.equal(isPerPropertyScrappaHttpError(new Error('Network failed')), false);
});

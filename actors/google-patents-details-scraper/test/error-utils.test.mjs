import assert from 'node:assert/strict';
import test from 'node:test';

import { formatGooglePatentsDetailsError } from '../dist/error-utils.js';
import { ScrappaTimeoutError } from '../dist/shared/scrappa-client.js';

test('formats Scrappa timeout errors with details guidance', () => {
    assert.equal(
        formatGooglePatentsDetailsError(new ScrappaTimeoutError(60000), 60000),
        'Scrappa API request timed out after 60000ms. The Google Patents details request exceeded the 60s Scrappa API timeout. Run the request again or try a smaller batch.',
    );
});

test('passes through non-timeout errors', () => {
    const error = new Error('Scrappa API error (500): Failed');
    assert.equal(formatGooglePatentsDetailsError(error, 60000), error);
});

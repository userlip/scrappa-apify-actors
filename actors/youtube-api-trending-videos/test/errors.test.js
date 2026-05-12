import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { errorMessage } from '../src/errors.js';

describe('errorMessage', () => {
    it('returns a friendly timeout message for TimeoutError', () => {
        const error = new Error('The operation timed out');
        error.name = 'TimeoutError';

        assert.equal(errorMessage(error), 'Scrappa API request timed out after 60s');
    });

    it('falls back to aborted message detection for older fetch errors', () => {
        assert.equal(errorMessage(new Error('The operation was aborted')), 'Scrappa API request timed out after 60s');
    });

    it('returns the original message for non-timeout errors', () => {
        assert.equal(errorMessage(new Error('Scrappa API request failed')), 'Scrappa API request failed');
    });

    it('stringifies non-Error thrown values', () => {
        assert.equal(errorMessage('plain failure'), 'plain failure');
    });
});

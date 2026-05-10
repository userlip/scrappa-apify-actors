import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { errorMessage } from '../src/errors.js';

describe('errorMessage', () => {
    it('formats AbortError as a Scrappa API timeout', () => {
        const error = new Error('The operation was aborted');
        error.name = 'AbortError';

        assert.equal(errorMessage(error), 'Scrappa API request timed out after 60s');
    });

    it('keeps compatibility with existing aborted message checks', () => {
        assert.equal(errorMessage(new Error('request aborted')), 'Scrappa API request timed out after 60s');
    });

    it('passes through other errors', () => {
        assert.equal(errorMessage(new Error('bad response')), 'bad response');
    });
});


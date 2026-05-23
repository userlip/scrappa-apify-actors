import assert from 'node:assert/strict';
import test from 'node:test';

import { buildTransientFailureStatusMessage } from '../dist/status-messages.js';

test('describes transient failure before any results are written', () => {
    assert.equal(
        buildTransientFailureStatusMessage('Scrappa upstream returned 503 after retries', 0, 1),
        'Scrappa upstream returned 503 after retries; no Google Finance search results were written or charged. Try the run again later.',
    );
});

test('describes incomplete transient batch failure before any results are written', () => {
    assert.equal(
        buildTransientFailureStatusMessage('Scrappa upstream returned 503 after retries', 0, 2),
        'Scrappa upstream returned 503 after retries; no Google Finance search results were written or charged. Remaining batch queries were not completed. Try the run again later.',
    );
});

test('describes transient failure after partial batch results are written', () => {
    assert.equal(
        buildTransientFailureStatusMessage('Scrappa API request timed out after 30000ms', 7, 2),
        'Scrappa API request timed out after 30000ms; 7 Google Finance search results were already written and may have been charged. Remaining queries were not completed. Try the run again later for the unfinished queries.',
    );
});

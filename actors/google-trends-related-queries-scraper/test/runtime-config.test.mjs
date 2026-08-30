import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

import {
    ACTOR_COMPLETION_RESERVE_MS,
    ACTOR_TIMEOUT_MS,
    AUTOCOMPLETE_MAX_ATTEMPTS,
    AUTOCOMPLETE_REQUEST_TIMEOUT_MS,
    RELATED_MAX_ATTEMPTS,
    RELATED_MAX_RETRY_DELAY_MS,
    RELATED_REQUEST_TIMEOUT_MS,
    getMaximumRequestDurationMs,
} from '../dist/runtime-config.js';

test('keeps primary, optional autocomplete, and persistence work within the Actor timeout', async () => {
    const relatedMaximumMs = getMaximumRequestDurationMs(
        RELATED_REQUEST_TIMEOUT_MS,
        RELATED_MAX_ATTEMPTS,
        RELATED_MAX_RETRY_DELAY_MS,
    );
    const autocompleteMaximumMs = getMaximumRequestDurationMs(
        AUTOCOMPLETE_REQUEST_TIMEOUT_MS,
        AUTOCOMPLETE_MAX_ATTEMPTS,
        0,
    );
    const configuredMaximumMs = relatedMaximumMs + autocompleteMaximumMs + ACTOR_COMPLETION_RESERVE_MS;

    assert.equal(relatedMaximumMs, 180000);
    assert.equal(autocompleteMaximumMs, 15000);
    assert.equal(configuredMaximumMs, 225000);
    assert.ok(configuredMaximumMs < ACTOR_TIMEOUT_MS);

    const actorConfig = JSON.parse(await readFile(new URL('../.actor/actor.json', import.meta.url), 'utf8'));
    assert.equal(actorConfig.defaultRunOptions.timeoutSecs * 1000, ACTOR_TIMEOUT_MS);
});

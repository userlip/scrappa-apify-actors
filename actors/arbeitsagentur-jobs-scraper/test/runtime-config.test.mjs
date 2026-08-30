import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

import {
    ACTOR_COMPLETION_RESERVE_MS,
    ACTOR_TIMEOUT_MS,
    JOBS_MAX_ATTEMPTS,
    JOBS_MAX_RETRY_DELAY_MS,
    JOBS_REQUEST_TIMEOUT_MS,
    getMaximumRequestDurationMs,
} from '../dist/runtime-config.js';

test('keeps retries and persistence work within the Actor timeout', async () => {
    const requestMaximumMs = getMaximumRequestDurationMs(
        JOBS_REQUEST_TIMEOUT_MS,
        JOBS_MAX_ATTEMPTS,
        JOBS_MAX_RETRY_DELAY_MS,
    );
    const configuredMaximumMs = requestMaximumMs + ACTOR_COMPLETION_RESERVE_MS;

    assert.equal(requestMaximumMs, 180000);
    assert.equal(configuredMaximumMs, 210000);
    assert.ok(configuredMaximumMs < ACTOR_TIMEOUT_MS);

    const actorConfig = JSON.parse(await readFile(new URL('../.actor/actor.json', import.meta.url), 'utf8'));
    assert.equal(actorConfig.defaultRunOptions.timeoutSecs * 1000, ACTOR_TIMEOUT_MS);
});

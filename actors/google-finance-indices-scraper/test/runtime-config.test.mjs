import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import {
    ACTOR_TIMEOUT_SECONDS,
    AVAILABLE_REQUEST_TIME_MS,
    REQUEST_ATTEMPTS,
    REQUEST_TIMEOUT_MS,
    RETRY_TIME_BUDGET_MS,
    retryBudgetFitsActorTimeout,
    RUN_FINALIZATION_RESERVE_MS,
} from '../dist/runtime-config.js';

test('retry policy fits the published Actor timeout with finalization reserve', async () => {
    const actorJson = JSON.parse(
        await readFile(new URL('../.actor/actor.json', import.meta.url), 'utf8'),
    );
    const publishedTimeoutSeconds = actorJson.defaultRunOptions.timeoutSecs;

    assert.equal(publishedTimeoutSeconds, ACTOR_TIMEOUT_SECONDS);
    assert.equal(REQUEST_TIMEOUT_MS, 30_000);
    assert.equal(REQUEST_ATTEMPTS, 3);
    assert.equal(
        AVAILABLE_REQUEST_TIME_MS,
        publishedTimeoutSeconds * 1_000 - RUN_FINALIZATION_RESERVE_MS,
    );
    assert.equal(RETRY_TIME_BUDGET_MS, 90_750);
    assert.equal(RETRY_TIME_BUDGET_MS <= AVAILABLE_REQUEST_TIME_MS, true);
    assert.equal(retryBudgetFitsActorTimeout(), true);
});

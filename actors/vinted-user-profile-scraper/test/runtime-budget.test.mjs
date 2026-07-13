import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const sourceDirectory = process.env.TEST_SOURCE === 'src' ? 'src' : 'dist';
const { MAX_USER_IDS_PER_RUN } = await import(`../${sourceDirectory}/request-params.js`);
const {
    ACTOR_TIMEOUT_MS,
    getWorstCaseRunDurationMs,
    getWorstCaseScrappaRequestDurationMs,
    PROFILE_REQUEST_CONCURRENCY,
    RUN_SAFETY_MARGIN_MS,
} = await import(`../${sourceDirectory}/runtime-budget.js`);

test('maximum supported batch fits within the configured Actor timeout', () => {
    const worstCaseRuntimeMs = getWorstCaseRunDurationMs(MAX_USER_IDS_PER_RUN);
    const waves = Math.ceil(MAX_USER_IDS_PER_RUN / PROFILE_REQUEST_CONCURRENCY);

    assert.equal(
        worstCaseRuntimeMs,
        waves * getWorstCaseScrappaRequestDurationMs() + RUN_SAFETY_MARGIN_MS,
    );
    assert.ok(worstCaseRuntimeMs < ACTOR_TIMEOUT_MS, `${worstCaseRuntimeMs}ms exceeds ${ACTOR_TIMEOUT_MS}ms`);
});

test('runtime budget matches the Actor timeout metadata', async () => {
    const actor = JSON.parse(await readFile(new URL('../.actor/actor.json', import.meta.url), 'utf8'));
    assert.equal(actor.defaultRunOptions.timeoutSecs * 1000, ACTOR_TIMEOUT_MS);
});

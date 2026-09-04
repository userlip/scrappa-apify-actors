import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { runPriceInsightsBatch } from '../dist/batch-runner.js';
import { normalizeLocations } from '../dist/request-params.js';
import { ScrappaClient } from '../dist/shared/index.js';

assert.ok(process.env.SCRAPPA_API_KEY, 'Set SCRAPPA_API_KEY to run the live integration check');
const schema = JSON.parse(await readFile(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));
const input = Object.fromEntries(Object.entries(schema.properties)
    .filter(([, property]) => Object.hasOwn(property, 'prefill'))
    .map(([name, property]) => [name, property.prefill]));
const requests = normalizeLocations(input);
const items = [];
const started = performance.now();
const result = await runPriceInsightsBatch(requests, new ScrappaClient({
    apiKey: process.env.SCRAPPA_API_KEY,
    timeoutMs: 10000,
}), {
    isPayPerEvent: () => false,
    async pushData(item) {
        items.push(item);
        return { chargedCount: 0, eventChargeLimitReached: false };
    },
});

assert.deepEqual(result.failures, [], 'Every prefilled location must resolve against the live API');
assert.equal(result.succeeded, requests.length);
assert.ok(items.length > 0, 'QA requires a non-empty dataset');
assert.ok(performance.now() - started < 300000, 'QA must finish within five minutes');
console.log(JSON.stringify({ input, seconds: (performance.now() - started) / 1000, items }, null, 2));

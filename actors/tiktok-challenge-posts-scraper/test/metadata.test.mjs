import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const actor = JSON.parse(await readFile(new URL('../.actor/actor.json', import.meta.url)));
const schema = JSON.parse(await readFile(new URL('../.actor/input_schema.json', import.meta.url)));
const readme = await readFile(new URL('../README.md', import.meta.url), 'utf8');
const dockerfile = await readFile(new URL('../.actor/Dockerfile', import.meta.url), 'utf8');

test('uses minimal wrapper resources and batch schema caps', () => {
    assert.equal(actor.resources.memoryMbytes, 128);
    assert.equal(actor.defaultRunOptions.timeoutSecs, 300);
    assert.equal(actor.defaultRunOptions.memoryMbytes, 128);
    assert.equal(actor.defaultRunOptions.maxItems, 2000);
    assert.equal(schema.properties.challenge_ids.maxItems, 20);
    assert.equal(schema.properties.results_per_challenge.maximum, 500);
});

test('documents paid event pricing, workflow, media expiry, and direct API path', () => {
    assert.match(readme, /challenge-post-result/);
    assert.match(readme, /\$0\.00025 per video/);
    assert.match(readme, /Search:[\s\S]*Details:[\s\S]*Posts:/);
    assert.match(readme, /may be signed and expire/);
    assert.match(readme, /Scrappa API/);
});

test('builds deterministically and removes build-only dependencies', () => {
    assert.match(dockerfile, /npm ci/);
    assert.match(dockerfile, /npm prune --omit=dev/);
    assert.doesNotMatch(dockerfile, /--omit=dev --include=dev/);
});

import assert from 'node:assert/strict';
import { randomBytes } from 'node:crypto';
import test from 'node:test';
import { binaryName, prebuiltSourceFiles, RUNTIME_DOCKERFILE } from './prebuilt-actor.mjs';

const dir = 'actors/similarweb-traffic-analytics-scraper';

test('reads the binary name from the Actor Dockerfile', () => {
  assert.equal(binaryName(dir), 'similarweb-traffic-analytics-scraper');
});

test('uploads metadata, the runtime Dockerfile and the gzipped binary only', () => {
  const files = prebuiltSourceFiles(dir, Buffer.from('fake-binary'));
  const names = files.map((file) => file.name);
  assert.ok(names.includes('.actor/actor.json'));
  assert.ok(names.includes('.actor/input_schema.json'));
  assert.ok(names.includes('.actor/actor.gz'));
  assert.ok(!names.some((name) => name.startsWith('src/') || name.startsWith('Cargo')));
  assert.equal(files.find((file) => file.name === '.actor/Dockerfile').content, RUNTIME_DOCKERFILE);
  const actorJson = JSON.parse(files.find((file) => file.name === '.actor/actor.json').content);
  assert.equal(actorJson.dockerfile, './Dockerfile');
  assert.equal(actorJson.maxMemoryMbytes, 128);
});

test('rejects uploads above the Apify 3 MB source limit', () => {
  assert.throws(() => prebuiltSourceFiles(dir, randomBytes(3 * 1024 * 1024)), /Apify limit/);
});

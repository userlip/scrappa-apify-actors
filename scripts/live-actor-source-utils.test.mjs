import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  findMissingLiveActors,
  isSafeSourcePath,
} from './live-actor-source-utils.mjs';

test('findMissingLiveActors accepts directory slugs even when manifest names are stale', () => {
  const liveActors = [
    { id: 'live-1', name: 'youtube-api-hashtags' },
    { id: 'live-2', name: 'youtube-api-playlists' },
    { id: 'live-3', name: 'missing-actor' },
  ];
  const localActors = new Map([
    ['youtube-api-hashtags', { directory: 'youtube-api-hashtags', manifestName: 'my-actor' }],
    ['my-actor', { directory: 'youtube-api-hashtags', manifestName: 'my-actor' }],
    ['youtube-api-playlists', { directory: 'youtube-api-playlists', manifestName: 'thescrappa/unlimited-youtube-api' }],
    ['thescrappa/unlimited-youtube-api', { directory: 'youtube-api-playlists', manifestName: 'thescrappa/unlimited-youtube-api' }],
  ]);

  assert.deepEqual(findMissingLiveActors(liveActors, localActors), [
    { id: 'live-3', name: 'missing-actor' },
  ]);
});

test('isSafeSourcePath rejects absolute and parent traversal paths', () => {
  assert.equal(isSafeSourcePath('src/main.js'), true);
  assert.equal(isSafeSourcePath('.actor/actor.json'), true);
  assert.equal(isSafeSourcePath('/tmp/main.js'), false);
  assert.equal(isSafeSourcePath('../main.js'), false);
  assert.equal(isSafeSourcePath('src/../../main.js'), false);
});

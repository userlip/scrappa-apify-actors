import assert from 'node:assert/strict';
import test from 'node:test';

import {
  auditActorHealth,
  createHealthAuditReport,
  createOwnedPublicActorScope,
  getActorOwnership,
  parseArgs,
} from './audit-apify-health.mjs';

const checkedAt = new Date('2026-05-24T05:01:12.000Z');

test('createOwnedPublicActorScope includes public TheScrappa actors by userId or username', () => {
  const scope = createOwnedPublicActorScope([
    {
      actor: { id: 'owned-by-id', name: 'owned-by-id' },
      detail: actorDetail({ id: 'owned-by-id', name: 'owned-by-id', userId: '8683TqwnXHrQ46FhH' }),
      error: null,
    },
    {
      actor: { id: 'owned-by-username', name: 'owned-by-username' },
      detail: actorDetail({ id: 'owned-by-username', name: 'owned-by-username', username: 'thescrappa' }),
      error: null,
    },
  ]);

  assert.deepEqual(scope.ownedPublicActors.map((actor) => actor.id), ['owned-by-id', 'owned-by-username']);
  assert.deepEqual(scope.excludedPublicActors, []);
  assert.deepEqual(scope.errors, []);
});

test('createOwnedPublicActorScope excludes visible Apify-owned public actors', () => {
  const scope = createOwnedPublicActorScope([
    {
      actor: { id: 'RB9HEZitC8hIUXAha', name: 'instagram-api-scraper' },
      detail: actorDetail({
        id: 'RB9HEZitC8hIUXAha',
        name: 'instagram-api-scraper',
        userId: 'ZscMwFR5H7eCtWtyh',
        username: 'apify',
      }),
      error: null,
    },
  ]);

  assert.deepEqual(scope.ownedPublicActors, []);
  assert.equal(scope.excludedPublicActors.length, 1);
  assert.equal(scope.excludedPublicActors[0].actorId, 'RB9HEZitC8hIUXAha');
  assert.equal(scope.excludedPublicActors[0].username, 'apify');
  assert.match(scope.excludedPublicActors[0].reason, /not owned by TheScrappa/);
});

test('createOwnedPublicActorScope excludes public actors with unknown ownership as warnings', () => {
  const scope = createOwnedPublicActorScope([
    {
      actor: { id: 'unknown-owner', name: 'unknown-owner' },
      detail: actorDetail({ id: 'unknown-owner', name: 'unknown-owner', userId: null, username: null }),
      error: null,
    },
  ]);

  assert.deepEqual(scope.ownedPublicActors, []);
  assert.equal(scope.excludedPublicActors.length, 1);
  assert.match(scope.excludedPublicActors[0].reason, /no userId or username/);
});

test('createOwnedPublicActorScope skips private actors', () => {
  const scope = createOwnedPublicActorScope([
    {
      actor: { id: 'private-actor', name: 'private-actor' },
      detail: actorDetail({ id: 'private-actor', name: 'private-actor', isPublic: false, userId: '8683TqwnXHrQ46FhH' }),
      error: null,
    },
  ]);

  assert.deepEqual(scope.ownedPublicActors, []);
  assert.deepEqual(scope.excludedPublicActors, []);
  assert.deepEqual(scope.errors, []);
});

test('auditActorHealth reports no-run actors', () => {
  const report = auditActorHealth({
    actor: actorDetail({ id: 'no-run-id', name: 'no-run-actor' }),
    runs: [],
    builds: [{ id: 'build-id', status: 'SUCCEEDED' }],
  });

  assert.equal(report.status, 'NO_RUNS');
  assert.equal(report.latestRun, null);
  assert.equal(report.latestBuild.id, 'build-id');
});

test('auditActorHealth reports failed latest runs', () => {
  const report = auditActorHealth({
    actor: actorDetail({ id: 'failed-id', name: 'failed-actor' }),
    runs: [
      { id: 'latest-run', status: 'FAILED', startedAt: '2026-05-24T05:00:00.000Z' },
      { id: 'previous-run', status: 'SUCCEEDED' },
    ],
    builds: [],
  });

  assert.equal(report.status, 'FAILED_LATEST_RUN');
  assert.equal(report.latestRun.id, 'latest-run');
  assert.deepEqual(report.recentStatuses, ['FAILED', 'SUCCEEDED']);
});

test('auditActorHealth reports recent failures when latest run recovered', () => {
  const report = auditActorHealth({
    actor: actorDetail({ id: 'recovered-id', name: 'recovered-actor' }),
    runs: [
      { id: 'latest-run', status: 'SUCCEEDED' },
      { id: 'previous-run', status: 'TIMED-OUT' },
    ],
    builds: [],
  });

  assert.equal(report.status, 'RECENT_FAILED_BUT_LATEST_OK');
  assert.equal(report.recentFailedRuns.length, 1);
  assert.equal(report.recentFailedRuns[0].status, 'TIMED-OUT');
});

test('auditActorHealth includes maintenance notice type and message', () => {
  const report = auditActorHealth({
    actor: actorDetail({
      id: 'notice-id',
      name: 'notice-actor',
      notices: [{ type: 'UNDER_MAINTENANCE', message: 'Temporarily under maintenance.' }],
    }),
    runs: [{ id: 'latest-run', status: 'SUCCEEDED' }],
    builds: [{ id: 'build-id', status: 'SUCCEEDED' }],
  });

  assert.equal(report.status, 'OK');
  assert.equal(report.notice.type, 'UNDER_MAINTENANCE');
  assert.equal(report.notice.message, 'Temporarily under maintenance.');
});

test('auditActorHealth ignores Apify NONE notices', () => {
  const stringNoticeReport = auditActorHealth({
    actor: actorDetail({ id: 'none-string-id', name: 'none-string-actor', notice: 'NONE' }),
    runs: [{ id: 'latest-run', status: 'SUCCEEDED' }],
    builds: [],
  });
  const objectNoticeReport = auditActorHealth({
    actor: actorDetail({ id: 'none-object-id', name: 'none-object-actor', notice: { type: 'NONE' } }),
    runs: [{ id: 'latest-run', status: 'SUCCEEDED' }],
    builds: [],
  });

  assert.equal(stringNoticeReport.notice, null);
  assert.equal(objectNoticeReport.notice, null);
});

test('createHealthAuditReport summarizes owned actors and exclusions', () => {
  const report = createHealthAuditReport({
    checkedAt,
    actors: [
      {
        actor: actorDetail({ id: 'ok-id', name: 'ok-actor' }),
        runs: [{ id: 'ok-run', status: 'SUCCEEDED' }],
        builds: [{ id: 'ok-build', status: 'SUCCEEDED' }],
      },
      {
        actor: actorDetail({ id: 'no-run-id', name: 'no-run-actor' }),
        runs: [],
        builds: [],
      },
      {
        actor: actorDetail({ id: 'failed-id', name: 'failed-actor' }),
        runs: [{ id: 'failed-run', status: 'ABORTED' }],
        builds: [],
      },
    ],
    excludedPublicActors: [{
      actorId: 'RB9HEZitC8hIUXAha',
      slug: 'instagram-api-scraper',
      username: 'apify',
      userId: 'ZscMwFR5H7eCtWtyh',
      reason: 'Public actor is visible to this token but is not owned by TheScrappa.',
    }],
  });

  assert.equal(report.checkedAt, '2026-05-24T05:01:12.000Z');
  assert.equal(report.publicActors, 3);
  assert.equal(report.summary.ok, 1);
  assert.equal(report.summary.noRuns, 1);
  assert.equal(report.summary.failedLatestRuns, 1);
  assert.equal(report.summary.excludedPublicActors, 1);
  assert.deepEqual(report.failedLatestActors.map((actor) => actor.actorId), ['failed-id']);
  assert.deepEqual(report.noRunActors.map((actor) => actor.actorId), ['no-run-id']);
});

test('getActorOwnership handles nested owner fields and case-insensitive username', () => {
  assert.deepEqual(getActorOwnership({ user: { id: '8683TqwnXHrQ46FhH', username: 'other' } }), {
    userId: '8683TqwnXHrQ46FhH',
    username: 'other',
    hasOwnershipFields: true,
    isOwned: true,
  });

  assert.equal(getActorOwnership({ owner: { username: 'TheScrappa' } }).isOwned, true);
});

test('parseArgs supports JSON output', () => {
  assert.deepEqual(parseArgs(['--json']), {
    help: false,
    json: true,
  });

  assert.throws(() => parseArgs(['--include-active']), /Unknown argument/);
});

function actorDetail(overrides) {
  return {
    id: 'actor-id',
    name: 'actor-slug',
    title: 'Actor Title',
    isPublic: true,
    userId: '8683TqwnXHrQ46FhH',
    username: 'thescrappa',
    ...overrides,
  };
}

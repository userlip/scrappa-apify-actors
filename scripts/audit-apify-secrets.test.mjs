import assert from 'node:assert/strict';
import test from 'node:test';

import {
  auditActorSecrets,
  auditActorSecretSafely,
  auditActorVersionSecret,
  parseArgs,
  resolveDefaultVersionNumber,
} from './audit-apify-secrets.mjs';

test('resolveDefaultVersionNumber uses defaultRunOptions build tag to pick legacy 0.0', () => {
  const versionNumber = resolveDefaultVersionNumber({
    id: 'legacy-actor',
    defaultRunOptions: { build: 'latest' },
    versions: [
      { versionNumber: '0.0', buildTag: 'latest' },
      { versionNumber: '1.0', buildTag: 'beta' },
    ],
  });

  assert.equal(versionNumber, '0.0');
});

test('resolveDefaultVersionNumber uses defaultRunOptions build tag to pick newer 1.0', () => {
  const versionNumber = resolveDefaultVersionNumber({
    id: 'newer-actor',
    defaultRunOptions: { build: 'latest' },
    versions: [
      { versionNumber: '0.0', buildTag: 'legacy' },
      { versionNumber: '1.0', buildTag: 'latest' },
    ],
  });

  assert.equal(versionNumber, '1.0');
});

test('resolveDefaultVersionNumber falls back to the only available version', () => {
  const versionNumber = resolveDefaultVersionNumber({
    id: 'single-version-actor',
    defaultRunOptions: { build: 'missing-tag' },
    versions: [
      { versionNumber: '0.0', buildTag: 'latest' },
    ],
  });

  assert.equal(versionNumber, '0.0');
});

test('resolveDefaultVersionNumber maps concrete build number strings to versions', () => {
  const versionNumber = resolveDefaultVersionNumber({
    id: 'concrete-build-actor',
    defaultRunOptions: { build: '1.0.15' },
    versions: [
      { versionNumber: '0.0', buildTag: 'legacy' },
      { versionNumber: '1.0', buildTag: 'latest' },
    ],
  });

  assert.equal(versionNumber, '1.0');
});

test('resolveDefaultVersionNumber maps numeric build identifiers to versions', () => {
  const versionNumber = resolveDefaultVersionNumber({
    id: 'numeric-build-actor',
    defaultRunOptions: { build: 10000015 },
    versions: [
      { versionNumber: '0.0', buildTag: 'legacy' },
      { versionNumber: '1.0', buildTag: 'latest' },
    ],
  });

  assert.equal(versionNumber, '1.0');
});

test('resolveDefaultVersionNumber preserves zero numeric build identifiers', () => {
  const versionNumber = resolveDefaultVersionNumber({
    id: 'zero-build-actor',
    defaultRunOptions: { build: 0 },
    versions: [
      { versionNumber: '0.0', buildTag: 'latest' },
      { versionNumber: '1.0', buildTag: 'beta' },
    ],
  });

  assert.equal(versionNumber, '0.0');
});

test('resolveDefaultVersionNumber maps taggedBuild build numbers to versions', () => {
  const versionNumber = resolveDefaultVersionNumber({
    id: 'tagged-build-actor',
    defaultRunOptions: { build: '10000015' },
    taggedBuilds: {
      latest: {
        buildNumber: '1.0.15',
        buildNumberInt: 10000015,
      },
    },
    versions: [
      { versionNumber: '0.0', buildTag: 'legacy' },
      { versionNumber: '1.0', buildTag: 'latest' },
    ],
  });

  assert.equal(versionNumber, '1.0');
});

test('resolveDefaultVersionNumber prefers taggedBuilds over duplicate version build tags', () => {
  const versionNumber = resolveDefaultVersionNumber({
    id: 'duplicate-tag-actor',
    defaultRunOptions: { build: 'latest' },
    taggedBuilds: {
      latest: {
        buildNumber: '1.0.15',
        buildNumberInt: 10000015,
      },
    },
    versions: [
      { versionNumber: '0.0', buildTag: 'latest' },
      { versionNumber: '1.0', buildTag: 'latest' },
    ],
  });

  assert.equal(versionNumber, '1.0');
});

test('resolveDefaultVersionNumber rejects unresolvable default builds when multiple versions exist', () => {
  assert.throws(() => resolveDefaultVersionNumber({
    id: 'ambiguous-build-actor',
    defaultRunOptions: { build: 'missing-tag' },
    versions: [
      { versionNumber: '0.0', buildTag: 'legacy' },
      { versionNumber: '1.0', buildTag: 'latest' },
    ],
  }), /default build missing-tag does not match/);
});

test('resolveDefaultVersionNumber rejects versions without version numbers', () => {
  assert.throws(() => resolveDefaultVersionNumber({
    id: 'invalid-version-actor',
    versions: [{ buildTag: 'latest' }],
  }), /no versions with versionNumber/);
});

test('auditActorVersionSecret checks the resolved default version for SCRAPPA_API_KEY', () => {
  const report = auditActorVersionSecret({
    id: 'actor-id',
    name: 'actor-slug',
    isPublic: true,
    defaultRunOptions: { build: 'latest' },
    versions: [{ versionNumber: '0.0', buildTag: 'latest' }],
  }, {
    versionNumber: '0.0',
    envVars: [{ name: 'SCRAPPA_API_KEY', isSecret: true }],
  });

  assert.equal(report.status, 'SECRET_PRESENT');
  assert.equal(report.versionNumber, '0.0');
});

test('auditActorVersionSecret flags missing and non-secret SCRAPPA_API_KEY', () => {
  const actor = {
    id: 'actor-id',
    name: 'actor-slug',
    isPublic: true,
    defaultRunOptions: { build: 'latest' },
    versions: [{ versionNumber: '1.0', buildTag: 'latest' }],
  };

  assert.equal(auditActorVersionSecret(actor, { versionNumber: '1.0', envVars: [] }).status, 'MISSING_SECRET');
  assert.equal(auditActorVersionSecret(actor, {
    versionNumber: '1.0',
    envVars: [{ name: 'SCRAPPA_API_KEY', isSecret: false }],
  }).status, 'NOT_SECRET');
});

test('auditActorSecretSafely fetches the resolved version instead of hardcoding 1.0', async () => {
  const originalFetch = globalThis.fetch;
  const requestedPaths = [];

  globalThis.fetch = async (url) => {
    const path = new URL(url).pathname;
    requestedPaths.push(path);

    if (path === '/v2/acts/actor-id') {
      return {
        ok: true,
        json: async () => ({
          data: {
            id: 'actor-id',
            name: 'legacy-actor',
            isPublic: true,
            defaultRunOptions: { build: 'latest' },
            versions: [{ versionNumber: '0.0', buildTag: 'latest' }],
          },
        }),
      };
    }

    if (path === '/v2/acts/actor-id/versions/0.0') {
      return {
        ok: true,
        json: async () => ({
          data: {
            versionNumber: '0.0',
            envVars: [{ name: 'SCRAPPA_API_KEY', isSecret: true }],
          },
        }),
      };
    }

    return {
      ok: false,
      status: 404,
      text: async () => 'not found',
    };
  };

  try {
    const report = await auditActorSecretSafely({
      id: 'actor-id',
      name: 'legacy-actor',
      isPublic: true,
    }, 'token');

    assert.equal(report.status, 'SECRET_PRESENT');
    assert.equal(report.versionNumber, '0.0');
    assert.deepEqual(requestedPaths, ['/v2/acts/actor-id', '/v2/acts/actor-id/versions/0.0']);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test('auditActorSecretSafely fetches detail when list actor has empty versions metadata', async () => {
  const originalFetch = globalThis.fetch;
  const requestedPaths = [];

  globalThis.fetch = async (url) => {
    const path = new URL(url).pathname;
    requestedPaths.push(path);

    if (path === '/v2/acts/actor-id') {
      return {
        ok: true,
        json: async () => ({
          data: {
            id: 'actor-id',
            name: 'actor-with-empty-list-versions',
            isPublic: true,
            defaultRunOptions: { build: 'latest' },
            versions: [{ versionNumber: '1.0', buildTag: 'latest' }],
          },
        }),
      };
    }

    if (path === '/v2/acts/actor-id/versions/1.0') {
      return {
        ok: true,
        json: async () => ({
          data: {
            versionNumber: '1.0',
            envVars: [{ name: 'SCRAPPA_API_KEY', isSecret: true }],
          },
        }),
      };
    }

    return {
      ok: false,
      status: 404,
      text: async () => 'not found',
    };
  };

  try {
    const report = await auditActorSecretSafely({
      id: 'actor-id',
      name: 'actor-with-empty-list-versions',
      isPublic: true,
      versions: [],
    }, 'token');

    assert.equal(report.status, 'SECRET_PRESENT');
    assert.deepEqual(requestedPaths, ['/v2/acts/actor-id', '/v2/acts/actor-id/versions/1.0']);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test('auditActorSecretSafely reports fetch errors when actor visibility is unknown', async () => {
  const originalFetch = globalThis.fetch;

  globalThis.fetch = async () => {
    return {
      ok: false,
      status: 400,
      text: async () => 'temporary failure',
    };
  };

  try {
    const report = await auditActorSecretSafely({
      id: 'unknown-visibility-id',
      name: 'unknown-visibility-actor',
    }, 'token');

    assert.equal(report.status, 'ERROR');
    assert.equal(report.actorId, 'unknown-visibility-id');
    assert.match(report.reason, /Apify API request failed \(400\)/);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test('auditActorSecretSafely skips fetch errors for known private actors', async () => {
  const originalFetch = globalThis.fetch;

  globalThis.fetch = async () => {
    return {
      ok: false,
      status: 400,
      text: async () => 'temporary failure',
    };
  };

  try {
    const report = await auditActorSecretSafely({
      id: 'private-id',
      name: 'private-actor',
      isPublic: false,
    }, 'token');

    assert.equal(report, null);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test('auditActorSecrets filters private actors after fetching details', async () => {
  const originalFetch = globalThis.fetch;

  globalThis.fetch = async (url) => {
    const path = new URL(url).pathname;

    if (path === '/v2/acts/public-id') {
      return {
        ok: true,
        json: async () => ({
          data: {
            id: 'public-id',
            name: 'public-actor',
            isPublic: true,
            defaultRunOptions: { build: 'latest' },
            versions: [{ versionNumber: '1.0', buildTag: 'latest' }],
          },
        }),
      };
    }

    if (path === '/v2/acts/public-id/versions/1.0') {
      return {
        ok: true,
        json: async () => ({
          data: {
            versionNumber: '1.0',
            envVars: [{ name: 'SCRAPPA_API_KEY', isSecret: true }],
          },
        }),
      };
    }

    if (path === '/v2/acts/private-id') {
      return {
        ok: true,
        json: async () => ({
          data: {
            id: 'private-id',
            name: 'private-actor',
            isPublic: false,
            defaultRunOptions: { build: 'latest' },
            versions: [{ versionNumber: '1.0', buildTag: 'latest' }],
          },
        }),
      };
    }

    return {
      ok: false,
      status: 404,
      text: async () => 'not found',
    };
  };

  try {
    const reports = await auditActorSecrets([
      { id: 'public-id', name: 'public-actor' },
      { id: 'private-id', name: 'private-actor' },
    ], 'token');

    assert.equal(reports.length, 1);
    assert.equal(reports[0].actorId, 'public-id');
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test('parseArgs supports JSON output and included passing actors', () => {
  assert.deepEqual(parseArgs(['--json', '--include-present']), {
    help: false,
    includePresent: true,
    json: true,
  });
});

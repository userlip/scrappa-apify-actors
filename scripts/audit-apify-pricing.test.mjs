import assert from 'node:assert/strict';
import test from 'node:test';

import {
  apifyGet,
  auditActor,
  fetchActorDetailSafely,
  formatDetailFetchError,
  getRetryDelayMs,
  isPaidPricingInfo,
  parseArgs,
} from './audit-apify-pricing.mjs';

const now = new Date('2026-05-17T15:00:00.000Z');

test('auditActor flags missing paid pricingInfos even when active fields look paid', () => {
  const report = auditActor({
    id: 'actor-missing-pricing-infos',
    name: 'missing-pricing-infos',
    isPublic: true,
    pricingInfos: [],
    currentPricingInfo: {
      pricingModel: 'PAY_PER_EVENT',
      startedAt: '2026-05-17T14:00:00.000Z',
    },
  }, now);

  assert.equal(report.status, 'MISSING_PAID_PRICING');
  assert.equal(report.currentPricingInfoPresent, true);
});

test('auditActor marks due paid pricing without active evidence as overdue', () => {
  const report = auditActor({
    id: 'actor-overdue',
    name: 'overdue',
    isPublic: true,
    pricingInfos: [{
      pricingModel: 'PAY_PER_EVENT',
      startedAt: '2026-05-17T14:00:00.000Z',
    }],
    currentPricingInfo: null,
    pricingInfo: null,
  }, now);

  assert.equal(report.status, 'OVERDUE_MISSING_ACTIVE_PRICING');
  assert.equal(report.overduePaidPricingInfos[0].startedAt, '2026-05-17T14:00:00.000Z');
});

test('auditActor marks future paid pricing as scheduled', () => {
  const report = auditActor({
    id: 'actor-future',
    name: 'future',
    isPublic: true,
    pricingInfos: [{
      pricingModel: 'PAY_PER_EVENT',
      startedAt: '2026-05-18T14:00:00.000Z',
    }],
  }, now);

  assert.equal(report.status, 'FUTURE_ONLY_PAID_PRICING');
  assert.equal(report.nextPaidPricingInfo.startedAt, '2026-05-18T14:00:00.000Z');
});

test('auditActor accepts active paid evidence only when pricingInfos are paid', () => {
  const report = auditActor({
    id: 'actor-active',
    name: 'active',
    isPublic: true,
    pricingInfos: [{
      pricingModel: 'PAY_PER_EVENT',
      startedAt: '2026-05-17T14:00:00.000Z',
    }],
    currentPricingInfo: {
      pricingModel: 'PAY_PER_EVENT',
      startedAt: '2026-05-17T14:00:00.000Z',
    },
  }, now);

  assert.equal(report.status, 'ACTIVE_PAID_PRICING');
});

test('isPaidPricingInfo detects paid event pricing fallbacks', () => {
  assert.equal(isPaidPricingInfo({
    pricingModel: 'UNKNOWN_MODEL',
    pricingPerEvent: {
      result: { priceUsd: 0.0002 },
    },
  }), true);

  assert.equal(isPaidPricingInfo({
    pricingModel: 'FREE',
    pricingPerEvent: {
      result: { priceUsd: 0.0002 },
    },
  }), false);
});

test('isPaidPricingInfo detects positive price fields without pricingModel', () => {
  assert.equal(isPaidPricingInfo({
    pricePerUnitUsd: 0.0002,
  }), true);

  assert.equal(isPaidPricingInfo({
    pricePerDatasetItemUsd: '0.0002',
  }), true);

  assert.equal(isPaidPricingInfo({
    pricingModel: 'FREE',
    pricePerUnitUsd: 0.0002,
  }), false);
});

test('parseArgs rejects --now without a date value', () => {
  assert.throws(() => parseArgs(['--now', '--json']), /--now requires an ISO date value/);
});

test('getRetryDelayMs uses retry-after when present and fallback otherwise', () => {
  const response = {
    headers: {
      get(name) {
        return name === 'retry-after' ? '2' : null;
      },
    },
  };

  assert.equal(getRetryDelayMs(response, 1), 2000);
  assert.equal(getRetryDelayMs(null, 2), 1500);
});

test('getRetryDelayMs parses HTTP-date retry-after values', () => {
  const future = new Date(Date.now() + 5000);
  const response = {
    headers: {
      get(name) {
        return name === 'retry-after' ? future.toUTCString() : null;
      },
    },
  };

  const delay = getRetryDelayMs(response, 1);

  assert.ok(delay > 0);
  assert.ok(delay <= 5000);
});

test('apifyGet retries transient fetch failures', async () => {
  const originalFetch = globalThis.fetch;
  let calls = 0;

  globalThis.fetch = async () => {
    calls += 1;
    if (calls === 1) {
      throw new TypeError('temporary network failure');
    }

    return {
      ok: true,
      json: async () => ({ data: { ok: true } }),
    };
  };

  try {
    const result = await apifyGet('/test', 'token');

    assert.deepEqual(result, { data: { ok: true } });
    assert.equal(calls, 2);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test('fetchActorDetailSafely returns detail fetch errors without throwing', async () => {
  const originalFetch = globalThis.fetch;

  globalThis.fetch = async () => {
    return {
      ok: false,
      status: 400,
      text: async () => 'bad actor id',
    };
  };

  try {
    const result = await fetchActorDetailSafely({ id: 'actor-id', name: 'actor-slug' }, 'token');
    const report = formatDetailFetchError(result.actor, result.error);

    assert.equal(result.detail, null);
    assert.equal(result.error instanceof Error, true);
    assert.equal(report.status, 'ERROR');
    assert.equal(report.actorId, 'actor-id');
    assert.match(report.reason, /Failed to fetch actor detail/);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

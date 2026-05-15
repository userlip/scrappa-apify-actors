#!/usr/bin/env node

import { pathToFileURL } from 'node:url';

const APIFY_API_BASE_URL = 'https://api.apify.com/v2';
const ACTOR_DETAIL_CONCURRENCY = 3;
const MAX_API_ATTEMPTS = 4;
const PAID_PRICING_MODELS = new Set([
  'PRICE_PER_DATASET_ITEM',
  'PRICE_PER_EVENT',
  'PAY_PER_EVENT',
  'MONTHLY_SUBSCRIPTION',
  'FLAT_PRICE_PER_MONTH',
  'RENTAL',
]);

if (isCliEntrypoint()) {
  runCli().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(2);
  });
}

async function runCli() {
  const args = parseArgs(process.argv.slice(2));
  const token = process.env.APIFY_TOKEN || process.env.APIFY_API_TOKEN;
  const now = args.now ? parseDate(args.now, '--now') : new Date();

  if (args.help) {
    printHelp();
    return;
  }

  if (!token) {
    console.error('Missing APIFY_TOKEN or APIFY_API_TOKEN.');
    console.error('Run with: APIFY_TOKEN=... pnpm audit:pricing');
    process.exit(2);
  }

  const listedActors = await fetchAllActors(token);
  const detailResults = await mapWithConcurrency(listedActors, ACTOR_DETAIL_CONCURRENCY, async (actor) => fetchActorDetailSafely(actor, token));
  const reports = createAuditReports(detailResults, now);

  const overdueMissingActive = reports.filter((report) => report.status === 'OVERDUE_MISSING_ACTIVE_PRICING');
  const missingPaidPricing = reports.filter((report) => report.status === 'MISSING_PAID_PRICING');
  const futureOnly = reports.filter((report) => report.status === 'FUTURE_ONLY_PAID_PRICING');
  const active = reports.filter((report) => report.status === 'ACTIVE_PAID_PRICING');
  const errors = reports.filter((report) => report.status === 'ERROR');

  if (args.json) {
    console.log(JSON.stringify({
      checkedAt: now.toISOString(),
      publicActors: reports.length,
      summary: {
        activePaidPricing: active.length,
        overdueMissingActivePricing: overdueMissingActive.length,
        missingPaidPricing: missingPaidPricing.length,
        futureOnlyPaidPricing: futureOnly.length,
        errors: errors.length,
      },
      overdueMissingActivePricing: overdueMissingActive,
      missingPaidPricing,
      futureOnlyPaidPricing: futureOnly,
      errors,
      activePaidPricing: args.includeActive ? active : undefined,
    }, null, 2));
  } else {
    printReport({
      checkedAt: now,
      publicActorsCount: reports.length,
      active,
      overdueMissingActive,
      missingPaidPricing,
      futureOnly,
      errors,
    });
  }

  if (overdueMissingActive.length > 0 || missingPaidPricing.length > 0 || errors.length > 0) {
    process.exit(1);
  }
}

export function createAuditReports(detailResults, nowDate) {
  const detailErrors = detailResults
    .filter((result) => result.error && result.actor?.isPublic === true)
    .map((result) => formatDetailFetchError(result.actor, result.error));
  const publicActors = detailResults
    .map((result) => result.detail)
    .filter((actor) => actor?.isPublic === true);

  return [
    ...publicActors.map((actor) => auditActorSafely(actor, nowDate)),
    ...detailErrors,
  ];
}

export function auditActor(actor, nowDate) {
  const pricingInfos = Array.isArray(actor.pricingInfos) ? actor.pricingInfos : [];
  const paidPricingInfos = pricingInfos.filter(isPaidPricingInfo);
  const duePaidPricingInfos = paidPricingInfos.filter((pricingInfo) => isStarted(pricingInfo, nowDate));
  const activeEvidence = getActivePricingEvidence(actor, nowDate, duePaidPricingInfos);
  const nextPaidPricingInfo = paidPricingInfos
    .filter((pricingInfo) => !isStarted(pricingInfo, nowDate))
    .sort(compareStartedAt)[0];

  const base = {
    actorId: actor.id,
    slug: actor.name,
    title: actor.title || null,
    isPublic: actor.isPublic === true,
    pricingInfoPresent: actor.pricingInfo != null,
    currentPricingInfoPresent: actor.currentPricingInfo != null,
    paidPricingInfos: paidPricingInfos.map(formatPricingInfo),
    activeEvidence,
  };

  if (paidPricingInfos.length === 0) {
    return {
      ...base,
      status: 'MISSING_PAID_PRICING',
      reason: 'Public actor has no paid pricingInfos.',
    };
  }

  if (activeEvidence) {
    return {
      ...base,
      status: 'ACTIVE_PAID_PRICING',
      reason: `Active pricing evidence found in ${activeEvidence.source}.`,
    };
  }

  if (duePaidPricingInfos.length > 0) {
    return {
      ...base,
      status: 'OVERDUE_MISSING_ACTIVE_PRICING',
      reason: 'A paid pricingInfo has started, but pricingInfo/currentPricingInfo still has no paid active evidence.',
      overduePaidPricingInfos: duePaidPricingInfos.map(formatPricingInfo),
    };
  }

  return {
    ...base,
    status: 'FUTURE_ONLY_PAID_PRICING',
    reason: 'Paid pricing is scheduled for a future activation time.',
    nextPaidPricingInfo: formatPricingInfo(nextPaidPricingInfo),
  };
}

function auditActorSafely(actor, nowDate) {
  try {
    return auditActor(actor, nowDate);
  } catch (error) {
    return {
      actorId: actor.id,
      slug: actor.name,
      title: actor.title || null,
      isPublic: actor.isPublic === true,
      status: 'ERROR',
      reason: error instanceof Error ? error.message : String(error),
    };
  }
}

export function getActivePricingEvidence(actor, nowDate, duePaidPricingInfos = null) {
  for (const field of ['currentPricingInfo', 'pricingInfo']) {
    const value = actor[field];
    if (value && isPaidPricingInfo(value) && isStarted(value, nowDate)) {
      return {
        source: field,
        pricing: formatPricingInfo(value),
      };
    }
  }

  const startedPaidPricingInfos = duePaidPricingInfos ??
    (Array.isArray(actor.pricingInfos)
      ? actor.pricingInfos.filter((pricingInfo) => isPaidPricingInfo(pricingInfo) && isStarted(pricingInfo, nowDate))
      : []);
  const latestStartedPaidPricingInfo = [...startedPaidPricingInfos].sort(compareStartedAt).at(-1);
  if (latestStartedPaidPricingInfo) {
    return {
      source: 'pricingInfos',
      pricing: formatPricingInfo(latestStartedPaidPricingInfo),
    };
  }

  return null;
}

export function isPaidPricingInfo(pricingInfo) {
  const pricingModel = pricingInfo?.pricingModel || pricingInfo?.pricingModelType;
  if (pricingModel === 'FREE') {
    return false;
  }

  if (PAID_PRICING_MODELS.has(pricingModel)) {
    return true;
  }

  return hasPositivePrice(pricingInfo?.pricePerUnitUsd) ||
    hasPositivePrice(pricingInfo?.pricePerDatasetItemUsd) ||
    hasPaidPricingEvent(pricingInfo);
}

function hasPositivePrice(value) {
  return Number(value) > 0;
}

function hasPaidPricingEvent(pricingInfo) {
  const pricingPerEvent = pricingInfo?.pricingPerEvent || pricingInfo?.eventPrices;
  if (!pricingPerEvent || typeof pricingPerEvent !== 'object') {
    return false;
  }

  return Object.values(pricingPerEvent).some(hasPaidEventPrice);
}

function hasPaidEventPrice(event) {
  if (typeof event === 'number') {
    return event > 0;
  }

  if (!event || typeof event !== 'object') {
    return false;
  }

  if (
    hasPositivePrice(event.priceUsd) ||
    hasPositivePrice(event.price) ||
    hasPositivePrice(event.unitPriceUsd)
  ) {
    return true;
  }

  return Object.values(event).some(hasPaidEventPrice);
}

export function isStarted(pricingInfo, nowDate) {
  const startedAt = pricingInfo?.startedAt || pricingInfo?.startAt;
  if (!startedAt) {
    return true;
  }

  return parseDate(startedAt, 'pricingInfo.startedAt').getTime() <= nowDate.getTime();
}

function compareStartedAt(left, right) {
  return getStartedAtTime(left) - getStartedAtTime(right);
}

function getStartedAtTime(pricingInfo) {
  const startedAt = pricingInfo?.startedAt || pricingInfo?.startAt;
  return startedAt ? parseDate(startedAt, 'pricingInfo.startedAt').getTime() : 0;
}

function formatPricingInfo(pricingInfo) {
  if (!pricingInfo) {
    return null;
  }

  return {
    pricingModel: pricingInfo.pricingModel || pricingInfo.pricingModelType || null,
    startedAt: pricingInfo.startedAt || pricingInfo.startAt || null,
    pricePerUnitUsd: pricingInfo.pricePerUnitUsd ?? null,
    pricePerDatasetItemUsd: pricingInfo.pricePerDatasetItemUsd ?? null,
    pricingPerEvent: pricingInfo.pricingPerEvent ?? pricingInfo.eventPrices ?? null,
  };
}

async function fetchAllActors(token) {
  const actors = [];
  let offset = 0;
  const limit = 1000;

  while (true) {
    const result = await apifyGet(`/acts?my=1&limit=${limit}&offset=${offset}`, token);
    const items = result?.data?.items;
    if (!Array.isArray(items)) {
      throw new Error('Unexpected Apify actor list response: missing data.items.');
    }

    actors.push(...items);

    if (items.length < limit) {
      return actors;
    }

    offset += limit;
  }
}

async function fetchActorDetail(actorId, token) {
  const result = await apifyGet(`/acts/${encodeURIComponent(actorId)}`, token);
  if (!result?.data?.id) {
    throw new Error(`Unexpected Apify actor detail response for ${actorId}.`);
  }

  return result.data;
}

export async function fetchActorDetailSafely(actor, token) {
  try {
    return {
      actor,
      detail: await fetchActorDetail(actor.id, token),
      error: null,
    };
  } catch (error) {
    return {
      actor,
      detail: null,
      error,
    };
  }
}

export function formatDetailFetchError(actor, error) {
  return {
    actorId: actor.id,
    slug: actor.name,
    title: actor.title || null,
    isPublic: actor.isPublic === true,
    status: 'ERROR',
    reason: `Failed to fetch actor detail: ${error instanceof Error ? error.message : String(error)}`,
  };
}

export async function apifyGet(path, token) {
  let lastError;

  for (let attempt = 1; attempt <= MAX_API_ATTEMPTS; attempt += 1) {
    let response;
    try {
      response = await fetch(`${APIFY_API_BASE_URL}${path}`, {
        headers: {
          Authorization: `Bearer ${token}`,
          Accept: 'application/json',
        },
      });
    } catch (error) {
      lastError = error;
      if (attempt < MAX_API_ATTEMPTS) {
        await sleep(getRetryDelayMs(null, attempt));
        continue;
      }

      break;
    }

    if (response.ok) {
      return response.json();
    }

    const body = await response.text();
    if (attempt < MAX_API_ATTEMPTS && (response.status === 429 || response.status >= 500)) {
      await sleep(getRetryDelayMs(response, attempt));
      continue;
    }

    throw new Error(`Apify API request failed (${response.status}) for ${path}: ${body}`);
  }

  const message = lastError instanceof Error ? lastError.message : String(lastError);
  throw new Error(`Apify API request failed for ${path}: ${message}`);
}

async function mapWithConcurrency(items, concurrency, callback) {
  const results = new Array(items.length);
  let index = 0;

  async function worker() {
    while (index < items.length) {
      const currentIndex = index;
      index += 1;
      results[currentIndex] = await callback(items[currentIndex], currentIndex);
    }
  }

  await Promise.all(Array.from({ length: Math.min(concurrency, items.length) }, worker));
  return results;
}

function printReport({ checkedAt, publicActorsCount, active, overdueMissingActive, missingPaidPricing, futureOnly, errors }) {
  console.log(`Apify pricing activation audit checked at ${checkedAt.toISOString()}`);
  console.log(`Public actors checked: ${publicActorsCount}`);
  console.log(`Active paid pricing evidence: ${active.length}`);
  console.log(`Overdue missing active pricing: ${overdueMissingActive.length}`);
  console.log(`Missing paid pricing: ${missingPaidPricing.length}`);
  console.log(`Future-only paid pricing: ${futureOnly.length}`);
  console.log(`Actor audit errors: ${errors.length}`);

  printActorSection('OVERDUE: paid pricing should be active but is not visible', overdueMissingActive, (report) => {
    const due = report.overduePaidPricingInfos?.[0];
    return `${actorLabel(report)} due ${due?.startedAt || 'unknown date'} (${due?.pricingModel || 'unknown model'})`;
  });

  printActorSection('MISSING: public actors without paid pricingInfos', missingPaidPricing, actorLabel);

  printActorSection('ERROR: actors that could not be audited', errors, (report) => {
    return `${actorLabel(report)} - ${report.reason}`;
  });

  printActorSection('SCHEDULED: paid pricing still starts in the future', futureOnly, (report) => {
    const next = report.nextPaidPricingInfo;
    return `${actorLabel(report)} starts ${next?.startedAt || 'unknown date'} (${next?.pricingModel || 'unknown model'})`;
  });
}

function printActorSection(title, reports, formatter) {
  if (reports.length === 0) {
    return;
  }

  console.log(`\n${title}`);
  for (const report of reports) {
    console.log(`- ${formatter(report)}`);
  }
}

function actorLabel(report) {
  return `${report.actorId} - ${report.slug}`;
}

export function parseArgs(argv) {
  const parsed = {
    help: false,
    includeActive: false,
    json: false,
    now: null,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === '--') {
      continue;
    } else if (arg === '--help' || arg === '-h') {
      parsed.help = true;
    } else if (arg === '--include-active') {
      parsed.includeActive = true;
    } else if (arg === '--json') {
      parsed.json = true;
    } else if (arg === '--now') {
      const value = argv[index + 1];
      if (!value || value.startsWith('--')) {
        throw new Error('--now requires an ISO date value.');
      }

      parsed.now = value;
      index += 1;
    } else if (arg.startsWith('--now=')) {
      parsed.now = arg.slice('--now='.length);
    } else {
      throw new Error(`Unknown argument: ${arg}`);
    }
  }

  return parsed;
}

function parseDate(value, label) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    throw new Error(`Invalid ${label}: ${value}`);
  }

  return date;
}

export function getRetryDelayMs(response, attempt) {
  const retryAfter = response?.headers?.get('retry-after');
  const retryAfterSeconds = retryAfter ? Number(retryAfter) : NaN;
  if (Number.isFinite(retryAfterSeconds) && retryAfterSeconds > 0) {
    return retryAfterSeconds * 1000;
  }

  if (retryAfter) {
    const retryAfterDate = new Date(retryAfter);
    if (!Number.isNaN(retryAfterDate.getTime())) {
      return Math.max(0, retryAfterDate.getTime() - Date.now());
    }
  }

  return attempt * 750;
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function isCliEntrypoint() {
  return process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href;
}

function printHelp() {
  console.log(`Audit public Apify actors for scheduled paid pricing that should be active.

Usage:
  APIFY_TOKEN=... pnpm audit:pricing
  APIFY_TOKEN=... pnpm audit:pricing --now 2026-05-17T15:00:00.000Z

Options:
  --json             Print machine-readable JSON.
  --include-active   Include active actor details in JSON output.
  --now <iso-date>   Evaluate activation against a specific time.
  -h, --help         Show this help.
`);
}

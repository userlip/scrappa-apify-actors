#!/usr/bin/env node

const APIFY_API_BASE_URL = 'https://api.apify.com/v2';
const ACTOR_DETAIL_CONCURRENCY = 3;
const MAX_API_ATTEMPTS = 4;
const PAID_PRICING_MODELS = new Set([
  'PRICE_PER_DATASET_ITEM',
  'PRICE_PER_EVENT',
  'PAY_PER_EVENT',
  'MONTHLY_SUBSCRIPTION',
  'RENTAL',
]);

const args = parseArgs(process.argv.slice(2));
const token = process.env.APIFY_TOKEN || process.env.APIFY_API_TOKEN;
const now = args.now ? parseDate(args.now, '--now') : new Date();

if (args.help) {
  printHelp();
  process.exit(0);
}

if (!token) {
  console.error('Missing APIFY_TOKEN or APIFY_API_TOKEN.');
  console.error('Run with: APIFY_TOKEN=... pnpm audit:pricing');
  process.exit(2);
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(2);
});

async function main() {
  const listedActors = await fetchAllActors();
  const details = await mapWithConcurrency(listedActors, ACTOR_DETAIL_CONCURRENCY, async (actor) => fetchActorDetail(actor.id));
  const publicActors = details.filter((actor) => actor.isPublic === true);
  const reports = publicActors.map((actor) => auditActor(actor, now));

  const overdueMissingActive = reports.filter((report) => report.status === 'OVERDUE_MISSING_ACTIVE_PRICING');
  const missingPaidPricing = reports.filter((report) => report.status === 'MISSING_PAID_PRICING');
  const futureOnly = reports.filter((report) => report.status === 'FUTURE_ONLY_PAID_PRICING');
  const active = reports.filter((report) => report.status === 'ACTIVE_PAID_PRICING');

  if (args.json) {
    console.log(JSON.stringify({
      checkedAt: now.toISOString(),
      publicActors: reports.length,
      summary: {
        activePaidPricing: active.length,
        overdueMissingActivePricing: overdueMissingActive.length,
        missingPaidPricing: missingPaidPricing.length,
        futureOnlyPaidPricing: futureOnly.length,
      },
      overdueMissingActivePricing: overdueMissingActive,
      missingPaidPricing,
      futureOnlyPaidPricing: futureOnly,
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
    });
  }

  if (overdueMissingActive.length > 0 || missingPaidPricing.length > 0) {
    process.exit(1);
  }
}

function auditActor(actor, nowDate) {
  const pricingInfos = Array.isArray(actor.pricingInfos) ? actor.pricingInfos : [];
  const paidPricingInfos = pricingInfos.filter(isPaidPricingInfo);
  const activeEvidence = getActivePricingEvidence(actor, nowDate);
  const duePaidPricingInfos = paidPricingInfos.filter((pricingInfo) => isStarted(pricingInfo, nowDate));
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

  if (paidPricingInfos.length > 0) {
    return {
      ...base,
      status: 'FUTURE_ONLY_PAID_PRICING',
      reason: 'Paid pricing is scheduled for a future activation time.',
      nextPaidPricingInfo: formatPricingInfo(nextPaidPricingInfo),
    };
  }

  return {
    ...base,
    status: 'MISSING_PAID_PRICING',
    reason: 'Public actor has no paid pricingInfos.',
  };
}

function getActivePricingEvidence(actor, nowDate) {
  for (const field of ['currentPricingInfo', 'pricingInfo']) {
    const value = actor[field];
    if (value && isPaidPricingInfo(value) && isStarted(value, nowDate)) {
      return {
        source: field,
        pricing: formatPricingInfo(value),
      };
    }
  }

  return null;
}

function isPaidPricingInfo(pricingInfo) {
  const pricingModel = pricingInfo?.pricingModel || pricingInfo?.pricingModelType;
  if (!pricingModel || pricingModel === 'FREE') {
    return false;
  }

  if (PAID_PRICING_MODELS.has(pricingModel)) {
    return true;
  }

  return pricingInfo?.pricePerUnitUsd != null ||
    pricingInfo?.pricePerDatasetItemUsd != null ||
    hasPaidPricingEvent(pricingInfo);
}

function hasPaidPricingEvent(pricingInfo) {
  const pricingPerEvent = pricingInfo?.pricingPerEvent || pricingInfo?.eventPrices;
  if (!pricingPerEvent || typeof pricingPerEvent !== 'object') {
    return false;
  }

  return Object.values(pricingPerEvent).some((event) => {
    if (typeof event === 'number') {
      return event > 0;
    }

    return Number(event?.priceUsd || event?.price || event?.unitPriceUsd || 0) > 0;
  });
}

function isStarted(pricingInfo, nowDate) {
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

async function fetchAllActors() {
  const actors = [];
  let offset = 0;
  const limit = 1000;

  while (true) {
    const result = await apifyGet(`/acts?my=1&limit=${limit}&offset=${offset}`);
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

async function fetchActorDetail(actorId) {
  const result = await apifyGet(`/acts/${encodeURIComponent(actorId)}`);
  if (!result?.data?.id) {
    throw new Error(`Unexpected Apify actor detail response for ${actorId}.`);
  }

  return result.data;
}

async function apifyGet(path) {
  for (let attempt = 1; attempt <= MAX_API_ATTEMPTS; attempt += 1) {
    const response = await fetch(`${APIFY_API_BASE_URL}${path}`, {
      headers: {
        Authorization: `Bearer ${token}`,
        Accept: 'application/json',
      },
    });

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

  throw new Error(`Apify API request failed for ${path}.`);
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

function printReport({ checkedAt, publicActorsCount, active, overdueMissingActive, missingPaidPricing, futureOnly }) {
  console.log(`Apify pricing activation audit checked at ${checkedAt.toISOString()}`);
  console.log(`Public actors checked: ${publicActorsCount}`);
  console.log(`Active paid pricing evidence: ${active.length}`);
  console.log(`Overdue missing active pricing: ${overdueMissingActive.length}`);
  console.log(`Missing paid pricing: ${missingPaidPricing.length}`);
  console.log(`Future-only paid pricing: ${futureOnly.length}`);

  printActorSection('OVERDUE: paid pricing should be active but is not visible', overdueMissingActive, (report) => {
    const due = report.overduePaidPricingInfos?.[0];
    return `${actorLabel(report)} due ${due?.startedAt || 'unknown date'} (${due?.pricingModel || 'unknown model'})`;
  });

  printActorSection('MISSING: public actors without paid pricingInfos', missingPaidPricing, actorLabel);

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

function parseArgs(argv) {
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
      parsed.now = argv[index + 1];
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

function getRetryDelayMs(response, attempt) {
  const retryAfter = response.headers.get('retry-after');
  const retryAfterSeconds = retryAfter ? Number(retryAfter) : NaN;
  if (Number.isFinite(retryAfterSeconds) && retryAfterSeconds > 0) {
    return retryAfterSeconds * 1000;
  }

  return attempt * 750;
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
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

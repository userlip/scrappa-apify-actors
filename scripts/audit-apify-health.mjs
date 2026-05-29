#!/usr/bin/env node

import { pathToFileURL } from 'node:url';

const APIFY_API_BASE_URL = 'https://api.apify.com/v2';
const ACTOR_DETAIL_CONCURRENCY = 3;
const HEALTH_FETCH_CONCURRENCY = 3;
const MAX_API_ATTEMPTS = 4;
const RECENT_RUN_LIMIT = 5;
const THESCRAPPA_USER_ID = '8683TqwnXHrQ46FhH';
const THESCRAPPA_USERNAME = 'thescrappa';
const FAILED_RUN_STATUSES = new Set(['FAILED', 'TIMED-OUT', 'ABORTED']);

if (isCliEntrypoint()) {
  runCli().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(2);
  });
}

async function runCli() {
  const args = parseArgs(process.argv.slice(2));
  const token = process.env.APIFY_TOKEN || process.env.APIFY_API_TOKEN;

  if (args.help) {
    printHelp();
    return;
  }

  if (!token) {
    console.error('Missing APIFY_TOKEN or APIFY_API_TOKEN.');
    console.error('Run with: APIFY_TOKEN=... pnpm audit:health');
    process.exit(2);
  }

  const listedActors = await fetchAllActors(token);
  const detailResults = await mapWithConcurrency(listedActors, ACTOR_DETAIL_CONCURRENCY, async (actor) => fetchActorDetailSafely(actor, token));
  const { ownedPublicActors, excludedPublicActors, errors: detailErrors } = createOwnedPublicActorScope(detailResults);
  const healthInputs = await mapWithConcurrency(ownedPublicActors, HEALTH_FETCH_CONCURRENCY, async (actor) => fetchActorHealthSafely(actor, token));
  const report = createHealthAuditReport({
    actors: healthInputs,
    excludedPublicActors,
    errors: detailErrors,
    checkedAt: new Date(),
  });

  if (args.json) {
    console.log(JSON.stringify(report, null, 2));
  } else {
    printReport(report);
  }

  if (report.summary.failedLatestRuns > 0 || report.summary.auditErrors > 0) {
    process.exit(1);
  }
}

export function createOwnedPublicActorScope(detailResults) {
  const ownedPublicActors = [];
  const excludedPublicActors = [];
  const errors = [];

  for (const result of detailResults) {
    if (result.error) {
      const actor = result.actor || {};
      if (actor.isPublic === true) {
        errors.push(formatDetailFetchError(actor, result.error));
        continue;
      }

      excludedPublicActors.push({
        actorId: actor.id,
        slug: actor.name,
        title: actor.title || null,
        userId: null,
        username: null,
        reason: `Actor detail fetch failed before public TheScrappa ownership could be confirmed; excluded from audit scope: ${formatErrorMessage(result.error)}`,
      });
      continue;
    }

    const actor = result.detail;
    if (!actor || actor.isPublic !== true) {
      continue;
    }

    const ownership = getActorOwnership(actor);
    if (ownership.isOwned) {
      ownedPublicActors.push(actor);
      continue;
    }

    excludedPublicActors.push({
      actorId: actor.id,
      slug: actor.name,
      title: actor.title || null,
      userId: ownership.userId,
      username: ownership.username,
      reason: ownership.hasOwnershipFields
        ? 'Public actor is visible to this token but is not owned by TheScrappa.'
        : 'Public actor has no userId or username ownership fields; excluded from TheScrappa audit scope.',
    });
  }

  ownedPublicActors.sort(compareActors);
  excludedPublicActors.sort(compareActors);
  errors.sort(compareActors);

  return { ownedPublicActors, excludedPublicActors, errors };
}

export function createHealthAuditReport({ actors, excludedPublicActors = [], errors = [], checkedAt = new Date() }) {
  const healthErrors = actors.filter((actor) => actor?.error).map(formatAuditError);
  const actorReports = actors.filter((actor) => !actor?.error).map((actor) => auditActorHealth(actor)).sort(compareActors);
  const noRunActors = actorReports.filter((report) => report.status === 'NO_RUNS');
  const failedLatestActors = actorReports.filter((report) => report.status === 'FAILED_LATEST_RUN');
  const recentFailedButLatestOkActors = actorReports.filter((report) => report.status === 'RECENT_FAILED_BUT_LATEST_OK');
  const noticeActors = actorReports.filter((report) => report.notice);
  const okActors = actorReports.filter((report) => report.status === 'OK');
  const auditErrors = [...errors.map(formatAuditError), ...healthErrors].sort(compareActors);

  return {
    checkedAt: checkedAt.toISOString(),
    ownerFilter: {
      userId: THESCRAPPA_USER_ID,
      username: THESCRAPPA_USERNAME,
    },
    publicActors: actorReports.length,
    summary: {
      ok: okActors.length,
      noRuns: noRunActors.length,
      failedLatestRuns: failedLatestActors.length,
      recentFailedButLatestOk: recentFailedButLatestOkActors.length,
      notices: noticeActors.length,
      excludedPublicActors: excludedPublicActors.length,
      auditErrors: auditErrors.length,
    },
    actors: actorReports,
    noRunActors,
    failedLatestActors,
    recentFailedButLatestOkActors,
    noticeActors,
    excludedPublicActors,
    errors: auditErrors,
  };
}

export function auditActorHealth(input) {
  if (input.error) {
    return formatAuditError(input);
  }

  const actor = input.actor || input;
  const runs = Array.isArray(input.runs) ? input.runs : [];
  const builds = Array.isArray(input.builds) ? input.builds : [];
  const latestRun = runs[0] || null;
  const latestBuild = builds[0] || null;
  const recentStatuses = runs.map((run) => run.status || 'UNKNOWN');
  const notice = formatNotice(getActorNotice(actor));
  const recentFailedRuns = runs.filter((run) => FAILED_RUN_STATUSES.has(run.status));
  const base = {
    actorId: actor.id,
    slug: actor.name,
    title: actor.title || null,
    isPublic: actor.isPublic === true,
    userId: getActorOwnership(actor).userId,
    username: getActorOwnership(actor).username,
    latestRun: formatRun(latestRun),
    recentStatuses,
    latestBuild: formatBuild(latestBuild),
    notice,
  };

  if (!latestRun) {
    return {
      ...base,
      status: 'NO_RUNS',
      reason: `Apify returned no runs for this public TheScrappa actor in the latest ${RECENT_RUN_LIMIT}-run history request.`,
    };
  }

  if (FAILED_RUN_STATUSES.has(latestRun.status)) {
    return {
      ...base,
      status: 'FAILED_LATEST_RUN',
      reason: `Latest run status is ${latestRun.status}.`,
    };
  }

  if (recentFailedRuns.length > 0) {
    return {
      ...base,
      status: 'RECENT_FAILED_BUT_LATEST_OK',
      reason: `Latest run is ${latestRun.status || 'UNKNOWN'}, but ${recentFailedRuns.length} recent run(s) failed.`,
      recentFailedRuns: recentFailedRuns.map(formatRun),
    };
  }

  return {
    ...base,
    status: 'OK',
    reason: `Latest run status is ${latestRun.status || 'UNKNOWN'}.`,
  };
}

export function getActorOwnership(actor) {
  const userId = actor?.userId || actor?.user?.id || actor?.owner?.id || null;
  const username = actor?.username || actor?.user?.username || actor?.owner?.username || null;

  return {
    userId,
    username,
    hasOwnershipFields: Boolean(userId || username),
    isOwned: userId === THESCRAPPA_USER_ID || String(username || '').toLowerCase() === THESCRAPPA_USERNAME,
  };
}

function getActorNotice(actor) {
  if (actor?.notice) {
    return actor.notice;
  }

  if (Array.isArray(actor?.notices) && actor.notices.length > 0) {
    return actor.notices[0];
  }

  if (actor?.noticeType || actor?.noticeMessage) {
    return {
      type: actor.noticeType,
      message: actor.noticeMessage,
    };
  }

  return null;
}

function formatNotice(notice) {
  if (!notice) {
    return null;
  }

  if (typeof notice === 'string') {
    if (notice === 'NONE') {
      return null;
    }

    return {
      type: notice,
      message: null,
    };
  }

  const type = notice.type || notice.name || notice.code || null;
  if (type === 'NONE') {
    return null;
  }

  return {
    type,
    message: notice.message || notice.text || null,
  };
}

function formatRun(run) {
  if (!run) {
    return null;
  }

  return {
    id: run.id || null,
    status: run.status || null,
    startedAt: run.startedAt || null,
    finishedAt: run.finishedAt || null,
    statusMessage: run.statusMessage || null,
  };
}

function formatBuild(build) {
  if (!build) {
    return null;
  }

  return {
    id: build.id || null,
    status: build.status || null,
    startedAt: build.startedAt || null,
    finishedAt: build.finishedAt || null,
    buildNumber: build.buildNumber || null,
  };
}

function formatDetailFetchError(actor, error) {
  return {
    actorId: actor.id,
    slug: actor.name,
    title: actor.title || null,
    isPublic: actor.isPublic === true,
    status: 'ERROR',
    reason: `Failed to fetch actor detail: ${formatErrorMessage(error)}`,
  };
}

function formatAuditError(input) {
  if (input.status === 'ERROR') {
    return input;
  }

  const actor = input.actor || input;
  return {
    actorId: actor.id,
    slug: actor.name,
    title: actor.title || null,
    isPublic: actor.isPublic === true,
    status: 'ERROR',
    reason: input.reason || (input.error instanceof Error ? input.error.message : String(input.error)),
  };
}

function formatErrorMessage(error) {
  return error instanceof Error ? error.message : String(error);
}

async function fetchActorHealthSafely(actor, token) {
  try {
    const [runs, builds] = await Promise.all([
      fetchActorRuns(actor.id, token),
      fetchActorBuilds(actor.id, token),
    ]);

    return {
      actor,
      runs,
      builds,
      error: null,
    };
  } catch (error) {
    return {
      actor,
      runs: [],
      builds: [],
      error,
    };
  }
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

async function fetchActorRuns(actorId, token) {
  const result = await apifyGet(`/acts/${encodeURIComponent(actorId)}/runs?limit=${RECENT_RUN_LIMIT}&desc=true`, token);
  const items = result?.data?.items;
  if (!Array.isArray(items)) {
    throw new Error(`Unexpected Apify actor runs response for ${actorId}.`);
  }

  return items;
}

async function fetchActorBuilds(actorId, token) {
  const result = await apifyGet(`/acts/${encodeURIComponent(actorId)}/builds?limit=1&desc=true`, token);
  const items = result?.data?.items;
  if (!Array.isArray(items)) {
    throw new Error(`Unexpected Apify actor builds response for ${actorId}.`);
  }

  return items;
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

function printReport(report) {
  console.log(`Apify TheScrappa health audit checked at ${report.checkedAt}`);
  console.log(`Public TheScrappa actors checked: ${report.publicActors}`);
  console.log(`Latest run OK: ${report.summary.ok}`);
  console.log(`No recent runs: ${report.summary.noRuns}`);
  console.log(`Failed latest runs: ${report.summary.failedLatestRuns}`);
  console.log(`Recent failed but latest OK: ${report.summary.recentFailedButLatestOk}`);
  console.log(`Actor notices: ${report.summary.notices}`);
  console.log(`Excluded public non-owned/unknown actors: ${report.summary.excludedPublicActors}`);
  console.log(`Actor audit errors: ${report.summary.auditErrors}`);

  printActorSection('FAILED LATEST RUNS', report.failedLatestActors, (actor) => {
    return `${actorLabel(actor)} latest ${actor.latestRun?.id || 'unknown'} ${actor.latestRun?.status || 'UNKNOWN'}`;
  });

  printActorSection('NO RECENT RUNS', report.noRunActors, actorLabel);

  printActorSection('RECENT FAILURES BUT LATEST OK', report.recentFailedButLatestOkActors, (actor) => {
    return `${actorLabel(actor)} statuses ${actor.recentStatuses.join(', ')}`;
  });

  printActorSection('ACTOR NOTICES', report.noticeActors, (actor) => {
    return `${actorLabel(actor)} ${actor.notice?.type || 'NOTICE'}: ${actor.notice?.message || 'no message'}`;
  });

  printActorSection('EXCLUDED PUBLIC ACTORS', report.excludedPublicActors, (actor) => {
    return `${actorLabel(actor)} owner userId=${actor.userId || 'unknown'} username=${actor.username || 'unknown'} - ${actor.reason}`;
  });

  printActorSection('ERRORS', report.errors, (actor) => {
    return `${actorLabel(actor)} - ${actor.reason}`;
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

function compareActors(left, right) {
  return String(left.slug || left.name || '').localeCompare(String(right.slug || right.name || '')) ||
    String(left.actorId || left.id || '').localeCompare(String(right.actorId || right.id || ''));
}

export function parseArgs(argv) {
  const parsed = {
    help: false,
    json: false,
  };

  for (const arg of argv) {
    if (arg === '--') {
      continue;
    } else if (arg === '--help' || arg === '-h') {
      parsed.help = true;
    } else if (arg === '--json') {
      parsed.json = true;
    } else {
      throw new Error(`Unknown argument: ${arg}`);
    }
  }

  return parsed;
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
  console.log(`Audit public TheScrappa-owned Apify actors for recent run/build/notice health.

The audit strictly includes only public actors owned by userId ${THESCRAPPA_USER_ID} or username ${THESCRAPPA_USERNAME}.

Usage:
  APIFY_TOKEN=... pnpm audit:health
  APIFY_TOKEN=... pnpm audit:health --json

Options:
  --json       Print machine-readable JSON.
  -h, --help   Show this help.
`);
}

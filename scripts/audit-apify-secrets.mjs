#!/usr/bin/env node

import { pathToFileURL } from 'node:url';

const APIFY_API_BASE_URL = 'https://api.apify.com/v2';
const ACTOR_DETAIL_CONCURRENCY = 3;
const MAX_API_ATTEMPTS = 4;
const REQUIRED_SECRET_NAME = 'SCRAPPA_API_KEY';

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
    console.error('Run with: APIFY_TOKEN=... pnpm audit:secrets');
    process.exit(2);
  }

  const listedActors = await fetchAllActors(token);
  const reports = await auditActorSecrets(listedActors, token);
  const missing = reports.filter((report) => report.status === 'MISSING_SECRET');
  const nonSecret = reports.filter((report) => report.status === 'NOT_SECRET');
  const errors = reports.filter((report) => report.status === 'ERROR');
  const present = reports.filter((report) => report.status === 'SECRET_PRESENT');

  if (args.json) {
    console.log(JSON.stringify({
      checkedAt: new Date().toISOString(),
      publicActors: reports.length,
      secretName: REQUIRED_SECRET_NAME,
      summary: {
        secretPresent: present.length,
        missingSecret: missing.length,
        notSecret: nonSecret.length,
        errors: errors.length,
      },
      missingSecret: missing,
      notSecret: nonSecret,
      errors,
      secretPresent: args.includePresent ? present : undefined,
    }, null, 2));
  } else {
    printReport({ present, missing, nonSecret, errors });
  }

  if (missing.length > 0 || nonSecret.length > 0 || errors.length > 0) {
    process.exit(1);
  }
}

export async function auditActorSecrets(actors, token) {
  const reports = await mapWithConcurrency(actors, ACTOR_DETAIL_CONCURRENCY, async (actor) => {
    return auditActorSecretSafely(actor, token);
  });

  return reports.filter(Boolean);
}

export async function auditActorSecretSafely(actor, token) {
  let detail = null;

  try {
    detail = hasVersionMetadata(actor) ? actor : await fetchActorDetail(actor.id, token);
    if (detail.isPublic !== true) {
      return null;
    }

    const versionNumber = resolveDefaultVersionNumber(detail);
    const version = await fetchActorVersion(detail.id, versionNumber, token);

    return auditActorVersionSecret(detail, version);
  } catch (error) {
    const actorForReport = detail || actor;
    if (actorForReport.isPublic === false) {
      return null;
    }

    return {
      actorId: actorForReport.id,
      slug: actorForReport.name,
      title: actorForReport.title || null,
      isPublic: actorForReport.isPublic === true,
      status: 'ERROR',
      reason: error instanceof Error ? error.message : String(error),
    };
  }
}

function hasVersionMetadata(actor) {
  return Array.isArray(actor?.versions) && actor.versions.length > 0;
}

export function auditActorVersionSecret(actor, version) {
  const versionNumber = version.versionNumber || resolveDefaultVersionNumber(actor);
  const envVar = getEnvVar(version, REQUIRED_SECRET_NAME);
  const base = {
    actorId: actor.id,
    slug: actor.name,
    title: actor.title || null,
    isPublic: actor.isPublic === true,
    defaultBuild: actor.defaultRunOptions?.build || null,
    versionNumber,
    secretName: REQUIRED_SECRET_NAME,
  };

  if (!envVar) {
    return {
      ...base,
      status: 'MISSING_SECRET',
      reason: `${REQUIRED_SECRET_NAME} is missing from actor version ${versionNumber}.`,
    };
  }

  if (envVar.isSecret !== true) {
    return {
      ...base,
      status: 'NOT_SECRET',
      reason: `${REQUIRED_SECRET_NAME} is present on actor version ${versionNumber}, but is not marked secret.`,
    };
  }

  return {
    ...base,
    status: 'SECRET_PRESENT',
    reason: `${REQUIRED_SECRET_NAME} is configured as a secret on actor version ${versionNumber}.`,
  };
}

export function resolveDefaultVersionNumber(actor) {
  const versions = Array.isArray(actor?.versions) ? actor.versions : [];
  if (versions.length === 0) {
    throw new Error(`Actor ${actor?.id || actor?.name || 'unknown'} has no available versions.`);
  }

  const defaultBuild = actor?.defaultRunOptions?.build;
  if (defaultBuild !== undefined && defaultBuild !== null && defaultBuild !== '') {
    const versionNumber = resolveVersionNumberFromBuild(actor, defaultBuild);
    if (versionNumber) {
      return versionNumber;
    }

    if (versions.length === 1 && versions[0]?.versionNumber) {
      return versions[0].versionNumber;
    }

    throw new Error(`Actor ${actor?.id || actor?.name || 'unknown'} default build ${defaultBuild} does not match any available version.`);
  }

  const latestVersion = versions.find((version) => version.buildTag === 'latest');
  if (latestVersion?.versionNumber) {
    return latestVersion.versionNumber;
  }

  const highestVersion = [...versions].sort(compareVersionNumbers).at(-1);
  if (highestVersion?.versionNumber) {
    return highestVersion.versionNumber;
  }

  throw new Error(`Actor ${actor?.id || actor?.name || 'unknown'} has no versions with versionNumber.`);
}

function resolveVersionNumberFromBuild(actor, build) {
  const versions = Array.isArray(actor?.versions) ? actor.versions : [];
  const buildString = String(build);

  const directVersionNumber = versions.find((version) => version.versionNumber === buildString)?.versionNumber;
  if (directVersionNumber) {
    return directVersionNumber;
  }

  const taggedBuilds = actor?.taggedBuilds && typeof actor.taggedBuilds === 'object' ? actor.taggedBuilds : {};
  const taggedBuild = taggedBuilds[buildString];
  const versionNumberFromTag = getVersionNumberFromBuildNumber(taggedBuild?.buildNumber, versions);
  if (versionNumberFromTag) {
    return versionNumberFromTag;
  }

  for (const taggedBuild of Object.values(taggedBuilds)) {
    if (!matchesBuildNumber(taggedBuild, buildString)) {
      continue;
    }
    const versionNumber = getVersionNumberFromBuildNumber(taggedBuild?.buildNumber, versions);
    if (versionNumber) {
      return versionNumber;
    }
  }

  const taggedVersionNumber = versions.find((version) => version.buildTag === buildString)?.versionNumber;
  if (taggedVersionNumber) {
    return taggedVersionNumber;
  }

  return getVersionNumberFromBuildNumber(buildString, versions);
}

function matchesBuildNumber(taggedBuild, buildString) {
  return taggedBuild?.buildNumber === buildString ||
    String(taggedBuild?.buildNumberInt ?? '') === buildString;
}

function getVersionNumberFromBuildNumber(buildNumber, versions) {
  const buildString = String(buildNumber ?? '');
  if (!buildString) {
    return null;
  }

  const buildVersion = buildString.match(/^(\d+\.\d+)\.\d+$/)?.[1];
  if (buildVersion && versions.some((version) => version.versionNumber === buildVersion)) {
    return buildVersion;
  }

  const buildNumberInt = Number(buildString);
  if (Number.isSafeInteger(buildNumberInt) && buildNumberInt >= 0) {
    const major = Math.floor(buildNumberInt / 10000000);
    const minor = Math.floor((buildNumberInt % 10000000) / 100000);
    const versionNumber = `${major}.${minor}`;
    if (versions.some((version) => version.versionNumber === versionNumber)) {
      return versionNumber;
    }
  }

  return null;
}

function compareVersionNumbers(left, right) {
  const leftParts = splitVersionNumber(left.versionNumber);
  const rightParts = splitVersionNumber(right.versionNumber);
  const partCount = Math.max(leftParts.length, rightParts.length);

  for (let index = 0; index < partCount; index += 1) {
    const difference = (leftParts[index] || 0) - (rightParts[index] || 0);
    if (difference !== 0) {
      return difference;
    }
  }

  return String(left.versionNumber).localeCompare(String(right.versionNumber));
}

function splitVersionNumber(versionNumber) {
  return String(versionNumber || '')
    .split('.')
    .map((part) => Number(part))
    .map((part) => (Number.isFinite(part) ? part : 0));
}

function getEnvVar(version, name) {
  const envVars = Array.isArray(version?.envVars) ? version.envVars : [];
  return envVars.find((envVar) => envVar?.name === name);
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

async function fetchActorVersion(actorId, versionNumber, token) {
  const result = await apifyGet(`/acts/${encodeURIComponent(actorId)}/versions/${encodeURIComponent(versionNumber)}`, token);
  if (!result?.data?.versionNumber) {
    throw new Error(`Unexpected Apify actor version response for ${actorId} version ${versionNumber}.`);
  }

  return result.data;
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

function printReport({ present, missing, nonSecret, errors }) {
  console.log(`Apify ${REQUIRED_SECRET_NAME} secret audit checked at ${new Date().toISOString()}`);
  console.log(`Secret configured: ${present.length}`);
  console.log(`Missing secret: ${missing.length}`);
  console.log(`Present but not secret: ${nonSecret.length}`);
  console.log(`Actor audit errors: ${errors.length}`);

  printActorSection('MISSING: public actors without SCRAPPA_API_KEY on their default version', missing, actorLabelWithVersion);
  printActorSection('NOT SECRET: public actors with plain SCRAPPA_API_KEY on their default version', nonSecret, actorLabelWithVersion);
  printActorSection('ERROR: actors that could not be audited', errors, (report) => {
    return `${actorLabel(report)} - ${report.reason}`;
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

function actorLabelWithVersion(report) {
  return `${actorLabel(report)} version ${report.versionNumber || 'unknown'} (${report.defaultBuild || 'no default build'})`;
}

function actorLabel(report) {
  return `${report.actorId} - ${report.slug}`;
}

export function parseArgs(argv) {
  const parsed = {
    help: false,
    includePresent: false,
    json: false,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === '--') {
      continue;
    } else if (arg === '--help' || arg === '-h') {
      parsed.help = true;
    } else if (arg === '--include-present') {
      parsed.includePresent = true;
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
  console.log(`Audit public Apify actors for the SCRAPPA_API_KEY secret on their default version.

Usage:
  APIFY_TOKEN=... pnpm audit:secrets

Options:
  --json              Print machine-readable JSON.
  --include-present   Include passing actor details in JSON output.
  -h, --help          Show this help.
`);
}

import { readdir, readFile } from 'node:fs/promises';
import path from 'node:path';

const APIFY_API_BASE_URL = 'https://api.apify.com/v2';
const MAX_API_ATTEMPTS = 4;

export function getToken() {
  const token = process.env.APIFY_TOKEN || process.env.APIFY_API_TOKEN;
  if (!token) {
    throw new Error('Missing APIFY_TOKEN or APIFY_API_TOKEN.');
  }

  return token;
}

export async function fetchLiveActors(token) {
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

export async function fetchActorDetail(actorId, token) {
  const result = await apifyGet(`/acts/${encodeURIComponent(actorId)}`, token);
  if (!result?.data?.id) {
    throw new Error(`Unexpected Apify actor detail response for ${actorId}.`);
  }

  return result.data;
}

export async function buildLocalActorIndex(actorsRoot) {
  const entries = await readdir(actorsRoot, { withFileTypes: true });
  const localActors = new Map();

  for (const entry of entries) {
    if (!entry.isDirectory()) {
      continue;
    }

    const directory = entry.name;
    const actorJsonPath = path.join(actorsRoot, directory, '.actor', 'actor.json');
    const manifestName = await readActorManifestName(actorJsonPath);
    const actorSource = {
      directory,
      manifestName,
    };

    localActors.set(directory, actorSource);
    if (manifestName) {
      localActors.set(manifestName, actorSource);
    }
  }

  return localActors;
}

export function findMissingLiveActors(liveActors, localActors) {
  return liveActors
    .filter((actor) => !localActors.has(actor.name))
    .sort((left, right) => left.name.localeCompare(right.name));
}

export function isSafeSourcePath(fileName) {
  if (!fileName || typeof fileName !== 'string') {
    return false;
  }

  if (path.isAbsolute(fileName)) {
    return false;
  }

  return !fileName.split(/[\\/]+/).includes('..');
}

async function readActorManifestName(actorJsonPath) {
  try {
    const content = await readFile(actorJsonPath, 'utf8');
    const parsed = JSON.parse(content);
    return typeof parsed.name === 'string' && parsed.name.trim() ? parsed.name : null;
  } catch (error) {
    if (error?.code === 'ENOENT') {
      return null;
    }

    throw new Error(`Failed to read ${actorJsonPath}: ${error instanceof Error ? error.message : String(error)}`);
  }
}

async function apifyGet(pathname, token) {
  let lastError;

  for (let attempt = 1; attempt <= MAX_API_ATTEMPTS; attempt += 1) {
    let response;
    try {
      response = await fetch(`${APIFY_API_BASE_URL}${pathname}`, {
        headers: {
          Authorization: `Bearer ${token}`,
          Accept: 'application/json',
        },
      });
    } catch (error) {
      lastError = error;
      if (attempt < MAX_API_ATTEMPTS) {
        await sleep(attempt * 750);
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

    throw new Error(`Apify API request failed (${response.status}) for ${pathname}: ${body}`);
  }

  const message = lastError instanceof Error ? lastError.message : String(lastError);
  throw new Error(`Apify API request failed for ${pathname}: ${message}`);
}

function getRetryDelayMs(response, attempt) {
  const retryAfter = response?.headers?.get('retry-after');
  const retryAfterSeconds = retryAfter ? Number(retryAfter) : NaN;
  if (Number.isFinite(retryAfterSeconds) && retryAfterSeconds > 0) {
    return retryAfterSeconds * 1000;
  }

  return attempt * 750;
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

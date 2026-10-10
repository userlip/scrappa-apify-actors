#!/usr/bin/env node
// Safe deploy for SOURCE_FILES Actors.
//
// For each Actor: upload the local directory as the source of the version that
// currently backs the `latest` build (env vars and secrets stay untouched),
// build it under the `candidate` tag, run the candidate with its prefilled
// input, and only move `latest` to the new build when the run succeeds with at
// least one dataset item. The previous `latest` build ID is printed for rollback.
//
// The Rust binary is compiled locally in Docker (same rust:1.90-slim-bookworm image
// the Actor Dockerfiles use) and uploaded gzipped, so the Apify build only copies it.
// A source build on Apify takes ~140 s on a 4 GB builder (~$0.06); a prebuilt one ~3 s.
//
// Usage:
//   APIFY_TOKEN=... node scripts/deploy-actor.mjs <actor-dir> [<actor-dir> ...] [--dry-run] [--no-promote] [--allow-empty]
//
// Rollback:
//   APIFY_TOKEN=... node scripts/deploy-actor.mjs --rollback <actorId> <buildId>

import fs from 'node:fs';
import path from 'node:path';
import { prebuiltSourceFiles } from './prebuilt-actor.mjs';

const API = 'https://api.apify.com/v2';
const TOKEN = process.env.APIFY_TOKEN;
const args = process.argv.slice(2);
const flags = new Set(args.filter((a) => a.startsWith('--')));
const positional = args.filter((a) => !a.startsWith('--'));
const DRY = flags.has('--dry-run');
const NO_PROMOTE = flags.has('--no-promote');
const ALLOW_EMPTY = flags.has('--allow-empty');
const SKIP_DIRS = new Set(['target', 'storage', 'node_modules', '.git', 'apify_storage', 'crawlee_storage', 'dist']);
const TEXT_EXT = /\.(rs|toml|lock|json|md|txt|svg|yml|yaml|js|mjs|ts|sh)$|Dockerfile$|\.dockerignore$|\.gitignore$/;

if (!TOKEN) {
  console.error('APIFY_TOKEN is required');
  process.exit(2);
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function api(method, p, body) {
  for (let attempt = 0; attempt < 6; attempt += 1) {
    let res;
    try {
      res = await fetch(`${API}${p}${p.includes('?') ? '&' : '?'}token=${TOKEN}`, {
        method,
        headers: body === undefined ? {} : { 'Content-Type': 'application/json' },
        body: body === undefined ? undefined : JSON.stringify(body),
      });
    } catch (error) {
      // Transient network errors ("fetch failed") are retried like 5xx responses.
      await sleep(5000 * (attempt + 1));
      continue;
    }
    if (res.status === 402 || res.status === 429 || res.status >= 500) {
      await sleep(5000 * (attempt + 1));
      continue;
    }
    const text = await res.text();
    if (!res.ok) throw new Error(`${method} ${p} -> HTTP ${res.status}: ${text.slice(0, 300)}`);
    return text ? JSON.parse(text).data : null;
  }
  throw new Error(`${method} ${p} kept failing`);
}

function collectFiles(root) {
  const out = [];
  const walk = (dir) => {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      if (SKIP_DIRS.has(entry.name)) continue;
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) walk(full);
      else {
        const name = path.relative(root, full).split(path.sep).join('/');
        const buf = fs.readFileSync(full);
        out.push(TEXT_EXT.test(name)
          ? { name, format: 'TEXT', content: buf.toString('utf8') }
          : { name, format: 'BASE64', content: buf.toString('base64') });
      }
    }
  };
  walk(root);
  return out.sort((a, b) => a.name.localeCompare(b.name));
}

function prefilledInput(schema) {
  const input = {};
  for (const [key, prop] of Object.entries(schema.properties || {})) {
    if ('prefill' in prop) input[key] = prop.prefill;
    else if ((schema.required || []).includes(key) && 'default' in prop) input[key] = prop.default;
  }
  return input;
}

async function waitFor(kind, id, timeoutSecs) {
  const started = Date.now();
  for (;;) {
    const item = await api('GET', `/${kind}/${id}`);
    if (!['READY', 'RUNNING'].includes(item.status)) return item;
    if ((Date.now() - started) / 1000 > timeoutSecs) throw new Error(`${kind} ${id} timed out`);
    await sleep(5000);
  }
}

async function deploy(dir) {
  const actorJson = JSON.parse(fs.readFileSync(path.join(dir, '.actor/actor.json'), 'utf8'));
  const schema = JSON.parse(fs.readFileSync(path.join(dir, '.actor/input_schema.json'), 'utf8'));
  const name = actorJson.name;
  const actor = await api('GET', `/acts/thescrappa~${name}`);
  const latest = actor.taggedBuilds?.latest;
  const latestBuild = latest ? await api('GET', `/actor-builds/${latest.buildId}`) : null;
  const versionNumber = latestBuild?.buildNumber?.split('.').slice(0, 2).join('.') || actor.versions.at(-1).versionNumber;
  const version = actor.versions.find((v) => v.versionNumber === versionNumber);
  if (!version) throw new Error(`${name}: version ${versionNumber} not found`);
  if (!(version.envVars || []).some((e) => e.name === 'SCRAPPA_API_KEY')) {
    throw new Error(`${name}: version ${versionNumber} has no SCRAPPA_API_KEY env var, refusing to deploy`);
  }
  const files = DRY ? collectFiles(dir) : prebuiltSourceFiles(dir);
  console.log(`${name}: version ${versionNumber}, ${files.length} files, previous latest ${latest?.buildId || 'none'}`);
  if (DRY) return { name, status: 'dry-run' };

  await api('PUT', `/acts/${actor.id}/versions/${versionNumber}`, { sourceType: 'SOURCE_FILES', sourceFiles: files, buildTag: 'candidate' });
  const build = await api('POST', `/acts/${actor.id}/builds?version=${versionNumber}&tag=candidate&useCache=true`);
  const finishedBuild = await waitFor('actor-builds', build.id, 900);
  if (finishedBuild.status !== 'SUCCEEDED') throw new Error(`${name}: build ${build.id} ${finishedBuild.status}`);

  const run = await api('POST', `/acts/${actor.id}/runs?build=${encodeURIComponent(finishedBuild.buildNumber)}`, prefilledInput(schema));
  const finishedRun = await waitFor('actor-runs', run.id, 600);
  // Dataset item counts can lag behind the run; read actual items for up to 30 s.
  let itemCount = 0;
  for (let i = 0; i < 6 && itemCount === 0; i += 1) {
    await sleep(5000);
    const res = await fetch(`${API}/datasets/${finishedRun.defaultDatasetId}/items?limit=1&token=${TOKEN}`);
    if (res.ok) itemCount = (await res.json()).length;
  }
  const ok = finishedRun.status === 'SUCCEEDED' && (itemCount > 0 || ALLOW_EMPTY);
  console.log(`${name}: candidate build ${build.id} run ${run.id} ${finishedRun.status} hasItems=${itemCount > 0} ${finishedRun.statusMessage || ''}`.trim());
  if (!ok) return { name, status: 'not-promoted', build: build.id, run: run.id, previous: latest?.buildId };
  if (NO_PROMOTE) return { name, status: 'candidate-only', build: build.id, previous: latest?.buildId };

  await api('PUT', `/acts/${actor.id}`, { taggedBuilds: { latest: { buildId: build.id } } });
  const check = await api('GET', `/acts/${actor.id}`);
  const promoted = check.taggedBuilds?.latest?.buildId === build.id;
  console.log(`${name}: ${promoted ? 'PROMOTED' : 'PROMOTION FAILED'} latest=${check.taggedBuilds?.latest?.buildId} (rollback: ${latest?.buildId})`);
  return { name, status: promoted ? 'promoted' : 'promotion-failed', build: build.id, previous: latest?.buildId };
}

if (flags.has('--rollback')) {
  const [actorId, buildId] = positional;
  await api('PUT', `/acts/${actorId}`, { taggedBuilds: { latest: { buildId } } });
  console.log(`rolled back ${actorId} to ${buildId}`);
  process.exit(0);
}

const results = [];
for (const dir of positional) {
  try {
    results.push(await deploy(dir));
  } catch (error) {
    console.error(`${dir}: ERROR ${error.message}`);
    results.push({ name: dir, status: 'error', error: error.message });
  }
}
if (process.env.DEPLOY_RESULTS) fs.writeFileSync(process.env.DEPLOY_RESULTS, JSON.stringify(results, null, 2));
const failed = results.filter((r) => !['promoted', 'dry-run', 'candidate-only'].includes(r.status));
console.log(`done: ${results.length - failed.length}/${results.length} ok`);
process.exit(failed.length ? 1 : 0);

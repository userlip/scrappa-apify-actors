#!/usr/bin/env node
// Create and verify new spec-generated Actors on Apify (private until published).
//
// For each actors/<slug> directory that has specs/<slug>.json:
//   1. create the Actor privately with Store metadata, limited permissions,
//      PAY_PER_EVENT tiered pricing and a SOURCE_FILES version whose
//      SCRAPPA_API_KEY secret comes from the SCRAPPA_API_KEY env var,
//   2. build it, run the prefilled input and require at least one dataset item.
// Existing Actors are skipped (use scripts/deploy-actor.mjs for updates).
// Publishing is a separate, explicit step: --publish <slug> ... sets isPublic.
//
// Usage:
//   APIFY_TOKEN=... SCRAPPA_API_KEY=... node scripts/create-actor.mjs actors/<slug> [...] [--dry-run]
//   APIFY_TOKEN=... node scripts/create-actor.mjs --publish <slug> [...]

import fs from 'node:fs';
import path from 'node:path';
import { prebuiltSourceFiles } from './prebuilt-actor.mjs';

const API = 'https://api.apify.com/v2';
const TOKEN = process.env.APIFY_TOKEN;
const args = process.argv.slice(2);
const flags = new Set(args.filter((a) => a.startsWith('--')));
const positional = args.filter((a) => !a.startsWith('--'));
const SKIP_DIRS = new Set(['target', 'storage', 'node_modules', '.git', 'apify_storage', 'crawlee_storage', 'dist']);
const TEXT_EXT = /\.(rs|toml|lock|json|md|txt|svg|yml|yaml|js|mjs|ts|sh)$|Dockerfile$|\.dockerignore$|\.gitignore$/;
const TIERS = { FREE: 1, BRONZE: 0.8333, SILVER: 0.7333, GOLD: 0.6667, PLATINUM: 0.6667, DIAMOND: 0.6667 };

if (!TOKEN) {
  console.error('APIFY_TOKEN is required');
  process.exit(2);
}
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function api(method, p, body, { allow404 = false } = {}) {
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
    if (allow404 && res.status === 404) return null;
    if (res.status === 402 || res.status === 429 || res.status >= 500) {
      await sleep(5000 * (attempt + 1));
      continue;
    }
    const text = await res.text();
    if (!res.ok) throw new Error(`${method} ${p.split('?')[0]} -> HTTP ${res.status}: ${text.slice(0, 300)}`);
    return text ? JSON.parse(text).data : null;
  }
  throw new Error(`${method} ${p.split('?')[0]} kept failing`);
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

function roundPrice(value) {
  return Math.round(value * 1e6) / 1e6;
}

export function pricingInfo(spec, now = new Date()) {
  const base = spec.priceUsdPerResult;
  if (!(base > 0)) throw new Error(`${spec.slug}: priceUsdPerResult missing`);
  const tiers = Object.fromEntries(Object.entries(TIERS).map(([tier, factor]) => [tier, { tieredEventPriceUsd: roundPrice(base * factor) }]));
  return {
    pricingModel: 'PAY_PER_EVENT',
    startedAt: now.toISOString(),
    pricingPerEvent: {
      actorChargeEvents: {
        'apify-default-dataset-item': {
          eventTitle: 'Result',
          eventDescription: 'One result saved to the default dataset.',
          isPrimaryEvent: true,
          eventTieredPricingUsd: tiers,
        },
        'apify-actor-start': {
          eventTitle: 'Actor Start',
          eventDescription: 'Charged when the Actor starts running. Number of events charged depends on Actor memory (one event per GB, minimum one event).',
          isOneTimeEvent: true,
          eventPriceUsd: 0.00005,
        },
      },
    },
  };
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

async function create(dir) {
  const slug = path.basename(dir);
  const spec = JSON.parse(fs.readFileSync(path.join('specs', `${slug}.json`), 'utf8'));
  const actorJson = JSON.parse(fs.readFileSync(path.join(dir, '.actor/actor.json'), 'utf8'));
  const schema = JSON.parse(fs.readFileSync(path.join(dir, '.actor/input_schema.json'), 'utf8'));
  const existing = await api('GET', `/acts/thescrappa~${slug}`, undefined, { allow404: true });
  if (existing) return { slug, status: 'exists', id: existing.id };
  const files = flags.has('--dry-run') ? collectFiles(dir) : prebuiltSourceFiles(dir);
  const store = spec.storeCopy || {};
  const body = {
    name: slug,
    title: spec.title,
    description: spec.description,
    seoTitle: store.seoTitle || spec.seoTitle,
    seoDescription: store.seoDescription || spec.seoDescription,
    categories: spec.categories || store.categories,
    isPublic: false,
    actorPermissionLevel: 'LIMITED_PERMISSIONS',
    defaultRunOptions: { build: 'latest', memoryMbytes: actorJson.defaultMemoryMbytes || 128, timeoutSecs: actorJson.defaultRunOptions?.timeoutSecs || spec.timeoutSecs || 300 },
    versions: [{
      versionNumber: '1.0',
      sourceType: 'SOURCE_FILES',
      buildTag: 'latest',
      sourceFiles: files,
      envVars: [{ name: 'SCRAPPA_API_KEY', value: process.env.SCRAPPA_API_KEY, isSecret: true }],
    }],
  };
  for (const key of ['title', 'description', 'seoTitle', 'seoDescription', 'categories']) {
    if (!body[key] || (Array.isArray(body[key]) && body[key].length === 0)) throw new Error(`${slug}: spec is missing ${key}`);
  }
  console.log(`${slug}: ${files.length} files, title "${body.title}", price ${spec.priceUsdPerResult}/result`);
  if (flags.has('--dry-run')) return { slug, status: 'dry-run' };
  if (!process.env.SCRAPPA_API_KEY) throw new Error('SCRAPPA_API_KEY env var is required to create Actors');

  const actor = await api('POST', '/acts', body);
  // Apify rejects pricing on create; set it right after, while the Actor is still private.
  await api('PUT', `/acts/${actor.id}`, { pricingInfos: [pricingInfo(spec)] });
  const build = await api('POST', `/acts/${actor.id}/builds?version=1.0&tag=latest`);
  const finishedBuild = await waitFor('actor-builds', build.id, 900);
  if (finishedBuild.status !== 'SUCCEEDED') return { slug, status: 'build-failed', id: actor.id, build: build.id };
  const run = await api('POST', `/acts/${actor.id}/runs`, prefilledInput(schema));
  const finishedRun = await waitFor('actor-runs', run.id, 600);
  let hasItems = false;
  for (let i = 0; i < 6 && !hasItems; i += 1) {
    await sleep(5000);
    const res = await fetch(`${API}/datasets/${finishedRun.defaultDatasetId}/items?limit=1&token=${TOKEN}`);
    if (res.ok) hasItems = (await res.json()).length > 0;
  }
  const ok = finishedRun.status === 'SUCCEEDED' && hasItems;
  console.log(`${slug}: created ${actor.id}, build ${finishedBuild.buildNumber}, run ${run.id} ${finishedRun.status} hasItems=${hasItems} ${finishedRun.statusMessage || ''}`.trim());
  return { slug, status: ok ? 'verified' : 'run-failed', id: actor.id, run: run.id };
}

async function publish(slug) {
  const actor = await api('GET', `/acts/thescrappa~${slug}`);
  const updated = await api('PUT', `/acts/${actor.id}`, { isPublic: true });
  console.log(`${slug}: isPublic=${updated.isPublic}`);
  return { slug, status: updated.isPublic ? 'published' : 'publish-failed', id: actor.id };
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const results = [];
  for (const target of positional) {
    try {
      results.push(flags.has('--publish') ? await publish(target) : await create(target));
    } catch (error) {
      console.error(`${target}: ERROR ${error.message}`);
      results.push({ slug: target, status: 'error', error: error.message });
    }
  }
  if (process.env.DEPLOY_RESULTS) fs.writeFileSync(process.env.DEPLOY_RESULTS, JSON.stringify(results, null, 2));
  const bad = results.filter((r) => !['verified', 'exists', 'dry-run', 'published'].includes(r.status));
  console.log(`done: ${results.length - bad.length}/${results.length} ok`);
  process.exit(bad.length ? 1 : 0);
}

#!/usr/bin/env node

import { readdir, readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const specsDirectory = path.join(root, 'specs');
const actorsDirectory = path.join(root, 'actors');
const bannedPhrases = [
  'returned for this result',
  'Provide the fields listed below',
  'Results can include',
  'Save and export Apify datasets',
  'Search terms',
  'collects structured data from',
  'for Campaign Research',
  'for Creator Research',
  'for Audience Research',
  'for Lead Research',
  'for Market Research',
  'for Video Analysis',
  'for Hiring Teams',
  'Each entry maps its',
  'The batch input associated with this result',
];
const requiredSections = [
  'What data can you extract?',
  'Use cases',
  'How to use',
  'Input',
  'Output example',
  'Pricing',
  'FAQ',
  'Related Scrappa Actors',
];

const failures = [];
const specs = (await readdir(specsDirectory))
  .filter((name) => name.endsWith('.json'))
  .sort();
const knownActors = new Set(await readdir(actorsDirectory));

function fail(slug, message) {
  failures.push(`${slug}: ${message}`);
}

function pointer(value, jsonPointer) {
  if (jsonPointer === '') return value;
  return jsonPointer.split('/').slice(1).reduce((current, token) => {
    const key = token.replaceAll('~1', '/').replaceAll('~0', '~');
    return current?.[key];
  }, value);
}

function resultRows(spec) {
  if (spec.mode === 'single') return [spec.fixture];
  for (const jsonPointer of [spec.resultPointer, ...(spec.fallbackResultPointers ?? [])]) {
    const rows = pointer(spec.fixture, jsonPointer);
    if (Array.isArray(rows)) return rows;
  }
  return [];
}

function addFlattenedKeys(row, pointers) {
  const keys = new Set(Object.keys(row && typeof row === 'object' && !Array.isArray(row) ? row : {}));
  for (const [name, jsonPointer] of Object.entries(pointers ?? {})) {
    if (pointer(row, jsonPointer) !== undefined) keys.add(name);
  }
  return keys;
}

function containsFixedDate(value) {
  if (typeof value === 'string') return /\b20\d{2}-\d{2}-\d{2}\b/.test(value);
  if (Array.isArray(value)) return value.some(containsFixedDate);
  if (value && typeof value === 'object') return Object.values(value).some(containsFixedDate);
  return false;
}

function containsPlaceholder(value, location = '$') {
  if (Array.isArray(value)) {
    if (value.length === 0) return location;
    for (let index = 0; index < value.length; index += 1) {
      const found = containsPlaceholder(value[index], `${location}[${index}]`);
      if (found) return found;
    }
    return undefined;
  }
  if (value && typeof value === 'object') {
    const keys = Object.keys(value);
    if (keys.length === 0) return location;
    for (const key of keys) {
      const found = containsPlaceholder(value[key], `${location}.${key}`);
      if (found) return found;
    }
  }
  if (typeof value === 'string' && /\b(?:example|synthetic|sample)\b|\b42\b/i.test(value)) return location;
  return undefined;
}

function parseOutputExample(readme, slug) {
  const section = readme.split('## Output example')[1]?.split('\n## ')[0] ?? '';
  const match = section.match(/```json\n([\s\S]*?)\n```/);
  if (!match) {
    fail(slug, 'README must include one fenced JSON output record');
    return undefined;
  }
  try {
    return JSON.parse(match[1]);
  } catch {
    fail(slug, 'README output example is not valid JSON');
    return undefined;
  }
}

for (const filename of specs) {
  const spec = JSON.parse(await readFile(path.join(specsDirectory, filename), 'utf8'));
  const slug = spec.slug;
  const actorDirectory = path.join(actorsDirectory, slug);
  const readmePath = path.join(actorDirectory, '.actor/README.md');
  const actorPath = path.join(actorDirectory, '.actor/actor.json');
  const [readme, actorFile] = await Promise.all([
    readFile(readmePath, 'utf8'),
    readFile(actorPath, 'utf8'),
  ]);
  const actor = JSON.parse(actorFile);
  const copy = spec.storeCopy ?? {};

  for (const key of ['title', 'description', 'seoTitle', 'seoDescription', 'categories']) {
    if (!spec[key] && !copy[key]) fail(slug, `missing ${key}`);
  }
  if (spec.description.length > 300) fail(slug, 'Store description exceeds 300 characters');
  if (copy.seoTitle?.length > 60) fail(slug, 'SEO title exceeds 60 characters');
  if (!copy.seoTitle?.includes(' | ')) fail(slug, 'SEO title must include a benefit after " | "');
  if (copy.seoDescription?.length < 140 || copy.seoDescription?.length > 155) {
    fail(slug, 'SEO description must be 140 to 155 characters');
  }
  if (!Array.isArray(spec.categories) || spec.categories.length < 1 || spec.categories.length > 3) {
    fail(slug, 'categories must contain 1 to 3 values');
  }
  const metadata = [spec.title, spec.description, copy.seoTitle, copy.seoDescription];
  for (const value of metadata) {
    if (typeof value !== 'string') continue;
    if (value.includes('—')) fail(slug, 'Store metadata contains an em dash');
    if (/\bRust\b|Scrappa API endpoint/i.test(value)) fail(slug, 'Store metadata contains implementation details');
  }
  const endpoints = [
    spec.endpoint,
    ...(spec.endpointSelectors ?? []).flatMap((selector) => selector.choices.map((choice) => choice.endpoint)),
    ...(spec.endpointSelector?.choices ?? []).map((choice) => choice.endpoint),
  ].filter(Boolean);
  for (const endpoint of endpoints) {
    if (metadata.some((value) => typeof value === 'string' && value.includes(endpoint))) {
      fail(slug, 'Store metadata contains an endpoint path');
    }
  }
  if (!Array.isArray(copy.useCases) || copy.useCases.length < 3 || copy.useCases.length > 6) {
    fail(slug, 'Store copy must have 3 to 6 use cases');
  }
  if (!Array.isArray(copy.howToUse) || copy.howToUse.length < 3) fail(slug, 'Store copy needs at least 3 usage steps');
  if (!Array.isArray(copy.faqs) || copy.faqs.length < 1 || copy.faqs.length > 3) {
    fail(slug, 'Store copy must have 1 to 3 Actor-specific FAQs');
  }
  if (!Array.isArray(copy.relatedActors) || copy.relatedActors.length < 3 || copy.relatedActors.length > 6) {
    fail(slug, 'Store copy must link 3 to 6 related Actors');
  } else {
    for (const related of copy.relatedActors) {
      if (!knownActors.has(related.slug)) fail(slug, `related Actor ${related.slug} does not exist`);
    }
  }
  if (!Array.isArray(copy.fields) || copy.fields.length === 0) {
    fail(slug, 'Store copy needs a complete output field table');
  } else {
    const documented = new Set(copy.fields.map((field) => field.name));
    const rows = resultRows(spec);
    if (rows.length === 0) fail(slug, 'fixture has no output record for the README example');
    const expected = new Set();
    for (const row of rows) {
      for (const name of addFlattenedKeys(row, spec.flattenPointers)) expected.add(name);
    }
    expected.add(spec.enrichment.field);
    expected.add('scraped_at');
    for (const name of expected) {
      if (!documented.has(name)) fail(slug, `field table omits output field ${name}`);
    }
    for (const field of copy.fields) {
      if (!field.name || !field.type || !field.description || field.description.length < 15) {
        fail(slug, `field ${field.name ?? '(unnamed)'} needs a type and specific description`);
      }
      if (!/^(String|Number|Integer|Boolean|Object|Array<[^>]+>)( or null)?$/.test(field.type)) {
        fail(slug, `field ${field.name} has an unsupported data type label`);
      }
    }
  }
  if (containsFixedDate(spec.prefill)) fail(slug, 'prefill contains a fixed date instead of a relative default');

  if (readme.split('\n')[0] !== `# ${spec.title}`) fail(slug, 'README H1 must match the Store title');
  const sectionPositions = requiredSections.map((section) => readme.indexOf(`## ${section}`));
  if (sectionPositions.some((position, index) => position < 0 || (index > 0 && position <= sectionPositions[index - 1]))) {
    fail(slug, 'README sections do not follow the required Store structure');
  }
  const h1Position = readme.indexOf('\n## ');
  const intro = readme.slice(readme.indexOf('\n\n') + 2, h1Position);
  const introSentences = intro.split(/[.!?](?:\s|$)/).filter((sentence) => sentence.trim()).length;
  if (introSentences < 2 || introSentences > 3) fail(slug, 'README intro must have 2 to 3 sentences');
  const apiUrl = `https://api.apify.com/v2/acts/thescrappa~${slug}/runs`;
  if (!readme.includes(`POST request to \`${apiUrl}\``)) fail(slug, 'FAQ must include the real Apify run API URL');
  if (readme.includes('—')) fail(slug, 'README contains an em dash');
  const lowerReadme = readme.toLowerCase();
  for (const phrase of bannedPhrases) {
    if (lowerReadme.includes(phrase.toLowerCase())
      || metadata.some((value) => typeof value === 'string' && value.toLowerCase().includes(phrase.toLowerCase()))) {
      fail(slug, `Store copy contains banned copy: ${phrase}`);
    }
  }
  for (const endpoint of endpoints) {
    if (readme.includes(endpoint)) fail(slug, 'README contains an internal endpoint path');
  }

  const output = parseOutputExample(readme, slug);
  if (output) {
    const placeholder = containsPlaceholder(output);
    if (placeholder) fail(slug, `output example contains an empty or placeholder value at ${placeholder}`);
    const names = new Set(Object.keys(output));
    for (const field of expectedOutputKeys(spec)) {
      if (!names.has(field)) fail(slug, `output example is missing ${field}`);
    }
  }

  const priceLine = `$${(spec.priceUsdPerResult * 1000).toFixed(2)} per 1,000 results`;
  if (!readme.includes(priceLine)) fail(slug, `README must state the price ${priceLine}`);
  if (actor.title !== spec.title || actor.description !== spec.description) {
    fail(slug, 'Apify actor metadata differs from its hand-written Store metadata');
  }
  if (JSON.stringify(actor.categories) !== JSON.stringify(spec.categories)) {
    fail(slug, 'Apify actor categories differ from the spec');
  }

  for (const field of spec.tableFields ?? []) {
    const words = field.name.includes('_') || /[a-z][A-Z]/.test(field.name);
    if (!field.label || field.label.includes('_') || /[a-z][A-Z]/.test(field.label) || (words && !field.label.includes(' '))) {
      fail(slug, `table view label for ${field.name} is not a human-readable label`);
    }
  }
  if (actor.defaultMemoryMbytes !== 128) fail(slug, 'Actor must use the 128 MB default');
  for (const related of copy.relatedActors ?? []) {
    if (!knownActors.has(related.slug)) continue;
    const relatedActor = JSON.parse(await readFile(
      path.join(actorsDirectory, related.slug, '.actor/actor.json'),
      'utf8',
    ));
    if (relatedActor.title !== related.title) fail(slug, `related Actor title for ${related.slug} is incorrect`);
  }
}

function expectedOutputKeys(spec) {
  const rows = resultRows(spec);
  const keys = rows.length > 0
    ? addFlattenedKeys(rows[0], spec.flattenPointers)
    : new Set();
  keys.add(spec.enrichment.field);
  keys.add('scraped_at');
  return keys;
}

if (failures.length > 0) {
  console.error(failures.join('\n'));
  process.exitCode = 1;
} else {
  console.log(`Store copy validation passed (${specs.length} generated Actors).`);
}

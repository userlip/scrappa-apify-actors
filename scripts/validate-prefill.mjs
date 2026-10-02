#!/usr/bin/env node
// Validate every Actor's prefilled input against its own input schema, the way
// Apify does before a run starts (and before the daily Store QA run). Local
// tests read INPUT directly and never see these errors.
//
// Usage: node scripts/validate-prefill.mjs [actors/<slug> ...]   (default: all generated Actors)

import fs from 'node:fs';
import path from 'node:path';

function prefilledInput(schema) {
  const input = {};
  for (const [key, prop] of Object.entries(schema.properties || {})) {
    if ('prefill' in prop) input[key] = structuredClone(prop.prefill);
    else if ('default' in prop) input[key] = structuredClone(prop.default);
  }
  return input;
}

function typeOf(value) {
  if (value === null) return 'null';
  if (Array.isArray(value)) return 'array';
  if (Number.isInteger(value)) return 'integer';
  return typeof value;
}

function typeMatches(expected, value) {
  const actual = typeOf(value);
  const list = Array.isArray(expected) ? expected : [expected];
  return list.some((t) => t === actual || (t === 'number' && actual === 'integer'));
}

export function validate(schema, value, at, errors) {
  if (schema.type && !typeMatches(schema.type, value)) {
    errors.push(`${at} must be ${schema.type}, got ${typeOf(value)}`);
    return;
  }
  if (schema.enum && !schema.enum.includes(value)) errors.push(`${at} must be one of ${JSON.stringify(schema.enum)}`);
  if (typeof value === 'string') {
    if (schema.pattern && !new RegExp(schema.pattern).test(value)) errors.push(`${at} must match ${schema.pattern} (got ${JSON.stringify(value)})`);
    if (schema.minLength !== undefined && value.length < schema.minLength) errors.push(`${at} is shorter than ${schema.minLength}`);
    if (schema.maxLength !== undefined && value.length > schema.maxLength) errors.push(`${at} is longer than ${schema.maxLength}`);
  }
  if (typeof value === 'number') {
    if (schema.minimum !== undefined && value < schema.minimum) errors.push(`${at} is below ${schema.minimum}`);
    if (schema.maximum !== undefined && value > schema.maximum) errors.push(`${at} is above ${schema.maximum}`);
  }
  if (Array.isArray(value)) {
    if (schema.minItems !== undefined && value.length < schema.minItems) errors.push(`${at} needs at least ${schema.minItems} items`);
    if (schema.maxItems !== undefined && value.length > schema.maxItems) errors.push(`${at} allows at most ${schema.maxItems} items`);
    if (schema.items) value.forEach((item, i) => validate(schema.items, item, `${at}.${i}`, errors));
  }
  if (value && typeof value === 'object' && !Array.isArray(value)) {
    const props = schema.properties || {};
    // Apify fills in defaults of nested object properties before validating.
    for (const [key, prop] of Object.entries(props)) {
      if (!(key in value) && 'default' in prop) value[key] = structuredClone(prop.default);
    }
    for (const key of schema.required || []) {
      if (!(key in value)) errors.push(`${at}.${key} is required`);
    }
    for (const [key, prop] of Object.entries(props)) {
      if (key in value) validate(prop, value[key], `${at}.${key}`, errors);
    }
  }
}

function checkActor(dir) {
  const schema = JSON.parse(fs.readFileSync(path.join(dir, '.actor/input_schema.json'), 'utf8'));
  const input = prefilledInput(schema);
  const errors = [];
  validate({ ...schema, type: 'object' }, input, 'input', errors);
  // Every property's own default must satisfy its own constraints too.
  const walkDefaults = (props, at) => {
    for (const [key, prop] of Object.entries(props || {})) {
      if ('default' in prop) validate(prop, structuredClone(prop.default), `${at}.${key} (default)`, errors);
      if (prop.items?.properties) walkDefaults(prop.items.properties, `${at}.${key}[]`);
      if (prop.properties) walkDefaults(prop.properties, `${at}.${key}`);
    }
  };
  walkDefaults(schema.properties, 'input');
  return [...new Set(errors)];
}

if (import.meta.url === `file://${process.argv[1]}`) {
  let dirs = process.argv.slice(2);
  if (dirs.length === 0) {
    dirs = fs.readdirSync('specs').filter((f) => f.endsWith('.json')).map((f) => path.join('actors', f.slice(0, -5)));
  }
  let failed = 0;
  for (const dir of dirs) {
    const errors = checkActor(dir);
    if (errors.length) {
      failed += 1;
      console.log(`${path.basename(dir)}:\n  - ${errors.join('\n  - ')}`);
    }
  }
  console.log(`Prefill validation: ${dirs.length - failed}/${dirs.length} Actors valid.`);
  process.exit(failed ? 1 : 0);
}

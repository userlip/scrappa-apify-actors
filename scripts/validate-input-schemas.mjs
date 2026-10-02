#!/usr/bin/env node
// Validate every Actor input schema with Apify's own validator, the same check
// an Apify build runs. Catches OpenAPI-only keywords (format, integer enums in
// batch items, wrong editors) before a build fails on the platform.
//
// Requires: npm install --no-save @apify/input_schema ajv
// Usage:    node scripts/validate-input-schemas.mjs

import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { validateInputSchema } = require('@apify/input_schema');
const Ajv2019 = require('ajv/dist/2019').default;

let checked = 0;
let failed = 0;
for (const dir of fs.readdirSync('actors').sort()) {
  const file = path.join('actors', dir, '.actor/input_schema.json');
  if (!fs.existsSync(file)) continue;
  checked += 1;
  try {
    validateInputSchema(new Ajv2019({ strict: false }), JSON.parse(fs.readFileSync(file, 'utf8')));
  } catch (error) {
    failed += 1;
    console.log(`${dir}: ${error.message}`);
  }
}
console.log(`Apify input schema validation: ${checked - failed}/${checked} valid.`);
process.exit(failed ? 1 : 0);

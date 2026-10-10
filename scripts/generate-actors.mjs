#!/usr/bin/env node

import { readdir, readFile, mkdir, rm, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const specsDirectory = path.join(root, 'specs');
const templateDirectory = path.join(root, 'templates/spec-driven-actor');
const actorsDirectory = path.join(root, 'actors');
const checkOnly = process.argv.includes('--check');
const onlySlug = process.argv.find((argument) => argument.startsWith('--only='))?.slice('--only='.length);

function json(value) {
  return `${JSON.stringify(value, null, 2)}\n`;
}

function escapeMarkdown(value) {
  return String(value ?? '')
    .replaceAll('\r', '')
    .replaceAll('\n', ' ')
    .split(/(`[^`\n]*`)/g)
    .map((part) => part.startsWith('`') && part.endsWith('`')
      ? `\`${escapeCode(part.slice(1, -1))}\``
      : part.replaceAll('\\', '\\\\').replace(/[|*_\[\]()#<>]/g, '\\$&'))
    .join('');
}

function escapeCode(value) {
  return String(value ?? '')
    .replaceAll('\\', '\\\\')
    .replaceAll('`', '\\`')
    .replaceAll('|', '\\|')
    .replaceAll('\r', '')
    .replaceAll('\n', ' ');
}

function endpointSelector(spec) {
  const selectors = spec.endpointSelectors ?? (spec.endpointSelector ? [spec.endpointSelector] : []);
  if (selectors.length > 1) {
    throw new Error(`${spec.slug} must define no more than one endpoint selector`);
  }
  return selectors[0];
}

function conditionalDescription(parameter, spec) {
  const fragments = parameter.requiredForEndpoints ?? [];
  const available = parameter.availableForEndpoints ?? [];
  const labels = (endpointSelector(spec)?.choices ?? [])
    .filter((choice) => fragments.some((fragment) => choice.endpoint.includes(fragment)))
    .map((choice) => choice.label);
  if (labels.length > 0) return `Required for ${labels.join(' or ')} searches.`;
  if (fragments.length > 0) return 'Required for the endpoint selected by the trip type.';

  const availableLabels = (endpointSelector(spec)?.choices ?? [])
    .filter((choice) => available.some((fragment) => choice.endpoint.includes(fragment)))
    .map((choice) => choice.label);
  return availableLabels.length > 0
    ? `Used only for ${availableLabels.join(' or ')} searches.`
    : '';
}

// Scrappa's OpenAPI comes from PHP and can use delimited patterns such as
// `#^https?://...#i`. Apify validates with JavaScript regexes without flags, so
// strip the delimiters and drop patterns that rely on flags.
export function jsPattern(pattern) {
  if (typeof pattern !== 'string') return pattern;
  const delimited = /^([#\/~@%])([\s\S]*)\1([a-z]*)$/.exec(pattern);
  if (!delimited) return pattern;
  return delimited[3] ? undefined : delimited[2];
}

function propertySchema(parameter, spec) {
  const descriptions = [
    parameter.schema?.description,
    conditionalDescription(parameter, spec),
    ...(spec.relativeDateDefaults ?? [])
      .filter((rule) => rule.input === parameter.input)
      .map((rule) => rule.description),
  ].filter(Boolean);
  const schema = { ...parameter.schema };
  if ('pattern' in schema) {
    const pattern = jsPattern(schema.pattern);
    if (pattern === undefined) delete schema.pattern;
    else schema.pattern = pattern;
  }
  return apifyProperty({
    title: parameter.title ?? parameter.input,
    ...schema,
    description: descriptions.join(' '),
    editor: parameter.editor ?? editorFor(parameter.schema?.type),
  });
}

// Apify's input schema is stricter than OpenAPI: no `format`, numeric fields
// use the number editor and cannot be enums inside batch items, and string
// enums need the select editor. Applied to every generated property; a
// property that is valid at the top level stays valid inside batch items.
export function apifyProperty(property) {
  const result = { ...property };
  delete result.format;
  if (result.type === 'integer' || result.type === 'number') {
    if (result.enum) {
      const values = result.enum.join(', ');
      result.description = [result.description, `Allowed values: ${values}.`].filter(Boolean).join(' ');
      if (result.minimum === undefined) result.minimum = Math.min(...result.enum);
      if (result.maximum === undefined) result.maximum = Math.max(...result.enum);
      delete result.enum;
      delete result.enumTitles;
    }
    if (!['number', 'hidden'].includes(result.editor)) result.editor = 'number';
  }
  if (result.type === 'string' && result.enum && !['select', 'hidden'].includes(result.editor)) result.editor = 'select';
  return result;
}

function editorFor(type) {
  if (type === 'boolean') return 'checkbox';
  if (type === 'integer' || type === 'number') return 'number';
  if (type === 'string') return 'textfield';
  return 'json';
}

function inputSchema(spec) {
  const parameterByInput = new Map(spec.parameters.map((parameter) => [parameter.input, parameter]));
  const batchProperties = {};
  for (const parameter of spec.parameters) {
    if (parameter.location === 'path') continue;
    batchProperties[parameter.input] = propertySchema(parameter, spec);
  }
  const batchPrimary = parameterByInput.get(spec.batch.valueField);
  const batchItemProperties = {
    [spec.batch.valueField]: batchPrimary
      ? propertySchema(batchPrimary, spec)
      : {
          title: spec.batch.valueField,
          type: 'string',
          minLength: 1,
          description: spec.batch.valueDescription ?? 'Value used for this batch entry.',
          editor: 'textfield',
        },
  };
  for (const [name, schema] of Object.entries(batchProperties)) {
    if (name !== spec.batch.valueField) batchItemProperties[name] = schema;
  }

  const itemRequired = [...new Set([spec.batch.valueField, ...(spec.batch.itemRequired ?? [])])];
  const properties = {
    [spec.batch.field]: {
      title: spec.batch.title,
      type: 'array',
      description: spec.batch.description,
      editor: 'json',
      minItems: 1,
      maxItems: 100,
      prefill: spec.prefill[spec.batch.field],
      items: {
        type: 'object',
        required: itemRequired,
        properties: batchItemProperties,
      },
    },
  };

  for (const parameter of spec.parameters) {
    if (parameter.input === spec.batch.valueField || parameter.location === 'path') continue;
    properties[parameter.input] = propertySchema(parameter, spec);
  }

  const selector = endpointSelector(spec);
  if (selector) {
    properties[selector.input] = {
        title: selector.title,
        type: 'string',
        description: selector.description,
        editor: 'select',
        enum: selector.choices.map((choice) => choice.value),
        enumTitles: selector.choices.map((choice) => choice.label),
        default: selector.default,
      };
  }

  properties.maxResults = {
    title: 'Maximum Results',
    type: 'integer',
    description: 'Maximum dataset items to save across this entire run.',
    editor: 'number',
    minimum: 1,
    maximum: spec.maxResults.hardLimit,
    default: spec.maxResults.default,
  };

  if (spec.pagination) {
    properties.maxPages = {
      title: 'Maximum Pages',
      type: 'integer',
      description: 'Maximum pages to request for each batch entry.',
      editor: 'number',
      minimum: 1,
      maximum: spec.pagination.maxPages,
      default: spec.defaultMaxPages,
    };
  }

  // Every top-level prefill value from the spec must reach the Apify form, not
  // only the batch list; Apify's daily QA run uses exactly these values.
  for (const [name, value] of Object.entries(spec.prefill ?? {})) {
    if (name !== spec.batch.field && properties[name]) properties[name].prefill = value;
  }

  const required = [spec.batch.field];
  const itemFields = new Set(itemRequired);
  for (const parameter of spec.parameters) {
    if (
      parameter.required
      && parameter.location !== 'path'
      && parameter.input !== spec.batch.valueField
      && !itemFields.has(parameter.input)
      && !(parameter.requiredForEndpoints?.length)
    ) required.push(parameter.input);
  }
  if (selector && !selector.default) required.push(selector.input);

  return {
    title: spec.title,
    description: spec.description,
    type: 'object',
    schemaVersion: 1,
    required: [...new Set(required)],
    properties,
  };
}

// Apify requires an output schema to publish an Actor. Results always live in
// the default dataset; the overview view matches the dataset table in actor.json.
function outputSchema(spec) {
  return {
    actorOutputSchemaVersion: 1,
    title: `${spec.title} output`,
    description: `Results saved by ${spec.title}, one dataset item per result.`,
    properties: {
      results: {
        type: 'string',
        title: 'Results',
        description: 'All results from this run in the default dataset.',
        template: '{{links.apiDefaultDatasetUrl}}/items',
      },
    },
  };
}

function actorJson(spec) {
  const fields = spec.tableFields;
  const display = {};
  for (const field of fields) {
    display[field.name] = {
      label: field.label,
      format: field.format,
    };
  }

  return {
    actorSpecification: 1,
    minMemoryMbytes: 128,
    maxMemoryMbytes: 128,
    name: spec.slug,
    title: spec.title,
    description: spec.description,
    categories: spec.categories,
    version: '1.0',
    buildTag: 'latest',
    input: './input_schema.json',
    output: './output_schema.json',
    dockerfile: './Dockerfile',
    environmentVariables: {
      SCRAPPA_API_KEY: '@SCRAPPA_API_KEY',
    },
    defaultMemoryMbytes: 128,
    defaultRunOptions: {
      timeoutSecs: spec.timeoutSecs ?? 300,
    },
    storages: {
      dataset: {
        actorSpecification: 1,
        views: {
          results: {
            title: 'Results',
            transformation: {
              fields: fields.map((field) => field.name),
            },
            display: {
              component: 'table',
              properties: display,
            },
          },
        },
      },
    },
  };
}

function pointer(value, jsonPointer) {
  if (jsonPointer === '') return value;
  return jsonPointer.split('/').slice(1).reduce((current, token) => {
    const key = token.replaceAll('~1', '/').replaceAll('~0', '~');
    return current?.[key];
  }, value);
}

function flattenExample(item, flattenPointers) {
  for (const [name, jsonPointer] of Object.entries(flattenPointers ?? {})) {
    const value = pointer(item, jsonPointer);
    if (value !== undefined) item[name] = value;
  }
  return item;
}

function fencedJson(value) {
  const contents = JSON.stringify(value, null, 2);
  if (contents.includes('```')) {
    throw new Error('An example contains triple backticks and cannot be safely fenced in Markdown.');
  }
  return `\`\`\`json\n${contents}\n\`\`\``;
}

function exampleItem(spec) {
  const fixture = spec.fixture;
  let item = fixture;
  if (spec.mode === 'list') {
    const pointers = [spec.resultPointer, ...(spec.fallbackResultPointers ?? [])];
    const rows = pointers.map((resultPointer) => pointer(fixture, resultPointer)).find(Array.isArray);
    item = rows?.[0];
  }
  if (item === undefined || item === null) {
    throw new Error(`${spec.slug} has no fixture result for its README output example`);
  }
  item = item && typeof item === 'object' && !Array.isArray(item) ? structuredClone(item) : { value: item };
  flattenExample(item, spec.flattenPointers);
  item[spec.enrichment.field] = spec.batch.enrichBatchItem
    ? structuredClone(spec.prefill[spec.batch.field][0])
    : spec.prefill[spec.batch.field][0][spec.batch.valueField];
  item.scraped_at = '2026-10-01T12:00:00Z';
  return item;
}

function schemaType(schema) {
  if (schema.type === 'array') {
    const itemType = schema.items?.type;
    return itemType ? `Array<${itemType}>` : 'Array';
  }
  return schema.type ?? 'Object';
}

function inputTable(spec) {
  const schema = inputSchema(spec);
  const rows = [];
  const required = new Set(schema.required);
  for (const [name, property] of Object.entries(schema.properties)) {
    const conditional = spec.parameters
      .filter((parameter) => parameter.input === name)
      .some((parameter) => conditionalDescription(parameter, spec));
    rows.push({
      name,
      type: schemaType(property),
      required: required.has(name) ? 'Yes' : conditional ? 'Conditional' : 'No',
      description: property.description ?? '',
    });
    if (name !== spec.batch.field) continue;
    const itemSchema = property.items;
    const itemRequired = new Set(itemSchema.required ?? []);
    for (const [itemName, itemProperty] of Object.entries(itemSchema.properties ?? {})) {
      const parameter = spec.parameters.find((candidate) => candidate.input === itemName);
      const conditional = parameter ? conditionalDescription(parameter, spec) : '';
      rows.push({
        name: `${name}[].${itemName}`,
        type: schemaType(itemProperty),
        required: itemRequired.has(itemName) ? 'Yes per entry' : conditional ? 'Conditional' : 'No',
        description: itemProperty.description ?? '',
      });
    }
  }
  return rows;
}

function readme(spec) {
  const copy = spec.storeCopy;
  const fields = copy.fields
    .map((field) => `| \`${escapeCode(field.name)}\` | ${escapeMarkdown(field.type)} | ${escapeMarkdown(field.description)} |`)
    .join('\n');
  const useCases = copy.useCases.map((useCase) => `- ${escapeMarkdown(useCase)}`).join('\n');
  const inputs = inputTable(spec)
    .map((field) => `| \`${escapeCode(field.name)}\` | ${escapeMarkdown(field.type)} | ${escapeMarkdown(field.required)} | ${escapeMarkdown(field.description)} |`)
    .join('\n');
  const related = copy.relatedActors
    .map((actor) => `- [${escapeMarkdown(actor.title)}](https://apify.com/thescrappa/${actor.slug})`)
    .join('\n');
  const howToUse = copy.howToUse
    .map((step, index) => `${index + 1}. ${escapeMarkdown(step)}`)
    .join('\n');
  const actorFaqs = copy.faqs
    .map((faq) => `### ${escapeMarkdown(faq.question)}\n\n${escapeMarkdown(faq.answer)}`)
    .join('\n\n');
  const inputExample = fencedJson(spec.prefill);
  const outputExample = fencedJson(exampleItem(spec));
  const price = `$${(spec.priceUsdPerResult * 1000).toFixed(2)} per 1,000 results`;
  const pagination = spec.pagination
    ? 'Pagination follows the source response. Set **maxPages** per batch entry and **maxResults** across the run.'
    : 'Set **maxResults** to cap the number of dataset items saved in one run.';
  const apiUrl = `https://api.apify.com/v2/acts/thescrappa~${spec.slug}/runs`;

  return `# ${escapeMarkdown(spec.title)}\n\n${escapeMarkdown(copy.intro)}\n\n## What data can you extract?\n\n| Field | Type | Description |\n| --- | --- | --- |\n${fields}\n\n## Use cases\n\n${useCases}\n\n## How to use\n\n${howToUse}\n\n${inputExample}\n\n## Input\n\n| Field | Type | Required | Description |\n| --- | --- | --- | --- |\n${inputs}\n\n## Output example\n\n${outputExample}\n\n## Pricing\n\n${price}. Apify saves one dataset item per result and applies the Actor’s per-result price to saved items.\n\n${pagination}\n\n## FAQ\n\n### Is scraping this information legal?\n\nRules depend on the source, location, data type, and intended use. Check applicable laws and source terms, and make sure your workflow follows privacy and data protection requirements.\n\n### What limits apply?\n\nSubmit up to 100 batch entries per run. Use **maxResults** to cap saved items${spec.pagination ? ' and **maxPages** to limit pages for each entry' : ''}. Results also depend on source availability and your Scrappa API plan.\n\n### Can I start runs through the Apify API?\n\nYes. Send a POST request to \`${apiUrl}\` with your Actor input, or use an Apify client library. Read the output from the run dataset.\n\n### Can I connect the results to other tools?\n\nYes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.\n\n### What happens when one input fails?\n\nThe Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.\n\n${actorFaqs}\n\n## Related Scrappa Actors\n\n${related}\n`;
}

async function templateFiles() {
  const result = new Map();
  async function walk(relativeDirectory) {
    const absoluteDirectory = path.join(templateDirectory, relativeDirectory);
    for (const entry of await readdir(absoluteDirectory, { withFileTypes: true })) {
      const relative = path.posix.join(relativeDirectory.replaceAll(path.sep, '/'), entry.name);
      if (entry.isDirectory()) await walk(relative);
      else result.set(relative, await readFile(path.join(templateDirectory, relative)));
    }
  }
  await walk('');
  return result;
}

async function render(spec, sourceFiles) {
  const files = new Map(sourceFiles);
  for (const relative of ['Cargo.toml', 'Cargo.lock', '.actor/Dockerfile']) {
    const contents = files.get(relative)?.toString('utf8');
    if (!contents) throw new Error(`Template is missing ${relative}`);
    files.set(
      relative,
      Buffer.from(contents.replaceAll('scrappa-spec-actor', spec.slug)),
    );
  }
  files.set('spec.json', Buffer.from(json(spec)));
  files.set('input-prefill.json', Buffer.from(json(spec.prefill)));
  files.set('fixtures/response.json', Buffer.from(json(spec.fixture)));
  files.set('.actor/actor.json', Buffer.from(json(actorJson(spec))));
  files.set('.actor/input_schema.json', Buffer.from(json(inputSchema(spec))));
  files.set('.actor/output_schema.json', Buffer.from(json(outputSchema(spec))));
  files.set('.actor/README.md', Buffer.from(readme(spec)));
  return files;
}

async function listFiles(directory, relative = '') {
  const result = [];
  for (const entry of await readdir(path.join(directory, relative), { withFileTypes: true })) {
    if (entry.isDirectory() && entry.name === 'target') continue;
    const item = path.posix.join(relative.replaceAll(path.sep, '/'), entry.name);
    if (entry.isDirectory()) result.push(...await listFiles(directory, item));
    else result.push(item);
  }
  return result;
}

async function writeOrCheck(slug, files) {
  const actorDirectory = path.join(actorsDirectory, slug);
  if (checkOnly) {
    let mismatch = false;
    const expectedPaths = new Set(files.keys());
    let actualPaths = [];
    try {
      actualPaths = await listFiles(actorDirectory);
    } catch {
      console.error(`${slug}: generated directory is missing`);
      return false;
    }
    for (const relative of [...new Set([...expectedPaths, ...actualPaths])].sort()) {
      const expected = files.get(relative);
      let actual;
      try {
        actual = await readFile(path.join(actorDirectory, relative));
      } catch {
        actual = undefined;
      }
      if (!expected || !actual || !expected.equals(actual)) {
        console.error(`${slug}: ${relative} is out of sync`);
        mismatch = true;
      }
    }
    return !mismatch;
  }

  await rm(actorDirectory, { recursive: true, force: true });
  for (const [relative, contents] of files) {
    const output = path.join(actorDirectory, relative);
    await mkdir(path.dirname(output), { recursive: true });
    await writeFile(output, contents);
  }
  console.log(`Generated actors/${slug}`);
  return true;
}

const sourceFiles = await templateFiles();
const specFiles = (await readdir(specsDirectory))
  .filter((filename) => filename.endsWith('.json'))
  .filter((filename) => !onlySlug || filename === `${onlySlug}.json`)
  .sort();
if (onlySlug && !specFiles.includes(`${onlySlug}.json`)) {
  console.error(`No spec found for ${onlySlug}`);
  process.exitCode = 1;
} else {
  let clean = true;
  for (const filename of specFiles) {
    const spec = JSON.parse(await readFile(path.join(specsDirectory, filename), 'utf8'));
    if (filename !== `${spec.slug}.json`) throw new Error(`${filename} does not match spec slug ${spec.slug}`);
    clean = await writeOrCheck(spec.slug, await render(spec, sourceFiles)) && clean;
  }
  if (checkOnly && clean) console.log(`Generated actor files are in sync (${specFiles.length} specs).`);
  if (!clean) process.exitCode = 1;
}

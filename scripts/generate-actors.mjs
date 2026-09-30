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
  return String(value ?? '').replaceAll('|', '\\|');
}

function propertySchema(parameter) {
  return {
    title: parameter.title ?? parameter.input,
    ...parameter.schema,
    editor: parameter.editor ?? editorFor(parameter.schema?.type),
  };
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
    batchProperties[parameter.input] = propertySchema(parameter);
  }
  const batchPrimary = parameterByInput.get(spec.batch.valueField);
  const batchItemProperties = {
    [spec.batch.valueField]: batchPrimary
      ? propertySchema(batchPrimary)
      : {
          title: spec.batch.valueField,
          type: 'string',
          minLength: 1,
          description: `Value sent as the ${spec.batch.apiParam} parameter.`,
          editor: 'textfield',
        },
  };
  for (const [name, schema] of Object.entries(batchProperties)) {
    if (name !== spec.batch.valueField) batchItemProperties[name] = schema;
  }

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
        required: [spec.batch.valueField],
        properties: batchItemProperties,
      },
    },
  };

  for (const parameter of spec.parameters) {
    if (parameter.input === spec.batch.valueField || parameter.location === 'path') continue;
    properties[parameter.input] = propertySchema(parameter);
  }

  if (spec.endpointByInput) {
    for (const [input, choices] of Object.entries(spec.endpointByInput)) {
      properties[input] = {
        title: 'Trip Type',
        type: 'string',
        description: 'Choose the flight search route. Round trips also require return_date.',
        editor: 'select',
        enum: Object.keys(choices),
        enumTitles: Object.keys(choices).map((choice) => choice === 'one-way' ? 'One way' : 'Round trip'),
        default: spec.endpointByInputDefault,
      };
    }
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

  const required = [spec.batch.field];
  for (const parameter of spec.parameters) {
    if (
      parameter.required
      && parameter.location !== 'path'
      && parameter.input !== spec.batch.valueField
      && !(parameter.requiredForEndpoints?.length)
    ) required.push(parameter.input);
  }
  if (spec.endpointByInput) required.push(...Object.keys(spec.endpointByInput));

  return {
    title: spec.title,
    description: `${spec.description} Submit multiple batch entries in one run.`,
    type: 'object',
    schemaVersion: 1,
    required: [...new Set(required)],
    properties,
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
    name: spec.slug,
    title: spec.title,
    description: spec.description,
    version: '1.0',
    buildTag: 'latest',
    input: './input_schema.json',
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

function exampleItem(spec) {
  const fixture = spec.fixture;
  let item = fixture;
  if (spec.mode === 'list') {
    const pointers = [spec.resultPointer, ...(spec.fallbackResultPointers ?? [])];
    const rows = pointers.map((resultPointer) => pointer(fixture, resultPointer)).find(Array.isArray);
    item = rows?.[0] ?? {};
  }
  item = item && typeof item === 'object' && !Array.isArray(item) ? { ...item } : { value: item };
  item[spec.enrichment.field] = spec.prefill[spec.batch.field][0][spec.batch.valueField];
  item.scraped_at = '2026-01-01T00:00:00Z';
  return item;
}

function readme(spec) {
  const inputExample = JSON.stringify(spec.prefill, null, 2);
  const outputExample = JSON.stringify(exampleItem(spec), null, 2);
  const keywords = spec.seo.keywords.map((keyword) => `\`${keyword}\``).join(', ');
  const fields = spec.seo.fieldDescriptions
    .map((field) => `- **${escapeMarkdown(field.name)}**: ${field.description}`)
    .join('\n');
  const useCases = spec.seo.useCases.map((useCase) => `- ${useCase}`).join('\n');
  const related = spec.seo.relatedActors
    .map((actor) => `- [${actor.title}](https://apify.com/thescrappa/${actor.slug})`)
    .join('\n');
  const price = `$${(spec.priceUsdPerResult * 1000).toFixed(2)} per 1,000 results`;
  const pagination = spec.pagination
    ? `This Actor supports pagination and stops at the configured **maxPages** or **maxResults** limit.`
    : `The Actor saves up to **maxResults** dataset items across the run.`;

  return `# ${spec.seo.h1}\n\n${spec.seo.intro}\n\n## Data you get\n\n${fields}\n\n## Use cases\n\n${useCases}\n\n## How to use\n\nAdd one or more entries to **${spec.batch.field}**. Each entry maps its **${spec.batch.valueField}** value to the Scrappa **${spec.batch.apiParam}** input. Shared endpoint options can be set at the top level.\n\n\`\`\`json\n${inputExample}\n\`\`\`\n\n## Output example\n\nThis synthetic example shows the response fields and the input value attached to each result.\n\n\`\`\`json\n${outputExample}\n\`\`\`\n\n## Pricing\n\n${price}. The Actor writes one dataset item for each result.\n\n${pagination}\n\n## FAQ\n\n### Is scraping this data legal?\n\nScraping rules depend on the source, the data, and how you use it. Review the applicable laws, source terms, and privacy requirements for your use case. You are responsible for your collection and use of the data.\n\n### Are there request limits?\n\nYou can submit up to 100 batch entries per run. Set **maxResults** to cap saved rows${spec.pagination ? ' and **maxPages** to bound pagination for each entry' : ''}. Scrappa API limits and source availability also apply.\n\n### Can I use the output with integrations or the API?\n\nYes. Read results from the Apify dataset, use the Apify API or client libraries, or connect the dataset to your existing data workflow. Each row includes **${spec.enrichment.field}** and **scraped_at** for traceability.\n\n## Related Actors\n\n${related}\n\n## Search terms\n\n${keywords}\n`;
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
  files.set('.actor/README.md', Buffer.from(readme(spec)));
  return files;
}

async function listFiles(directory, relative = '') {
  const result = [];
  for (const entry of await readdir(path.join(directory, relative), { withFileTypes: true })) {
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

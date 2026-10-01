import { existsSync } from 'node:fs';
import { readFile, readdir } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';

// Store titles may carry a price suffix such as "- $0.30/1k results"; README headings drop it.
export function headingTitle(title) {
  return title.replace(/\s*(?:-\s*\$[\d.]+\/1k results|\(\$[\d.]+\/1k results\))$/, '').trim();
}

const REPO_ROOT = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
const DEFAULT_LIVE_DIR = '/tmp/df3601fd/act';
const ALLOWED_CATEGORIES = new Set([
    'AI', 'AGENTS', 'AUTOMATION', 'BUSINESS', 'DEVELOPER_TOOLS', 'ECOMMERCE', 'JOBS',
    'LEAD_GENERATION', 'MARKETING', 'NEWS', 'SEO_TOOLS', 'SOCIAL_MEDIA', 'TRAVEL',
    'VIDEOS', 'REAL_ESTATE', 'OTHER', 'INTEGRATIONS', 'EDUCATION', 'FOR_CREATORS',
]);
const PRICING_FIELDS = ['FREE', 'BRONZE', 'SILVER', 'GOLD', 'PLATINUM', 'DIAMOND'];
const PROTECTED_TITLES = new Set([
    'Google Maps Advanced Search Scraper', 'LinkedIn Company Scraper', 'LinkedIn Profile Scraper',
    'Google Search Scraper', 'Google Images Scraper',
    'Instagram User Info | Cheapest $0.20/1k results',
    'Instagram Post Info | Cheapest $0.20/1k results',
    'Google Maps Photos Scraper', 'Vinted Search Scraper', 'Trustpilot Company Reviews Scraper',
]);
const BANNED_COPY_PHRASES = [
    'returned for this result', 'Provide the fields listed below', 'Results can include',
    'Save and export Apify datasets', 'Search terms', 'collects structured data from',
    'for Campaign Research', 'for Creator Research', 'for Audience Research', 'for Lead Research',
    'for Market Research', 'for Video Analysis', 'for Hiring Teams', 'Each entry maps its',
];
const AUDIENCE_SUFFIX = /\s+for (?:Campaign|Creator|Audience|Lead|Market|Video|Hiring|Candidate|Travel Planning|Property Buyers|Seller|Real Estate)\b/i;
const GENERIC_FAQ_QUESTIONS = new Set([
    'Is it legal to collect public information?',
    'How many records will a run return?',
    'Can I call it through the API or connect it to other tools?',
    'What happens when a request fails?',
]);
const BATCH_FIELD_NAMES = new Set([
    'urls', 'ids', 'domains', 'queries', 'searches', 'symbols', 'indices', 'routes', 'profiles',
    'keywords', 'challenge_ids', 'challenge_names', 'business_ids', 'property_ids', 'patent_ids',
    'doctorUrls', 'doctor_urls', 'usernames', 'items', 'targets', 'company_domains', 'ad_ids',
    'musicIds', 'tsids', 'item_ids', 'user_ids', 'hotels', 'locations',
]);

function humanJoin(values) {
    if (values.length < 2) return values[0] ?? '';
    if (values.length === 2) return `${values[0]} and ${values[1]}`;
    return `${values.slice(0, -1).join(', ')}, and ${values.at(-1)}`;
}

function formatUsd(value, decimals = 2) {
    return `$${Number(value).toFixed(decimals)}`;
}

function livePriceLine(actor) {
    const latest = actor.pricingInfos?.at(-1);
    const events = latest?.pricingPerEvent?.actorChargeEvents ?? {};
    const descriptions = [];

    for (const event of Object.values(events)) {
        const title = (event.eventTitle ?? 'result').trim();
        const lowerTitle = title.toLowerCase();
        const isActorStart = event.isOneTimeEvent === true || lowerTitle.includes('actor start');
        if (isActorStart) {
            if (event.eventPriceUsd == null) continue;
            const amount = formatUsd(event.eventPriceUsd, 5).replace(/0+$/, '').replace(/\.$/, '');
            descriptions.push(`${amount} per Actor Start event`);
            continue;
        }

        const unit = lowerTitle.includes('result')
            ? 'results'
            : lowerTitle.includes('search')
                ? 'searches'
                : lowerTitle.includes('quer')
                    ? 'queries'
                    : lowerTitle.includes('point')
                        ? 'price points'
                        : 'results';
        const tiers = event.eventTieredPricingUsd ?? {};
        if (Object.keys(tiers).length > 0) {
            const grouped = new Map();
            for (const tier of PRICING_FIELDS) {
                const price = tiers[tier]?.tieredEventPriceUsd;
                if (price == null) continue;
                const amount = formatUsd(price * 1000);
                grouped.set(amount, [...(grouped.get(amount) ?? []), tier[0] + tier.slice(1).toLowerCase()]);
            }
            const tierDescriptions = [];
            for (const [amount, names] of grouped) {
                const tierName = names.length === 1 && names[0] === 'Free'
                    ? 'Apify Free tier'
                    : humanJoin(names);
                tierDescriptions.push(`${tierName}: ${amount} per 1,000 ${unit}`);
            }
            if (tierDescriptions.length > 0) descriptions.push(tierDescriptions.join('; '));
        } else if (event.eventPriceUsd != null) {
            descriptions.push(`${formatUsd(event.eventPriceUsd * 1000)} per 1,000 ${unit}`);
        }
    }

    if (descriptions.length === 0) return 'No per-event rate is listed in the latest live pricing entry.';
    return `${descriptions.join('; plus ')}.`;
}

function isBatchProperty(key, property) {
    const description = String(property?.description ?? '');
    const type = property?.type;
    const isArray = type === 'array' || (Array.isArray(type) && type.includes('array'));
    return BATCH_FIELD_NAMES.has(key)
        && (isArray || /batch|multiple|many|one or more|list of|process many|run multiple/i.test(description));
}

function matchesSchemaType(value, type) {
    const allowedTypes = Array.isArray(type) ? type : [type];
    return allowedTypes.some((schemaType) => {
        if (schemaType === 'array') return Array.isArray(value);
        if (schemaType === 'object') return value !== null && typeof value === 'object' && !Array.isArray(value);
        if (schemaType === 'integer') return Number.isInteger(value);
        if (schemaType === 'number') return typeof value === 'number';
        if (schemaType === 'boolean') return typeof value === 'boolean';
        if (schemaType === 'string') return typeof value === 'string';
        return true;
    });
}

function selectedReadme(actorDirectory, actor) {
    if (typeof actor.readme === 'string' && actor.readme.trim()) {
        const candidates = [
            path.resolve(actorDirectory, actor.readme),
            path.resolve(actorDirectory, '.actor', actor.readme),
        ];
        return candidates.find((candidate) => candidate.startsWith(`${actorDirectory}${path.sep}`));
    }
    return path.join(actorDirectory, '.actor', 'README.md');
}

function markdownTableFieldNames(section) {
    return [...section.matchAll(/^\| `([^`]+)` \|/gm)].map((match) => match[1]);
}

function readJsonBlock(readme, actorName, errors, blockIndex = 0) {
    const matches = [...readme.matchAll(/```json\s*\n([\s\S]*?)\n```/g)];
    const match = matches[blockIndex];
    if (!match) {
        errors.push(`${actorName}: missing working JSON input example`);
        return null;
    }
    try {
        return JSON.parse(match[1]);
    } catch (error) {
        errors.push(`${actorName}: invalid JSON input example (${error.message})`);
        return null;
    }
}

function addError(errors, condition, message) {
    if (!condition) errors.push(message);
}

function collectValues(value, visit, key = '') {
    visit(value, key);
    if (Array.isArray(value)) {
        for (const child of value) collectValues(child, visit, key);
    } else if (value && typeof value === 'object') {
        for (const [childKey, child] of Object.entries(value)) collectValues(child, visit, childKey);
    }
}

function findPastPrefillDate(value, now = new Date()) {
    let pastDate = null;
    collectValues(value, (candidate) => {
        if (typeof candidate !== 'string') return;
        for (const match of candidate.matchAll(/\b20\d{2}-\d{2}-\d{2}\b/g)) {
            const date = new Date(`${match[0]}T00:00:00.000Z`);
            if (!Number.isNaN(date.valueOf()) && date < new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate()))) {
                pastDate = match[0];
                return;
            }
        }
    });
    return pastDate;
}

function containsEmptyJsonContainers(value) {
    if (Array.isArray(value)) return value.length === 0 || value.some(containsEmptyJsonContainers);
    if (value && typeof value === 'object') {
        return Object.keys(value).length === 0 || Object.values(value).some(containsEmptyJsonContainers);
    }
    return false;
}

async function jsonFile(filePath) {
    return JSON.parse(await readFile(filePath, 'utf8'));
}

async function main() {
    const liveArgIndex = process.argv.indexOf('--live-dir');
    const liveDirectory = liveArgIndex >= 0
        ? path.resolve(process.argv[liveArgIndex + 1] ?? '')
        : DEFAULT_LIVE_DIR;
    if (process.argv.some((arg, index) => arg === '--live-dir' && !process.argv[index + 1])) {
        throw new Error('Usage: node scripts/validate-store-pages.mjs [--live-dir <directory>]');
    }

    const metadata = await jsonFile(path.join(REPO_ROOT, 'docs/store-metadata.json'));
    const actorDirectory = path.join(REPO_ROOT, 'actors');
    const actorDefinitions = new Map();
    for (const directoryName of await readdir(actorDirectory)) {
        const directory = path.join(actorDirectory, directoryName);
        try {
            const actor = await jsonFile(path.join(directory, '.actor/actor.json'));
            // Spec-generated Actors are validated by scripts/validate-store-copy.mjs.
            if (existsSync(path.join(REPO_ROOT, 'specs', `${actor.name}.json`))) continue;
            actorDefinitions.set(actor.name, { directory, actor });
        } catch {
            // Non-actor directories do not have an actor definition.
        }
    }

    const liveActors = new Map();
    for (const fileName of await readdir(liveDirectory)) {
        if (!fileName.endsWith('.json')) continue;
        const snapshot = await jsonFile(path.join(liveDirectory, fileName));
        if (snapshot.data?.name) liveActors.set(snapshot.data.name, snapshot.data);
    }

    const errors = [];
    const metadataByName = new Map(metadata.map((entry) => [entry.name, entry]));
    addError(errors, metadata.length === 106, `metadata: expected 106 entries, found ${metadata.length}`);
    addError(errors, new Set(metadata.map((entry) => entry.name)).size === metadata.length, 'metadata: actor names are not unique');
    addError(errors, new Set(metadata.map((entry) => entry.id)).size === metadata.length, 'metadata: actor IDs are not unique');
    addError(errors, actorDefinitions.size === 106, `actors: expected 106 definitions, found ${actorDefinitions.size}`);

    let readmeCount = 0;
    let minimumReadmeLength = Number.POSITIVE_INFINITY;
    let maximumReadmeLength = 0;
    let checkedPricing = 0;
    const actorFaqQuestionOwners = new Map();

    for (const entry of metadata) {
        const actorRecord = actorDefinitions.get(entry.name);
        const liveActor = liveActors.get(entry.name);
        if (!actorRecord) {
            errors.push(`${entry.name}: actor directory not found`);
            continue;
        }
        if (!liveActor) {
            errors.push(`${entry.name}: live snapshot not found`);
            continue;
        }

        addError(errors, liveActor.id === entry.id, `${entry.name}: metadata ID does not match the live snapshot`);
        addError(errors, entry.title.length >= 15 && entry.title.length <= 60, `${entry.name}: title length ${entry.title.length} is outside 15-60`);
        addError(errors, entry.title.length <= 45 || PROTECTED_TITLES.has(entry.title), `${entry.name}: title is padded beyond the natural 45-character range`);
        addError(errors, !AUDIENCE_SUFFIX.test(entry.title), `${entry.name}: title contains an audience suffix`);
        addError(errors, entry.description.length <= 300, `${entry.name}: description exceeds 300 characters`);
        addError(errors, entry.seoTitle.length <= 60, `${entry.name}: SEO title exceeds 60 characters`);
        addError(errors, entry.seoDescription.length >= 145 && entry.seoDescription.length <= 155, `${entry.name}: SEO description length ${entry.seoDescription.length} is outside 145-155`);
        const metadataText = [entry.title, entry.description, entry.seoTitle, entry.seoDescription].join('\n');
        addError(errors, !metadataText.includes('`'), `${entry.name}: metadata text contains backtick field names`);
        for (const phrase of BANNED_COPY_PHRASES) {
            addError(errors, !metadataText.toLowerCase().includes(phrase.toLowerCase()), `${entry.name}: metadata contains banned phrase "${phrase}"`);
        }
        addError(errors, Array.isArray(entry.categories) && entry.categories.length >= 1 && entry.categories.length <= 3, `${entry.name}: category count is outside 1-3`);
        addError(errors, entry.categories.every((category) => ALLOWED_CATEGORIES.has(category)), `${entry.name}: has an unsupported category`);
        addError(errors, entry.categories[0] !== 'DEVELOPER_TOOLS', `${entry.name}: DEVELOPER_TOOLS cannot be the first category`);

        const actor = actorRecord.actor;
        addError(errors, actor.title === entry.title, `${entry.name}: .actor/actor.json title differs from metadata`);
        addError(errors, actor.description === entry.description, `${entry.name}: .actor/actor.json description differs from metadata`);

        const readmePath = selectedReadme(actorRecord.directory, actor);
        if (!readmePath) {
            errors.push(`${entry.name}: could not resolve actor.json readme path`);
            continue;
        }
        let readme;
        try {
            readme = await readFile(readmePath, 'utf8');
        } catch {
            errors.push(`${entry.name}: selected Store README is missing (${path.relative(REPO_ROOT, readmePath)})`);
            continue;
        }
        readmeCount += 1;
        minimumReadmeLength = Math.min(minimumReadmeLength, readme.length);
        maximumReadmeLength = Math.max(maximumReadmeLength, readme.length);
        addError(errors, readme.length >= 3000, `${entry.name}: README length ${readme.length} is below 3,000`);
        addError(errors, /^# .+$/m.test(readme), `${entry.name}: missing H1`);
        addError(errors, /^# .+$/m.exec(readme)?.[0] === `# ${headingTitle(entry.title)}`, `${entry.name}: H1 does not match the Store title`);
        addError(errors, !readme.includes('—'), `${entry.name}: contains an em dash`);
        addError(errors, !readme.includes('{{'), `${entry.name}: contains a template placeholder`);
        addError(errors, !/\b(?:Rust ports?|QA runs?|run IDs?|build numbers?|maintenance notices?|test fixtures?|internal endpoints?)\b/i.test(readme), `${entry.name}: contains internal jargon`);
        for (const phrase of BANNED_COPY_PHRASES) {
            addError(errors, !readme.toLowerCase().includes(phrase.toLowerCase()), `${entry.name}: README contains banned phrase "${phrase}"`);
        }

        const intro = readme.split(/^## What data can you extract\?$/m)[0]
            .replace(/^# .+$/m, '')
            .trim();
        const introSentenceCount = intro.split(/(?<=[.!?])\s+/).filter(Boolean).length;
        addError(errors, introSentenceCount >= 2 && introSentenceCount <= 3, `${entry.name}: intro has ${introSentenceCount} sentences instead of 2-3`);

        for (const requiredHeading of [
            'What data can you extract?', 'Use cases', 'How to use', 'Output example', 'Input', 'Pricing', 'FAQ', 'Related Scrappa Actors',
        ]) {
            addError(errors, readme.includes(`## ${requiredHeading}`), `${entry.name}: missing section "${requiredHeading}"`);
        }

        const priceMatch = readme.match(/^\*\*Current live price:\*\* (.+)$/m);
        const expectedPrice = livePriceLine(liveActor);
        addError(errors, priceMatch?.[1] === expectedPrice, `${entry.name}: pricing line differs from latest live pricing (expected "${expectedPrice}")`);
        if (priceMatch?.[1] === expectedPrice) checkedPricing += 1;

        const related = [...readme.matchAll(/https:\/\/apify\.com\/thescrappa\/([a-z0-9-]+)/g)].map((match) => match[1]);
        const uniqueRelated = [...new Set(related)];
        addError(errors, uniqueRelated.length >= 3 && uniqueRelated.length <= 6, `${entry.name}: expected 3-6 related actor links, found ${uniqueRelated.length}`);
        addError(errors, uniqueRelated.every((name) => metadataByName.has(name)), `${entry.name}: related actor URL contains an unknown actor slug`);

        const outputSection = readme.split('## What data can you extract?')[1]?.split('## Use cases')[0] ?? '';
        const outputFields = new Set(markdownTableFieldNames(outputSection));
        const outputBlock = readJsonBlock(readme, entry.name, errors, 1);
        const outputExampleSection = readme.split('## Output example')[1]?.split('## Pricing')[0] ?? '';
        addError(errors, !/\bExample result\b/i.test(outputExampleSection), `${entry.name}: output example uses the banned "Example result" placeholder`);
        addError(errors, !/^\s*(?:\{\}|\[\])\s*$/m.test(outputExampleSection), `${entry.name}: output example contains an empty JSON placeholder`);
        if (outputBlock && typeof outputBlock === 'object' && !Array.isArray(outputBlock)) {
            addError(errors, Object.keys(outputBlock).length > 0, `${entry.name}: output example is an empty object`);
            addError(errors, !containsEmptyJsonContainers(outputBlock), `${entry.name}: output example contains an empty object or array`);
            let number42Count = 0;
            collectValues(outputBlock, (value, key) => {
                if (value === 42) number42Count += 1;
                if (typeof value === 'string') {
                    addError(errors, !/record_demo_\d+|^Board name for the\b|\bExample result\b/i.test(value), `${entry.name}: output example contains a synthetic placeholder value`);
                }
                if (/(?:email|phone|telephone)/i.test(key)) addError(errors, value === null, `${entry.name}: output example must not contain a real email or phone value`);
            });
            addError(errors, number42Count < 2, `${entry.name}: output example repeats the placeholder value 42`);
            addError(errors, Object.keys(outputBlock).every((field) => outputFields.has(field)), `${entry.name}: output example contains a field not listed in the output table`);
        }

        const schemaPath = path.join(actorRecord.directory, '.actor/input_schema.json');
        const inputSchema = await jsonFile(schemaPath);
        const inputExample = readJsonBlock(readme, entry.name, errors);
        if (inputExample) {
            const properties = inputSchema.properties ?? {};
            addError(errors, Object.keys(inputExample).every((key) => Object.hasOwn(properties, key)), `${entry.name}: input example contains an unknown top-level field`);
            for (const required of inputSchema.required ?? []) {
                addError(errors, Object.hasOwn(inputExample, required), `${entry.name}: input example omits required field "${required}"`);
            }
            const batchFields = Object.entries(properties)
                .filter(([key, property]) => isBatchProperty(key, property))
                .map(([key]) => key);
            if (batchFields.length > 0) {
                addError(errors, batchFields.some((key) => Object.hasOwn(inputExample, key)), `${entry.name}: batch-capable actor has no batch field in the input example`);
            }
        }

        for (const [key, property] of Object.entries(inputSchema.properties ?? {})) {
            if (!property || typeof property !== 'object') continue;
            if (Object.hasOwn(property, 'prefill')) {
                addError(errors, matchesSchemaType(property.prefill, property.type), `${entry.name}: prefill "${key}" does not match its schema type`);
            }
            const value = property.prefill ?? property.default;
            const pastDate = findPastPrefillDate(property.prefill);
            addError(errors, !pastDate, `${entry.name}: prefill "${key}" contains fixed past date ${pastDate}`);
            if (Array.isArray(property.prefill)) addError(errors, property.prefill.length <= 2, `${entry.name}: prefill "${key}" has more than two values`);
            if (/(limit|count|num_homes|max_results)/i.test(key) && Number.isFinite(value)) {
                addError(errors, value <= 10, `${entry.name}: prefill/default "${key}" is above the 10-result QA cap`);
            }
            if (entry.name.startsWith('youtube-api-') && ['id', 'ids'].includes(key) && typeof property.prefill === 'string') {
                const targetCount = property.prefill.split(',').filter(Boolean).length;
                addError(errors, targetCount <= 2, `${entry.name}: prefill "${key}" has more than two comma-separated targets`);
            }
        }

        const faqSection = readme.split('## FAQ')[1]?.split('## Related Scrappa Actors')[0] ?? '';
        const faqQuestions = [...faqSection.matchAll(/^### (.+)$/gm)].map((match) => match[1]);
        const actorFaqQuestions = faqQuestions.filter((question) => !GENERIC_FAQ_QUESTIONS.has(question));
        addError(errors, actorFaqQuestions.length >= 1 && actorFaqQuestions.length <= 3, `${entry.name}: expected 1-3 Actor-specific FAQ questions beyond the four generic questions, found ${actorFaqQuestions.length}`);
        addError(errors, new Set(actorFaqQuestions).size === actorFaqQuestions.length, `${entry.name}: Actor-specific FAQ questions are duplicated`);
        addError(errors, [...GENERIC_FAQ_QUESTIONS].every((question) => faqQuestions.includes(question))
            && faqQuestions.filter((question) => GENERIC_FAQ_QUESTIONS.has(question)).length === GENERIC_FAQ_QUESTIONS.size,
        `${entry.name}: FAQ is missing one or more of the four generic questions`);
        for (const question of actorFaqQuestions) {
            const previousOwner = actorFaqQuestionOwners.get(question);
            addError(errors, !previousOwner, `${entry.name}: Actor-specific FAQ question is reused from ${previousOwner}`);
            actorFaqQuestionOwners.set(question, entry.name);
        }

        const apiEndpoint = `POST https://api.apify.com/v2/acts/thescrappa~${entry.name}/runs`;
        addError(errors, readme.includes(apiEndpoint), `${entry.name}: FAQ is missing its exact Apify API run URL`);

        const rootReadme = path.join(actorRecord.directory, 'README.md');
        const actorReadme = path.join(actorRecord.directory, '.actor/README.md');
        try {
            const [rootText, actorText] = await Promise.all([readFile(rootReadme, 'utf8'), readFile(actorReadme, 'utf8')]);
            addError(errors, rootText === actorText, `${entry.name}: root README and .actor/README.md differ`);
        } catch {
            errors.push(`${entry.name}: root README or .actor/README.md is missing`);
        }
    }

    const inventoryLines = (await readFile(path.join(REPO_ROOT, 'README.md'), 'utf8'))
        .split('\n')
        .filter((line) => line.startsWith('| `actors/'));
    addError(errors, inventoryLines.length === 106, `root README inventory: expected 106 actor rows, found ${inventoryLines.length}`);
    for (const line of inventoryLines) {
        const cells = line.split('|').map((cell) => cell.trim());
        const name = cells[2]?.replaceAll('`', '');
        const title = cells[4]?.replaceAll('&#124;', '|');
        const metadataEntry = metadataByName.get(name);
        if (metadataEntry) addError(errors, title === metadataEntry.title, `root README inventory: title mismatch for ${name}`);
    }

    const requiredPrefills = {
        'youtube-api-playlists': ['q', 'music'],
        'youtube-api-hashtags': ['hashtag', 'music'],
    };
    for (const [name, [key, expected]] of Object.entries(requiredPrefills)) {
        const actorRecord = actorDefinitions.get(name);
        const schema = actorRecord ? await jsonFile(path.join(actorRecord.directory, '.actor/input_schema.json')) : {};
        addError(errors, schema.properties?.[key]?.prefill === expected, `${name}: expected ${key} prefill ${JSON.stringify(expected)}`);
    }
    const trending = actorDefinitions.get('youtube-api-trending-videos');
    const trendingSchema = trending ? await jsonFile(path.join(trending.directory, '.actor/input_schema.json')) : {};
    addError(errors, !Object.hasOwn(trendingSchema.properties?.category ?? {}, 'prefill'), 'youtube-api-trending-videos: category prefill must be removed');

    if (errors.length > 0) {
        console.error(`Store validation failed with ${errors.length} issue(s):`);
        for (const error of errors) console.error(`- ${error}`);
        process.exitCode = 1;
        return;
    }

    console.log(`Validated ${readmeCount} actor Store READMEs and ${metadata.length} metadata entries.`);
    console.log(`README length range: ${minimumReadmeLength}-${maximumReadmeLength} characters (minimum required: 3,000).`);
    console.log(`Pricing lines match the latest live pricing entry for ${checkedPricing} actors.`);
    console.log('All section, H1 and intro checks passed, including output realism, unique Actor-specific FAQs, banned-copy, related-link, input-example, prefill, metadata, pricing and actor.json checks.');
}

main().catch((error) => {
    console.error(error.message);
    process.exitCode = 1;
});

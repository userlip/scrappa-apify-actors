import { readFile, readdir } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';

const REPO_ROOT = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
const DEFAULT_LIVE_DIR = '/tmp/df3601fd/act';
const ALLOWED_CATEGORIES = new Set([
    'AI', 'AGENTS', 'AUTOMATION', 'BUSINESS', 'DEVELOPER_TOOLS', 'ECOMMERCE', 'JOBS',
    'LEAD_GENERATION', 'MARKETING', 'NEWS', 'SEO_TOOLS', 'SOCIAL_MEDIA', 'TRAVEL',
    'VIDEOS', 'REAL_ESTATE', 'OTHER', 'INTEGRATIONS', 'EDUCATION', 'FOR_CREATORS',
]);
const PRICING_FIELDS = ['FREE', 'BRONZE', 'SILVER', 'GOLD', 'PLATINUM', 'DIAMOND'];
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

        const unit = lowerTitle.includes('search')
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
        addError(errors, entry.title.length >= 40 && entry.title.length <= 50, `${entry.name}: title length ${entry.title.length} is outside 40-50`);
        addError(errors, entry.description.length <= 300, `${entry.name}: description exceeds 300 characters`);
        addError(errors, entry.seoTitle.length >= 40 && entry.seoTitle.length <= 50, `${entry.name}: SEO title length ${entry.seoTitle.length} is outside 40-50`);
        addError(errors, entry.seoDescription.length >= 145 && entry.seoDescription.length <= 155, `${entry.name}: SEO description length ${entry.seoDescription.length} is outside 145-155`);
        addError(errors, Array.isArray(entry.categories) && entry.categories.length >= 1 && entry.categories.length <= 3, `${entry.name}: category count is outside 1-3`);
        addError(errors, entry.categories.every((category) => ALLOWED_CATEGORIES.has(category)), `${entry.name}: has an unsupported category`);

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
        addError(errors, /^# .+$/m.exec(readme)?.[0] === `# ${entry.title}`, `${entry.name}: H1 does not match the Store title`);
        addError(errors, !readme.includes('—'), `${entry.name}: contains an em dash`);
        addError(errors, !readme.includes('{{'), `${entry.name}: contains a template placeholder`);
        addError(errors, !/\b(?:Rust ports?|QA runs?|run IDs?|build numbers?|maintenance notices?|test fixtures?|internal endpoints?)\b/i.test(readme), `${entry.name}: contains internal jargon`);

        for (const requiredHeading of [
            'What data can you extract?', 'Use cases', 'How to use', 'Output example', 'Input fields', 'Pricing', 'FAQ', 'Related Scrappa Actors',
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
        if (outputBlock && typeof outputBlock === 'object' && !Array.isArray(outputBlock)) {
            addError(errors, Object.keys(outputBlock).every((field) => outputFields.has(field)), `${entry.name}: output example contains a field not listed in the output table`);
            for (const [field, value] of Object.entries(outputBlock)) {
                if (/email|phone|telephone/i.test(field)) addError(errors, value === null, `${entry.name}: output example must not contain a real email or phone value`);
            }
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
            if (Array.isArray(property.prefill)) addError(errors, property.prefill.length <= 2, `${entry.name}: prefill "${key}" has more than two values`);
            if (/(limit|count|num_homes|max_results)/i.test(key) && Number.isFinite(value)) {
                addError(errors, value <= 10, `${entry.name}: prefill/default "${key}" is above the 10-result QA cap`);
            }
            if (entry.name.startsWith('youtube-api-') && ['id', 'ids'].includes(key) && typeof property.prefill === 'string') {
                const targetCount = property.prefill.split(',').filter(Boolean).length;
                addError(errors, targetCount <= 2, `${entry.name}: prefill "${key}" has more than two comma-separated targets`);
            }
        }

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
    console.log('All README section, H1, placeholder, related-link, input-example, prefill, metadata-length, actor.json mirror, and inventory checks passed.');
}

main().catch((error) => {
    console.error(error.message);
    process.exitCode = 1;
});

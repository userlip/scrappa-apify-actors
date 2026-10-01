import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { isDeepStrictEqual } from 'node:util';
import { fileURLToPath } from 'node:url';

const API_BASE_URL = 'https://api.apify.com/v2/acts';
const METADATA_PATH = new URL('../docs/store-metadata.json', import.meta.url);
const FIELDS = ['title', 'description', 'seoTitle', 'seoDescription', 'categories'];

export function diffMetadata(entry, live) {
    return Object.fromEntries(
        FIELDS
            .filter((field) => !isDeepStrictEqual(entry[field], live[field]))
            .map((field) => [field, { before: live[field] ?? null, after: entry[field] }]),
    );
}

function formatValue(value) {
    return JSON.stringify(value);
}

function printDiff(entry, diff) {
    console.log(`${entry.name} (${entry.id})`);
    for (const [field, values] of Object.entries(diff)) {
        console.log(`  ${field}: ${formatValue(values.before)} -> ${formatValue(values.after)}`);
    }
}

async function readResponse(response, operation, actorName) {
    const body = await response.text();
    if (!response.ok) {
        throw new Error(`${operation} failed for ${actorName}: HTTP ${response.status} ${body}`);
    }
    return body ? JSON.parse(body) : {};
}

async function main() {
    const args = process.argv.slice(2);
    if (args.some((arg) => arg !== '--dry-run')) {
        throw new Error('Usage: node scripts/apply-store-metadata.mjs [--dry-run]');
    }

    const dryRun = args.includes('--dry-run');
    const token = process.env.APIFY_TOKEN?.trim();
    if (!token) {
        throw new Error('Set APIFY_TOKEN before running this script.');
    }

    const metadata = JSON.parse(await readFile(METADATA_PATH, 'utf8'));
    let changedCount = 0;

    for (const entry of metadata) {
        const url = `${API_BASE_URL}/${encodeURIComponent(entry.id)}`;
        const headers = { Authorization: `Bearer ${token}`, Accept: 'application/json' };
        const getResponse = await fetch(url, { headers });
        const body = await readResponse(getResponse, 'GET', entry.name);
        const live = body.data ?? body;
        const diff = diffMetadata(entry, live);

        if (Object.keys(diff).length === 0) {
            console.log(`${entry.name} (${entry.id}): no changes`);
            continue;
        }

        changedCount += 1;
        printDiff(entry, diff);
        if (dryRun) continue;

        const payload = Object.fromEntries(FIELDS.map((field) => [field, entry[field]]));
        const putResponse = await fetch(url, {
            method: 'PUT',
            headers: { ...headers, 'Content-Type': 'application/json' },
            body: JSON.stringify(payload),
        });
        await readResponse(putResponse, 'PUT', entry.name);
    }

    console.log(`${dryRun ? 'Dry run' : 'Updated'} complete: ${changedCount} actor(s) with changes out of ${metadata.length}.`);
}

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
    main().catch((error) => {
        console.error(error.message);
        process.exitCode = 1;
    });
}

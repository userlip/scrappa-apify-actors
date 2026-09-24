import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const actorDirectory = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const schema = JSON.parse(await readFile(resolve(actorDirectory, '.actor/input_schema.json'), 'utf8'));
const prefilledInput = Object.fromEntries(
    Object.entries(schema.properties)
        .filter(([, property]) => property.prefill !== undefined)
        .map(([name, property]) => [name, property.prefill]),
);

assert.deepEqual(prefilledInput, {
    url: 'https://www.booking.com/hotel/fr/ritz-paris.html',
    country: 'fr',
    slug: 'ritz-paris',
});

const datasetItems = [];
const charges = [];
const statusUpdates = [];
const scrappaRequests = [];
let inputReads = 0;
let loseNextDatasetResponse = false;
const server = createServer(async (request, response) => {
    const requestUrl = new URL(request.url, 'http://127.0.0.1');
    const chunks = [];
    for await (const chunk of request) chunks.push(chunk);
    const body = Buffer.concat(chunks).toString('utf8');

    if (requestUrl.pathname === '/v2/key-value-stores/store-1/records/INPUT' && request.method === 'GET') {
        inputReads += 1;
        if (inputReads === 1) return sendJson(response, 503, { error: 'temporary Apify API failure' });
        return sendJson(response, 200, prefilledInput);
    }
    if (requestUrl.pathname === '/v2/actor-runs/run-1' && request.method === 'GET') {
        return sendJson(response, 200, {
            data: {
                pricingInfo: {
                    pricingModel: 'PAY_PER_EVENT',
                    pricingPerEvent: {
                        actorChargeEvents: {
                            'hotel-result': { eventTitle: 'Hotel result', eventPriceUsd: 0.1 },
                            'apify-default-dataset-item': { eventTitle: 'Dataset item', eventPriceUsd: 0.02 },
                        },
                    },
                },
                options: { maxTotalChargeUsd: 0.12 },
                chargedEventCounts: {},
            },
        });
    }
    if (requestUrl.pathname === '/api/booking/hotel' && request.method === 'GET') {
        scrappaRequests.push({ url: requestUrl, headers: request.headers });
        return sendJson(response, 200, {
            success: true,
            data: {
                title: 'Ritz Paris QA prefill smoke',
                canonical_url: prefilledInput.url,
                hotel_schema: { '@type': 'Hotel', name: 'Ritz Paris QA prefill smoke' },
                aggregate_rating: { ratingValue: '9.4' },
                json_ld: [{ '@type': 'Hotel' }],
                parsed: true,
            },
        });
    }
    if (requestUrl.pathname === '/v2/datasets/dataset-1/items' && request.method === 'POST') {
        datasetItems.push(JSON.parse(body));
        if (loseNextDatasetResponse) {
            loseNextDatasetResponse = false;
            request.socket.destroy();
            return;
        }
        return sendJson(response, 201, { data: {} });
    }
    if (requestUrl.pathname === '/v2/actor-runs/run-1/charge' && request.method === 'POST') {
        charges.push({ body: JSON.parse(body), idempotencyKey: request.headers['idempotency-key'] });
        return sendJson(response, 200, { data: {} });
    }
    if (requestUrl.pathname === '/v2/actor-runs/run-1' && request.method === 'PUT') {
        statusUpdates.push(JSON.parse(body));
        return sendJson(response, 200, { data: {} });
    }

    return sendJson(response, 404, { error: `Unexpected ${request.method} ${requestUrl.pathname}` });
});

function sendJson(response, status, value) {
    response.writeHead(status, { 'content-type': 'application/json' });
    response.end(JSON.stringify(value));
}

await new Promise((resolveListen) => server.listen(0, '0.0.0.0', resolveListen));
const { port } = server.address();
const image = process.env.BOOKING_HOTEL_ACTOR_IMAGE ?? 'booking-hotel-details-scraper:local';
const dockerArguments = [
    'run', '--rm', '--network', 'host',
    '-e', `APIFY_API_PUBLIC_BASE_URL=http://127.0.0.1:${port}`,
    '-e', 'APIFY_TOKEN=smoke-token',
    '-e', 'APIFY_IS_AT_HOME=true',
    '-e', 'ACTOR_RUN_ID=run-1',
    '-e', 'ACTOR_DEFAULT_KEY_VALUE_STORE_ID=store-1',
    '-e', 'ACTOR_DEFAULT_DATASET_ID=dataset-1',
    '-e', 'ACTOR_INPUT_KEY=INPUT',
    '-e', 'SCRAPPA_API_KEY=smoke-scrappa-key',
    '-e', `SCRAPPA_API_BASE_URL=http://127.0.0.1:${port}/api`,
    image,
];
const run = await runDocker(dockerArguments);

try {
    assert.equal(run.error, undefined, run.error?.message);
    assert.equal(run.status, 0, `Docker actor exited ${run.status}\n${run.stdout}\n${run.stderr}`);
    assert.equal(scrappaRequests.length, 1);
    assert.equal(inputReads, 2);
    assert.equal(scrappaRequests[0].url.searchParams.get('url'), prefilledInput.url);
    assert.equal(scrappaRequests[0].url.searchParams.has('country'), false);
    assert.equal(scrappaRequests[0].headers['x-api-key'], 'smoke-scrappa-key');
    assert.equal(scrappaRequests[0].headers['user-agent'], 'thescrappa-booking-hotel-details-scraper/1.0');
    assert.equal(datasetItems.length, 1);
    assert.equal(datasetItems[0].title, 'Ritz Paris QA prefill smoke');
    assert.equal(datasetItems[0].request_index, 0);
    assert.equal(datasetItems[0].request_input_type, 'url');
    assert.equal(datasetItems[0].request_url, prefilledInput.url);
    assert.equal(datasetItems[0].request_success, true);
    assert.deepEqual(charges, [{
        body: { eventName: 'hotel-result', count: 1 },
        idempotencyKey: 'run-1-hotel-result-0',
    }]);
    assert.equal(statusUpdates.length, 1);
    assert.equal(statusUpdates[0].statusMessage, 'Charge limit reached after saving Booking.com hotel detail result 1.');
    assert.equal(statusUpdates[0].isStatusMessageTerminal, true);
    assert.equal(statusUpdates[0].level, 'INFO');
    assert.match(run.stdout, /Running 1 Booking\.com hotel detail request\(s\)/);
    assert.match(run.stdout, /"charged_count":2/);
    console.log('Local image smoke passed: schema prefills -> Scrappa request -> dataset row -> hotel-result charge.');
    console.log(`Image logs:\n${run.stdout.trim()}`);

    const datasetCountBeforeLostResponse = datasetItems.length;
    const chargeCountBeforeLostResponse = charges.length;
    loseNextDatasetResponse = true;
    const lostResponseRun = await runDocker(dockerArguments);

    assert.equal(lostResponseRun.error, undefined, lostResponseRun.error?.message);
    assert.equal(lostResponseRun.status, 1, `Docker actor should fail after an ambiguous dataset append\n${lostResponseRun.stdout}\n${lostResponseRun.stderr}`);
    assert.equal(datasetItems.length, datasetCountBeforeLostResponse + 1);
    assert.equal(datasetItems.at(-1).request_success, true);
    assert.equal(charges.length, chargeCountBeforeLostResponse);
    assert.equal(statusUpdates.length, 2);
    assert.equal(statusUpdates.at(-1).level, 'ERROR');
    console.log('Lost-response smoke passed: the stored result was not followed by an error row or charge.');
} finally {
    await new Promise((resolveClose) => server.close(resolveClose));
}

function runDocker(arguments_) {
    return new Promise((resolveRun, rejectRun) => {
        const child = spawn('docker', arguments_, { stdio: ['ignore', 'pipe', 'pipe'] });
        let stdout = '';
        let stderr = '';
        let timedOut = false;
        const timeout = setTimeout(() => {
            timedOut = true;
            child.kill('SIGTERM');
        }, 30_000);

        child.stdout.setEncoding('utf8').on('data', (chunk) => { stdout += chunk; });
        child.stderr.setEncoding('utf8').on('data', (chunk) => { stderr += chunk; });
        child.once('error', (error) => {
            clearTimeout(timeout);
            rejectRun(error);
        });
        child.once('close', (status, signal) => {
            clearTimeout(timeout);
            resolveRun({
                status,
                stdout,
                stderr,
                error: timedOut ? new Error(`docker run timed out (${signal ?? 'no signal'})`) : undefined,
            });
        });
    });
}

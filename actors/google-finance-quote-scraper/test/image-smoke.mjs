import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createServer } from 'node:http';

const image = process.argv[2] ?? 'google-finance-quote-scraper:local';
const input = {
    symbol: 'AAPL',
    exchange: 'NASDAQ',
    period_type: 'quarterly',
    hl: 'en',
    gl: 'us',
};
const upstreamResponse = {
    quote: {
        summary: {
            symbol: 'AAPL',
            exchange: 'NASDAQ',
            name: 'Apple Inc',
            current_price: '198.53',
            currency: 'USD',
            price_change: '1.18',
            percent_change: '0.6',
            market_status: 'Closed',
            extensions: ['USD'],
        },
        key_stats: { 'Market cap': '2.96T USD' },
        about: { description: 'Apple Inc. profile' },
        financials: [{ title: 'Revenue' }],
        news: [{ title: 'Apple news' }],
        discover_more: [{ title: 'Related', items: [{ symbol: 'MSFT' }] }],
    },
    pagination: { current_page: 1, has_next_page: false },
};
const sharedEventPrices = {
    'quote-result': { eventPriceUsd: 0.001 },
    'apify-default-dataset-item': { eventPriceUsd: 0.0002 },
    'other-result': { eventPriceUsd: 0.01 },
};
const captured = {
    apifyRequests: [],
    scrappaRequests: [],
    datasetItems: [],
    chargedEvents: [],
    outputs: [],
    errors: [],
};
let runNumber = 0;

function sendJson(response, status, value) {
    response.writeHead(status, { 'content-type': 'application/json' });
    response.end(JSON.stringify(value));
}

async function readJson(request) {
    let body = '';
    for await (const chunk of request) body += chunk;
    return body ? JSON.parse(body) : null;
}

const apifyServer = createServer(async (request, response) => {
    try {
        assert.equal(request.headers.authorization, 'Bearer smoke-apify-token');
        captured.apifyRequests.push(`${request.method} ${request.url}`);

        if (request.method === 'GET' && request.url === '/v2/actor-runs/smoke-run') {
            runNumber += 1;
            sendJson(response, 200, {
                data: {
                    pricingInfo: {
                        pricingModel: 'PAY_PER_EVENT',
                        pricingPerEvent: { actorChargeEvents: sharedEventPrices },
                    },
                    chargedEventCounts: runNumber === 1 ? {} : { 'other-result': 1 },
                    options: { maxTotalChargeUsd: 0.005 },
                },
            });
            return;
        }

        if (request.method === 'GET' && request.url === '/v2/key-value-stores/smoke-store/records/INPUT') {
            sendJson(response, 200, input);
            return;
        }

        if (request.method === 'POST' && request.url === '/v2/datasets/smoke-dataset/items') {
            captured.datasetItems.push(await readJson(request));
            response.writeHead(201);
            response.end();
            return;
        }

        if (request.method === 'POST' && request.url === '/v2/actor-runs/smoke-run/charge') {
            assert.ok(request.headers['idempotency-key']);
            captured.chargedEvents.push(await readJson(request));
            response.writeHead(201);
            response.end();
            return;
        }

        if (request.method === 'PUT' && request.url === '/v2/key-value-stores/smoke-store/records/OUTPUT') {
            captured.outputs.push(await readJson(request));
            response.writeHead(201);
            response.end();
            return;
        }

        sendJson(response, 404, { message: `Unexpected Apify request: ${request.method} ${request.url}` });
    } catch (error) {
        captured.errors.push(error instanceof Error ? error.message : String(error));
        sendJson(response, 500, { message: 'Smoke fixture assertion failed' });
    }
});

const scrappaServer = createServer((request, response) => {
    try {
        assert.equal(request.method, 'GET');
        assert.equal(request.url.split('?')[0], '/api/google-finance/quote');
        assert.equal(request.headers['x-api-key'], 'smoke-scrappa-key');
        assert.equal(request.headers['user-agent'], 'thescrappa-google-finance-quote-scraper/1.0');
        const query = new URL(request.url, 'http://localhost').searchParams;
        for (const [key, value] of Object.entries(input)) assert.equal(query.get(key), value);
        captured.scrappaRequests.push(request.url);
        sendJson(response, 200, upstreamResponse);
    } catch (error) {
        captured.errors.push(error instanceof Error ? error.message : String(error));
        sendJson(response, 500, { message: 'Smoke fixture assertion failed' });
    }
});

async function listen(server) {
    await new Promise((resolve, reject) => {
        server.once('error', reject);
        server.listen(0, '127.0.0.1', resolve);
    });
    return server.address().port;
}

function runImage(environment) {
    const args = ['run', '--rm', '--network', 'host'];
    for (const [key, value] of Object.entries(environment)) args.push('--env', `${key}=${value}`);
    args.push(image);

    return new Promise((resolve, reject) => {
        const child = spawn('docker', args, { stdio: ['ignore', 'pipe', 'pipe'] });
        let stdout = '';
        let stderr = '';
        const timeout = setTimeout(() => child.kill('SIGKILL'), 180_000);
        child.stdout.setEncoding('utf8').on('data', (chunk) => { stdout += chunk; });
        child.stderr.setEncoding('utf8').on('data', (chunk) => { stderr += chunk; });
        child.once('error', (error) => {
            clearTimeout(timeout);
            reject(error);
        });
        child.once('close', (code) => {
            clearTimeout(timeout);
            resolve({ code, stdout, stderr });
        });
    });
}

async function close(server) {
    await new Promise((resolve) => server.close(resolve));
}

const [apifyPort, scrappaPort] = await Promise.all([listen(apifyServer), listen(scrappaServer)]);
const environment = {
    APIFY_API_PUBLIC_BASE_URL: `http://127.0.0.1:${apifyPort}`,
    APIFY_IS_AT_HOME: '1',
    APIFY_TOKEN: 'smoke-apify-token',
    ACTOR_RUN_ID: 'smoke-run',
    ACTOR_DEFAULT_KEY_VALUE_STORE_ID: 'smoke-store',
    ACTOR_DEFAULT_DATASET_ID: 'smoke-dataset',
    ACTOR_INPUT_KEY: 'INPUT',
    SCRAPPA_API_BASE_URL: `http://127.0.0.1:${scrappaPort}/api`,
    SCRAPPA_API_KEY: 'smoke-scrappa-key',
};

try {
    const successfulRun = await runImage(environment);
    assert.equal(successfulRun.code, 0, `${successfulRun.stdout}\n${successfulRun.stderr}`);
    assert.match(successfulRun.stdout, /Google Finance quote scraping completed successfully/);

    const budgetLimitedRun = await runImage(environment);
    assert.equal(budgetLimitedRun.code, 0, `${budgetLimitedRun.stdout}\n${budgetLimitedRun.stderr}`);
    assert.match(budgetLimitedRun.stdout, /Charge limit reached before saving the Google Finance quote result/);

    assert.deepEqual(captured.errors, []);
    assert.equal(captured.scrappaRequests.length, 2);
    assert.equal(captured.datasetItems.length, 1);
    assert.equal(captured.chargedEvents.length, 1);
    assert.deepEqual(captured.chargedEvents[0], { eventName: 'quote-result', count: 1 });
    assert.equal(captured.outputs.length, 1);
    assert.deepEqual(captured.outputs[0], upstreamResponse);
    assert.equal(captured.datasetItems[0].symbol, 'AAPL');
    assert.equal(captured.datasetItems[0].current_price, 198.53);
    assert.deepEqual(captured.datasetItems[0].related_tickers, [{ symbol: 'MSFT' }]);
    assert.deepEqual(captured.datasetItems[0].pagination, upstreamResponse.pagination);
    assert.deepEqual(captured.datasetItems[0].upstream_fallback, null);
    assert.deepEqual(captured.datasetItems[0].result_counts, {
        financials: 1,
        news: 1,
        discover_more: 1,
        related_tickers: 1,
    });
    console.log(`Local image smoke passed for ${image}: successful quote/charge/OUTPUT plus exhausted-budget no-write path.`);
} finally {
    await Promise.all([close(apifyServer), close(scrappaServer)]);
}

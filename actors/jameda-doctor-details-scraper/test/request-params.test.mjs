import assert from 'node:assert/strict';
import test from 'node:test';

const requestParamsModule = process.env.TEST_SOURCE === 'src'
    ? '../src/request-params.ts'
    : '../dist/request-params.js';
const {
    buildDoctorDetailsParams,
    buildJamedaDoctorDetailsPlan,
    cleanJamedaDoctorUrl,
    describeJamedaDoctorDetailsRequest,
} = await import(requestParamsModule);

const markusUrl = 'https://www.jameda.de/markus-lietzau-msc/zahnarzt/berlin';

test('builds params for a single Jameda doctor details request', () => {
    const plan = buildJamedaDoctorDetailsPlan({
        doctorUrl: ' https://www.jameda.de/markus-lietzau-msc/zahnarzt/berlin?utm_source=test ',
    });

    assert.deepEqual(plan, {
        doctorUrls: [markusUrl],
        inputFailures: [],
    });
    assert.deepEqual(buildDoctorDetailsParams(markusUrl), {
        doctor_url: markusUrl,
    });
    assert.equal(describeJamedaDoctorDetailsRequest(plan), markusUrl);
});

test('accepts and deduplicates batch doctor URLs', () => {
    const plan = buildJamedaDoctorDetailsPlan({
        doctorUrl: markusUrl,
        doctorUrls: [
            markusUrl,
            '/markus-lietzau-msc/zahnarzt/berlin',
            'markus-lietzau-msc/zahnarzt/berlin/',
            'https://www.jameda.de/anna-example/aerztin/hamburg',
        ],
    });

    assert.deepEqual(plan.doctorUrls, [
        markusUrl,
        'https://www.jameda.de/anna-example/aerztin/hamburg',
    ]);
    assert.equal(describeJamedaDoctorDetailsRequest(plan), '2 doctor URLs');
});

test('accepts comma and newline separated batch doctor URLs', () => {
    const plan = buildJamedaDoctorDetailsPlan({
        doctorUrls: `${markusUrl}, /anna-example/aerztin/hamburg\n/hans-example/orthopaede/muenchen`,
    });

    assert.deepEqual(plan.doctorUrls, [
        markusUrl,
        'https://www.jameda.de/anna-example/aerztin/hamburg',
        'https://www.jameda.de/hans-example/orthopaede/muenchen',
    ]);
});

test('normalizes encoded paths and http URLs', () => {
    assert.equal(
        cleanJamedaDoctorUrl('http://jameda.de/markus-lietzau-msc/zahnarzt/berlin'),
        markusUrl,
    );

    assert.equal(
        cleanJamedaDoctorUrl('/markus-lietzau-msc/zahnarzt/berlin%20mitte'),
        'https://www.jameda.de/markus-lietzau-msc/zahnarzt/berlin%20mitte',
    );
});

test('collects invalid URL entries when valid entries are present', () => {
    const plan = buildJamedaDoctorDetailsPlan({
        doctorUrls: [
            markusUrl,
            'https://example.com/markus-lietzau-msc/zahnarzt/berlin',
            '/search',
            123,
        ],
    });

    assert.deepEqual(plan.doctorUrls, [markusUrl]);
    assert.equal(plan.inputFailures.length, 3);
    assert.match(plan.inputFailures[0].error, /jameda.de domain/);
    assert.match(plan.inputFailures[1].error, /doctor profile path/);
    assert.match(plan.inputFailures[2].error, /doctorUrls must be a string/);
});

test('rejects invalid overall input', () => {
    assert.throws(
        () => buildJamedaDoctorDetailsPlan({}),
        /Provide doctorUrls or doctorUrl/,
    );

    assert.throws(
        () => buildJamedaDoctorDetailsPlan({ doctorUrls: 123 }),
        /doctorUrls must be an array/,
    );

    assert.throws(
        () => buildJamedaDoctorDetailsPlan({ doctorUrl: 'https://example.com/markus-lietzau-msc/zahnarzt/berlin' }),
        /No valid Jameda doctor URLs/,
    );

    assert.throws(
        () => buildJamedaDoctorDetailsPlan({
            doctorUrls: Array.from({ length: 101 }, (_, index) => `/doctor-${index}/zahnarzt/berlin`),
        }),
        /at most 100 doctor URLs/,
    );
});

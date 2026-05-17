import assert from 'node:assert/strict';
import test from 'node:test';

import {
    getArbeitsagenturJobs,
    getArbeitsagenturMetadata,
    getFormattedLocation,
    toArbeitsagenturDatasetJob,
} from '../dist/arbeitsagentur-response.js';

test('returns jobs from wrapped Scrappa response data', () => {
    const jobs = [{ titel: 'Software Entwickler' }];

    assert.deepEqual(getArbeitsagenturJobs({
        success: true,
        data: { stellenangebote: jobs },
    }), jobs);
});

test('returns jobs from top-level response shape', () => {
    const jobs = [{ titel: 'Software Entwickler' }];

    assert.deepEqual(getArbeitsagenturJobs({ stellenangebote: jobs }), jobs);
});

test('falls back to top-level jobs when data exists without jobs', () => {
    const jobs = [{ titel: 'Pflegefachkraft' }];

    assert.deepEqual(getArbeitsagenturJobs({
        data: { maxErgebnisse: 1 },
        stellenangebote: jobs,
    }), jobs);
});

test('returns an empty jobs array for unexpected response shape', () => {
    const originalDebug = console.debug;
    const messages = [];
    console.debug = (message) => messages.push(message);

    try {
        assert.deepEqual(getArbeitsagenturJobs({ success: false }), []);
    } finally {
        console.debug = originalDebug;
    }

    assert.deepEqual(messages, [
        'Unexpected Arbeitsagentur Jobs response shape: expected "data.stellenangebote" or "stellenangebote" array.',
    ]);
});

test('reads metadata from wrapped or top-level response fields', () => {
    assert.deepEqual(getArbeitsagenturMetadata({ data: { maxErgebnisse: 120, page: 2, size: 25 } }), {
        maxErgebnisse: 120,
        page: 2,
        size: 25,
    });
    assert.deepEqual(getArbeitsagenturMetadata({ maxErgebnisse: '1', page: 1, size: 10 }), {
        maxErgebnisse: '1',
        page: 1,
        size: 10,
    });
});

test('formats Arbeitsagentur locations for table summaries', () => {
    assert.equal(getFormattedLocation('Berlin'), 'Berlin');
    assert.equal(getFormattedLocation({ ort: 'Berlin', plz: '10115', region: 'Berlin', land: 'Deutschland' }), '10115, Berlin, Berlin, Deutschland');
    assert.equal(getFormattedLocation(null), undefined);
});

test('adds table-friendly dataset aliases while preserving raw job fields', () => {
    const job = {
        refnr: '12265-399943_JB5100405-S',
        titel: 'Software Entwickler (m/w/d)',
        beruf: 'Softwareentwickler/-in',
        arbeitgeber: 'TechGmbH',
        arbeitsort: {
            ort: 'Berlin',
            plz: '10115',
            region: 'Berlin',
            land: 'Deutschland',
            entfernung: '3',
            koordinaten: { lat: 52.531976, lon: 13.386737 },
        },
        aktuelleVeroeffentlichungsdatum: '2026-03-20',
        eintrittsdatum: '2026-04-01',
        externeUrl: 'https://www.arbeitsagentur.de/jobsuche/jobdetail/10000001',
    };

    assert.deepEqual(toArbeitsagenturDatasetJob(job), {
        ...job,
        title: 'Software Entwickler (m/w/d)',
        occupation: 'Softwareentwickler/-in',
        company_name: 'TechGmbH',
        location_formatted: '10115, Berlin, Berlin, Deutschland',
        location_city: 'Berlin',
        postal_code: '10115',
        region: 'Berlin',
        country: 'Deutschland',
        published_date: '2026-03-20',
        start_date: '2026-04-01',
        job_url: 'https://www.arbeitsagentur.de/jobsuche/jobdetail/10000001',
        reference_number: '12265-399943_JB5100405-S',
        distance_km: '3',
        latitude: 52.531976,
        longitude: 13.386737,
    });
});

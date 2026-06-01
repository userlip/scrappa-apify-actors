import assert from 'node:assert/strict';
import test from 'node:test';

import {
    buildDomainAvailabilityDatasetItem,
    buildDomainAvailabilityFailureItem,
    isPerDomainAvailabilityFailure,
} from '../dist/results.js';
import { ScrappaHttpError, ScrappaTimeoutError } from '../dist/shared/index.js';

test('buildDomainAvailabilityDatasetItem maps a successful Scrappa response', () => {
    const response = {
        domain: 'example.com',
        available: false,
        registered: true,
        status: 'registered',
        confidence: 'high',
        source: 'rdap',
        rdap_url: 'https://rdap.example/domain/example.com',
        rdap_status_code: 200,
        rdap_events: [{ action: 'registration', date: '1995-08-14T04:00:00Z' }],
        nameservers: ['A.IANA-SERVERS.NET'],
    };

    assert.deepEqual(
        buildDomainAvailabilityDatasetItem(response, 'https://example.com/path', 'example.com'),
        {
            success: true,
            input_domain: 'https://example.com/path',
            domain: 'example.com',
            available: false,
            registered: true,
            status: 'registered',
            confidence: 'high',
            source: 'rdap',
            rdap_url: 'https://rdap.example/domain/example.com',
            rdap_status_code: 200,
            rdap_events: [{ action: 'registration', date: '1995-08-14T04:00:00Z' }],
            nameservers: ['A.IANA-SERVERS.NET'],
            message: undefined,
        },
    );
});

test('buildDomainAvailabilityFailureItem preserves per-domain errors', () => {
    assert.deepEqual(
        buildDomainAvailabilityFailureItem(new ScrappaHttpError(422, 'Invalid request'), 'bad_domain.com', 'bad_domain.com'),
        {
            success: false,
            input_domain: 'bad_domain.com',
            domain: 'bad_domain.com',
            available: null,
            registered: null,
            status: 'error',
            confidence: null,
            source: 'scrappa',
            rdap_url: null,
            rdap_status_code: null,
            rdap_events: [],
            nameservers: [],
            error: 'Scrappa API error (422): Invalid request',
            status_code: 422,
        },
    );
});

test('buildDomainAvailabilityFailureItem supports validation failures without normalized domains', () => {
    assert.deepEqual(
        buildDomainAvailabilityFailureItem(new Error('Invalid domain'), 'localhost'),
        {
            success: false,
            input_domain: 'localhost',
            domain: null,
            available: null,
            registered: null,
            status: 'error',
            confidence: null,
            source: 'scrappa',
            rdap_url: null,
            rdap_status_code: null,
            rdap_events: [],
            nameservers: [],
            error: 'Invalid domain',
            status_code: undefined,
        },
    );
});

test('isPerDomainAvailabilityFailure classifies batch-safe Scrappa failures', () => {
    assert.equal(isPerDomainAvailabilityFailure(new ScrappaTimeoutError(1000)), true);
    assert.equal(isPerDomainAvailabilityFailure(new ScrappaHttpError(422, 'Invalid request')), true);
    assert.equal(isPerDomainAvailabilityFailure(new ScrappaHttpError(400, 'Bad request')), true);
    assert.equal(isPerDomainAvailabilityFailure(new ScrappaHttpError(404, 'Not found')), true);
    assert.equal(isPerDomainAvailabilityFailure(new ScrappaHttpError(408, 'Timeout')), false);
    assert.equal(isPerDomainAvailabilityFailure(new ScrappaHttpError(429, 'Rate limited')), false);
    assert.equal(isPerDomainAvailabilityFailure(new ScrappaHttpError(503, 'Unavailable')), false);
    assert.equal(isPerDomainAvailabilityFailure(new ScrappaHttpError(401, 'Unauthorized')), false);
    assert.equal(isPerDomainAvailabilityFailure(new Error('network reset')), false);
});

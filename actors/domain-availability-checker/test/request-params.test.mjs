import assert from 'node:assert/strict';
import test from 'node:test';

import { buildDomainAvailabilityParams } from '../dist/request-params.js';

test('buildDomainAvailabilityParams sends only the Scrappa domain query param', () => {
    assert.deepEqual(
        buildDomainAvailabilityParams('example.com'),
        { domain: 'example.com' },
    );
});

import assert from 'node:assert/strict';
import test from 'node:test';

import { getSavedCount } from '../dist/charging.js';

test('caps saved count to requested dataset item count', () => {
    assert.equal(getSavedCount({ chargedCount: 20 }, 10), 10);
    assert.equal(getSavedCount({ chargedCount: 4 }, 10), 4);
});

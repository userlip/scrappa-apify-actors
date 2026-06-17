import assert from 'node:assert/strict';
import test from 'node:test';

import { getSavedCount } from '../dist/charging.js';

test('caps saved count at requested item count', () => {
    assert.equal(getSavedCount({ chargedCount: 2 }, 5), 2);
    assert.equal(getSavedCount({ chargedCount: 8 }, 5), 5);
});

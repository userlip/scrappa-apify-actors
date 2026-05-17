import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const schema = JSON.parse(readFileSync(new URL('../.actor/input_schema.json', import.meta.url), 'utf8'));

test('does not default page so start offset pagination remains usable', () => {
    assert.equal(schema.properties.page.default, undefined);
    assert.equal(schema.properties.start.default, undefined);
});

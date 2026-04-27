import assert from 'node:assert/strict';
import test from 'node:test';

import { addContactAliases, addSearchResponseAliases } from '../dist/output-aliases.js';

test('adds dataset-friendly address and phone aliases', () => {
    const item = addContactAliases({
        name: 'Example Cafe',
        full_address: '123 Main St, New York, NY',
        phone_numbers: ['+1 555-0100', '+1 555-0101'],
    });

    assert.equal(item.address, '123 Main St, New York, NY');
    assert.equal(item.phone, '+1 555-0100, +1 555-0101');
    assert.equal(item.full_address, '123 Main St, New York, NY');
    assert.deepEqual(item.phone_numbers, ['+1 555-0100', '+1 555-0101']);
});

test('keeps existing address and phone aliases', () => {
    const item = addContactAliases({
        full_address: '123 Main St',
        address: 'Existing address',
        phone_numbers: ['+1 555-0100'],
        phone: 'Existing phone',
    });

    assert.equal(item.address, 'Existing address');
    assert.equal(item.phone, 'Existing phone');
});

test('adds aliases to every response item', () => {
    const response = addSearchResponseAliases({
        items: [
            { name: 'A', full_address: 'Address A', phone_numbers: ['111'] },
            { name: 'B', full_address: 'Address B', phone_numbers: ['222'] },
        ],
        query: 'coffee',
    });

    assert.equal(response.items?.[0].address, 'Address A');
    assert.equal(response.items?.[1].phone, '222');
    assert.equal(response.query, 'coffee');
});

import { Actor } from 'apify';
import type { BaseActorInput } from './types.js';

/**
 * Validates that required fields are present in the input
 */
export function validateRequiredFields<T extends BaseActorInput>(
    input: T,
    requiredFields: (keyof T)[]
): void {
    const missing: string[] = [];

    for (const field of requiredFields) {
        const value = input[field];
        if (value === undefined || value === null || value === '') {
            missing.push(String(field));
        }
    }

    if (missing.length > 0) {
        throw new Error(`Missing required fields: ${missing.join(', ')}`);
    }
}

/**
 * Creates actor input with defaults and validation
 */
export async function createActorInput<T extends BaseActorInput>(): Promise<T> {
    const input = await Actor.getInput<T>();

    if (!input) {
        throw new Error('No input provided');
    }

    return input;
}

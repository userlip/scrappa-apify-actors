import { buildLinkedInProfileOutput, type LinkedInProfileResult } from './results.js';

export interface LinkedInProfileRunStorage {
    pushData(result: LinkedInProfileResult): Promise<void>;
    setValue(key: string, value: unknown): Promise<void>;
}

export interface LinkedInProfileRunSummary {
    requested: number;
    succeeded: number;
    failed: number;
}

export async function publishLinkedInProfileResults(
    results: LinkedInProfileResult[],
    storage: LinkedInProfileRunStorage,
): Promise<LinkedInProfileRunSummary> {
    const successfulResults = results.filter((result) => result.success);
    const failures = results.filter((result) => !result.success);

    for (const result of successfulResults) {
        await storage.pushData(result);
    }

    const summary = {
        requested: results.length,
        succeeded: successfulResults.length,
        failed: failures.length,
    };

    if (results.length === 1) {
        await storage.setValue('OUTPUT', buildLinkedInProfileOutput(results[0]));
    } else {
        await storage.setValue('OUTPUT', summary);
    }

    if (failures.length > 0) {
        await storage.setValue('FAILURES', failures);
    }

    return summary;
}

import { mkdtemp, readFile, rm, stat, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';

import { afterEach, describe, expect, it } from 'vitest';

import {
  defaultSessionStore,
  readSessionStore,
  writeSessionStore,
} from '../session-store.js';
import type { SessionStore } from '../types.js';

const temporaryDirectories: string[] = [];

const createTemporaryDirectory = async (): Promise<string> => {
  const directory = await mkdtemp(path.join(os.tmpdir(), 'midnight-did-session-'));
  temporaryDirectories.push(directory);
  return directory;
};

afterEach(async () => {
  await Promise.all(temporaryDirectories.splice(0).map((directory) => rm(directory, { recursive: true, force: true })));
});

describe('session store persistence', () => {
  const storeWithSeed = (): SessionStore => ({
    ...defaultSessionStore(false),
    profiles: {
      standalone: {
        seed: 'a'.repeat(64),
        unshieldedAddress: 'mn_addr_demo',
        contractAddress: 'f'.repeat(64),
        contractAddresses: ['f'.repeat(64)],
        updatedAt: '2026-01-01T00:00:00.000Z',
      },
    },
  });

  it('does not persist seed material and writes a private file', async () => {
    const filePath = path.join(await createTemporaryDirectory(), 'profiles', 'manager-session.json');

    await writeSessionStore(filePath, storeWithSeed());

    const persisted = await readFile(filePath, 'utf8');
    expect(persisted).not.toContain('a'.repeat(64));
    expect(JSON.parse(persisted).profiles.standalone).toEqual({
      unshieldedAddress: 'mn_addr_demo',
      contractAddress: 'f'.repeat(64),
      contractAddresses: ['f'.repeat(64)],
      updatedAt: '2026-01-01T00:00:00.000Z',
    });
    expect((await stat(filePath)).mode & 0o777).toBe(0o600);
  });

  it('drops legacy plaintext seeds when loading persisted state', async () => {
    const filePath = path.join(await createTemporaryDirectory(), 'manager-session.json');
    const legacy = storeWithSeed();
    await writeSessionStore(filePath, legacy);
    const legacyJson = await readFile(filePath, 'utf8');
    await writeFile(
      filePath,
      legacyJson.replace('"unshieldedAddress"', `"seed":"${'b'.repeat(64)}","unshieldedAddress"`),
      'utf8',
    );

    const loaded = await readSessionStore(filePath, false);
    expect(loaded.profiles.standalone?.seed).toBeUndefined();
    expect(loaded.profiles.standalone?.unshieldedAddress).toBe('mn_addr_demo');

    await writeSessionStore(filePath, loaded);
    expect(await readFile(filePath, 'utf8')).not.toContain('b'.repeat(64));
  });
});

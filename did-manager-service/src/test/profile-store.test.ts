import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';

import { afterEach, describe, expect, it } from 'vitest';

import { loadConfig } from '../config.js';
import { ManagerProfileStore } from '../manager/profile-store.js';
import { defaultSessionStore } from '../session-store.js';

const temporaryDirectories: string[] = [];

afterEach(async () => {
  await Promise.all(temporaryDirectories.splice(0).map((directory) => rm(directory, { recursive: true, force: true })));
});

describe('manager profile migration', () => {
  it('sanitizes migrated sessions before removing legacy files', async () => {
    const dataDir = await mkdtemp(path.join(os.tmpdir(), 'midnight-did-profile-'));
    temporaryDirectories.push(dataDir);
    const config = loadConfig({ DID_MANAGER_DATA_DIR: dataDir });
    const legacySession = {
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
    };
    await writeFile(config.sessionFilePath, JSON.stringify(legacySession), 'utf8');
    await writeFile(config.secretStorePath, 'encrypted legacy secret store', 'utf8');

    const store = new ManagerProfileStore(config, () => 'standalone');
    await store.ensureLoaded();

    await expect(readFile(config.sessionFilePath, 'utf8')).rejects.toThrow();
    await expect(readFile(config.secretStorePath, 'utf8')).rejects.toThrow();
    const migratedSessionPath = path.join(
      dataDir,
      'profiles',
      'standalone',
      'default',
      'manager-session.json',
    );
    const migratedSession = JSON.parse(await readFile(migratedSessionPath, 'utf8')) as {
      profiles: { standalone?: { seed?: string; unshieldedAddress?: string } };
    };
    expect(migratedSession.profiles.standalone).toEqual({
      unshieldedAddress: 'mn_addr_demo',
      contractAddress: 'f'.repeat(64),
      contractAddresses: ['f'.repeat(64)],
      updatedAt: '2026-01-01T00:00:00.000Z',
    });

    await store.saveCurrentProfileState({ unshieldedAddress: 'mn_addr_updated' });
    expect(store.currentProfileState()?.seed).toBeUndefined();

    await store.saveCurrentProfileState({ seed: 'b'.repeat(64) });
    await store.selectProfile('default');
    expect(store.currentProfileState()?.seed).toBe('b'.repeat(64));

    await writeFile(config.sessionFilePath, JSON.stringify(legacySession), 'utf8');
    await writeFile(config.secretStorePath, 'stale legacy secret store', 'utf8');
    await new ManagerProfileStore(config, () => 'standalone').ensureLoaded();
    await expect(readFile(config.sessionFilePath, 'utf8')).rejects.toThrow();
    await expect(readFile(config.secretStorePath, 'utf8')).rejects.toThrow();
  });

  it('migrates legacy state even when another profile directory already exists', async () => {
    const dataDir = await mkdtemp(path.join(os.tmpdir(), 'midnight-did-profile-existing-'));
    temporaryDirectories.push(dataDir);
    const config = loadConfig({ DID_MANAGER_DATA_DIR: dataDir });
    await mkdir(path.join(dataDir, 'profiles', 'standalone', 'other'), { recursive: true });
    await writeFile(
      path.join(dataDir, 'profiles', 'standalone', 'other', 'manager-session.json'),
      JSON.stringify(defaultSessionStore(false)),
      'utf8',
    );
    await writeFile(
      config.sessionFilePath,
      JSON.stringify({
        ...defaultSessionStore(false),
        profiles: {
          standalone: {
            seed: 'c'.repeat(64),
            unshieldedAddress: 'mn_addr_legacy',
            updatedAt: '2026-01-01T00:00:00.000Z',
          },
        },
      }),
      'utf8',
    );
    await writeFile(config.secretStorePath, 'legacy secret store', 'utf8');

    const store = new ManagerProfileStore(config, () => 'standalone');
    await store.ensureLoaded();

    await expect(readFile(config.sessionFilePath, 'utf8')).rejects.toThrow();
    await expect(readFile(config.secretStorePath, 'utf8')).rejects.toThrow();
    expect(store.currentProfileState()?.unshieldedAddress).toBe('mn_addr_legacy');
  });
});

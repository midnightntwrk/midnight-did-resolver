import { randomUUID } from 'node:crypto';
import { chmod, copyFile, mkdir, open, readdir, readFile, rename, rm, writeFile } from 'node:fs/promises';
import path from 'node:path';

import type { NetworkProfile, SessionStore } from './types.js';

type ProfileIndex = {
  version: 1;
  selectedProfiles: Partial<Record<NetworkProfile, string>>;
  legacyMigrationCompleted: Partial<Record<NetworkProfile, boolean>>;
};

const defaultProfileIndex = (): ProfileIndex => ({
  version: 1,
  selectedProfiles: {},
  legacyMigrationCompleted: {},
});

export const defaultSessionStore = (rememberUnlockedSession: boolean): SessionStore => ({
  version: 1,
  rememberUnlockedSession,
  lastProfile: null,
  profiles: {},
});

const PRIVATE_FILE_MODE = 0o600;

const writePrivateAtomic = async (filePath: string, contents: string): Promise<void> => {
  const temporaryPath = `${filePath}.${process.pid}.${randomUUID()}.tmp`;
  let handle: Awaited<ReturnType<typeof open>> | undefined;

  try {
    handle = await open(temporaryPath, 'wx', PRIVATE_FILE_MODE);
    await handle.writeFile(contents, 'utf8');
    await handle.sync();
    await handle.close();
    handle = undefined;
    await rename(temporaryPath, filePath);
    await chmod(filePath, PRIVATE_FILE_MODE);
  } catch (error) {
    if (handle !== undefined) await handle.close().catch(() => undefined);
    await rm(temporaryPath, { force: true }).catch(() => undefined);
    throw error;
  }
};

const persistedProfile = (profileState: SessionStore['profiles'][keyof SessionStore['profiles']]) => {
  if (profileState === undefined) return undefined;
  return {
    unshieldedAddress: profileState.unshieldedAddress,
    contractAddress: profileState.contractAddress,
    contractAddresses:
      Array.isArray(profileState.contractAddresses)
        ? profileState.contractAddresses
        : typeof profileState.contractAddress === 'string'
          ? [profileState.contractAddress]
          : [],
    updatedAt: profileState.updatedAt,
  };
};

export const readSessionStore = async (
  filePath: string,
  rememberUnlockedSession: boolean,
): Promise<SessionStore> => {
  try {
    const raw = await readFile(filePath, 'utf8');
    const parsed = JSON.parse(raw) as Partial<SessionStore>;
    if (parsed.version !== 1 || parsed.profiles === undefined) {
      return defaultSessionStore(rememberUnlockedSession);
    }
    return {
      version: 1,
      rememberUnlockedSession:
        typeof parsed.rememberUnlockedSession === 'boolean'
          ? parsed.rememberUnlockedSession
          : rememberUnlockedSession,
      lastProfile: parsed.lastProfile ?? null,
      profiles: Object.fromEntries(
        Object.entries(parsed.profiles).map(([profile, state]) => [
          profile,
          persistedProfile(state as SessionStore['profiles'][keyof SessionStore['profiles']]),
        ]),
      ) as SessionStore['profiles'],
    };
  } catch {
    return defaultSessionStore(rememberUnlockedSession);
  }
};

export const writeSessionStore = async (filePath: string, store: SessionStore): Promise<void> => {
  await mkdir(path.dirname(filePath), { recursive: true });
  const persisted: SessionStore = {
    ...store,
    profiles: Object.fromEntries(
      Object.entries(store.profiles).map(([profile, state]) => [profile, persistedProfile(state)]),
    ) as SessionStore['profiles'],
  };
  await writePrivateAtomic(filePath, JSON.stringify(persisted, null, 2));
};

export const readProfileIndex = async (filePath: string): Promise<ProfileIndex> => {
  try {
    const raw = await readFile(filePath, 'utf8');
    const parsed = JSON.parse(raw) as Partial<ProfileIndex>;
    if (parsed.version !== 1 || parsed.selectedProfiles === undefined) {
      return defaultProfileIndex();
    }
    return {
      version: 1,
      selectedProfiles: parsed.selectedProfiles,
      legacyMigrationCompleted:
        parsed.legacyMigrationCompleted ?? defaultProfileIndex().legacyMigrationCompleted,
    };
  } catch {
    return defaultProfileIndex();
  }
};

export const writeProfileIndex = async (filePath: string, store: ProfileIndex): Promise<void> => {
  await mkdir(path.dirname(filePath), { recursive: true });
  await writeFile(filePath, JSON.stringify(store, null, 2), 'utf8');
};

export const listProfileNames = async (profilesRootDir: string): Promise<string[]> => {
  try {
    const entries = await readdir(profilesRootDir, { withFileTypes: true });
    return entries
      .filter((entry) => entry.isDirectory())
      .map((entry) => entry.name)
      .sort((left, right) => left.localeCompare(right));
  } catch {
    return [];
  }
};

export const migrateLegacyProfileFile = async (legacyPath: string, targetPath: string): Promise<boolean> => {
  try {
    const raw = await readFile(targetPath, 'utf8');
    if (raw.length > 0) return false;
  } catch {
    try {
      await mkdir(path.dirname(targetPath), { recursive: true });
      await copyFile(legacyPath, targetPath);
      return true;
    } catch {
      // ignore missing legacy files or copy failures; a fresh profile will be created on demand
    }
  }
  return false;
};

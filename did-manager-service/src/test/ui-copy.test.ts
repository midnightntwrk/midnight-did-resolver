import { describe, expect, it } from 'vitest';

import { walletPage } from '../ui.js';

// Regression coverage for #173: the wallet UI must not imply the shared seed
// is persisted to disk. The backend holds it only in process memory (see the
// manager README aligned in #172); after a restart the operator re-enters it
// in provided mode.
describe('wallet page seed copy (#173)', () => {
  it('does not describe the shared seed as persisted or stored', () => {
    expect(walletPage).not.toContain('persist the shared seed');
    expect(walletPage).not.toContain('Reuse the stored profile seed.');
    expect(walletPage).not.toContain('a shared seed is stored');
    expect(walletPage).not.toContain('A shared seed is stored');
  });

  it('describes the prepared seed as held in memory for the current process', () => {
    expect(walletPage).toContain('held only in memory for the current process');
    expect(walletPage).toContain('Reuse the seed loaded for this profile in the current process.');
    expect(walletPage).toContain(
      'Reuse is available for this profile because a shared seed is loaded in the current process.',
    );
    expect(walletPage).toContain(
      'A shared seed is loaded in the current process, but funding is not prepared. Click Prepare funding.',
    );
  });

  it('keeps the restart guidance: back the seed up separately and re-enter it in provided mode', () => {
    expect(walletPage).toContain('back it up separately');
    expect(walletPage).toContain('re-enter it in provided mode after a restart');
    // Pre-existing correct hint for the restart-without-seed state.
    expect(walletPage).toContain(
      'This profile has prepared funding, but its seed is not retained after restart. Re-enter it in provided mode to start a session.',
    );
  });
});

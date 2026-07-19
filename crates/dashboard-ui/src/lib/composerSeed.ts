const COMPOSER_SEED_KEY = "anycode.composer.seed";

/** Persist a one-shot prompt for the home hero composer. */
export function setComposerSeed(text: string): void {
  const trimmed = text.trim();
  if (!trimmed) return;
  try {
    sessionStorage.setItem(COMPOSER_SEED_KEY, trimmed);
  } catch {
    /* private mode / quota */
  }
}

/** Read and clear the composer seed (consume-once). */
export function consumeComposerSeed(): string | null {
  try {
    const value = sessionStorage.getItem(COMPOSER_SEED_KEY);
    if (value == null) return null;
    sessionStorage.removeItem(COMPOSER_SEED_KEY);
    const trimmed = value.trim();
    return trimmed.length > 0 ? trimmed : null;
  } catch {
    return null;
  }
}

/**
 * Validation logic for the prediction pool creation form.
 *
 * Intentionally framework-free — follows the same manual validation pattern
 * used throughout the project (ProfileForm, WaitlistForm, etc.).
 */

/** Supported pool categories. */
export const POOL_CATEGORIES = [
  "Sports",
  "Crypto",
  "Politics",
  "Entertainment",
  "Technology",
  "Finance",
  "Science",
  "Other",
] as const;

export type PoolCategory = (typeof POOL_CATEGORIES)[number];

/** Minimum stake denominations per token (display units, not stroops). */
export const MIN_STAKE: Record<string, number> = {
  XLM: 1,
  STRK: 0.0001,
};

/** Maximum stake denominations per token (display units). */
export const MAX_STAKE: Record<string, number> = {
  XLM: 1_000_000,
  STRK: 1_000_000,
};

/** Minimum number of outcomes a pool must define. */
export const MIN_OUTCOMES = 2;

/** Maximum number of outcomes a pool may define (mirrors the on-chain MAX_OPTIONS_COUNT). */
export const MAX_OUTCOMES = 10;

/** Form field values for pool creation. */
export interface CreatePoolFormValues {
  /** Human-readable pool name. */
  name: string;
  /** Short description of what is being predicted. */
  description: string;
  /** Category bucket this pool belongs to. */
  category: PoolCategory | "";
  /**
   * Labels for each possible outcome (e.g. `["Yes", "No"]` or
   * `["Team A", "Team B", "Draw"]`). Must contain between
   * {@link MIN_OUTCOMES} and {@link MAX_OUTCOMES} non-empty, unique entries.
   */
  outcomes: string[];
  /** Minimum stake required to participate (display units). */
  minStake: string;
  /** Maximum stake per participant (display units). */
  maxStake: string;
  /** Pool close time as an ISO datetime-local string. */
  closeTime: string;
  /** Selected token ID (e.g. "XLM" or "STRK"). */
  token: string;
  /** Creator agrees to pool creation terms. */
  termsAccepted: boolean;
}

/** Per-field validation error messages. */
export type CreatePoolFormErrors = Partial<
  Record<keyof CreatePoolFormValues, string>
> & {
  /** Per-outcome error messages, indexed the same as `values.outcomes`. */
  outcomeErrors?: (string | undefined)[];
};

/** Minimum minutes a pool close time must be in the future. */
const MIN_CLOSE_MINUTES = 30;

/**
 * Validate pool creation form values.
 *
 * Returns an object with only the fields that have errors.
 * An empty object means the form is valid.
 */
export function validateCreatePool(
  values: CreatePoolFormValues,
): CreatePoolFormErrors {
  const errors: CreatePoolFormErrors = {};

  // ── name ──────────────────────────────────────────────────────────────────
  const name = values.name.trim();
  if (!name) {
    errors.name = "Pool name is required.";
  } else if (name.length < 5) {
    errors.name = "Pool name must be at least 5 characters.";
  } else if (name.length > 80) {
    errors.name = "Pool name must be 80 characters or fewer.";
  }

  // ── description ───────────────────────────────────────────────────────────
  const description = values.description.trim();
  if (!description) {
    errors.description = "Description is required.";
  } else if (description.length < 10) {
    errors.description = "Description must be at least 10 characters.";
  } else if (description.length > 300) {
    errors.description = "Description must be 300 characters or fewer.";
  }

  // ── category ──────────────────────────────────────────────────────────────
  if (!values.category) {
    errors.category = "Please select a category.";
  }

  // ── outcomes ──────────────────────────────────────────────────────────────
  const outcomeErrors: (string | undefined)[] = values.outcomes.map(
    (outcome) => {
      const trimmed = outcome.trim();
      if (!trimmed) return "Outcome label is required.";
      if (trimmed.length > 50) return "Must be 50 characters or fewer.";
      return undefined;
    },
  );

  const trimmedOutcomes = values.outcomes.map((o) => o.trim().toLowerCase());
  trimmedOutcomes.forEach((outcome, index) => {
    if (!outcome || outcomeErrors[index]) return;
    const firstDuplicateIndex = trimmedOutcomes.indexOf(outcome);
    if (firstDuplicateIndex !== index) {
      outcomeErrors[index] = "Outcome labels must be unique.";
    }
  });

  if (outcomeErrors.some((e) => e !== undefined)) {
    errors.outcomeErrors = outcomeErrors;
  }

  if (values.outcomes.length < MIN_OUTCOMES) {
    errors.outcomes = `A pool needs at least ${MIN_OUTCOMES} outcomes.`;
  } else if (values.outcomes.length > MAX_OUTCOMES) {
    errors.outcomes = `A pool can have at most ${MAX_OUTCOMES} outcomes.`;
  }

  // ── stake bounds ──────────────────────────────────────────────────────────
  const token = values.token || "XLM";
  const minAllowed = MIN_STAKE[token] ?? 1;
  const maxAllowed = MAX_STAKE[token] ?? 1_000_000;

  const minStakeNum = parseFloat(values.minStake);
  const maxStakeNum = parseFloat(values.maxStake);

  if (!values.minStake) {
    errors.minStake = "Minimum stake is required.";
  } else if (Number.isNaN(minStakeNum) || minStakeNum <= 0) {
    errors.minStake = "Minimum stake must be a positive number.";
  } else if (minStakeNum < minAllowed) {
    errors.minStake = `Minimum stake must be at least ${minAllowed} ${token}.`;
  } else if (minStakeNum > maxAllowed) {
    errors.minStake = `Minimum stake cannot exceed ${maxAllowed.toLocaleString()} ${token}.`;
  }

  if (!values.maxStake) {
    errors.maxStake = "Maximum stake is required.";
  } else if (Number.isNaN(maxStakeNum) || maxStakeNum <= 0) {
    errors.maxStake = "Maximum stake must be a positive number.";
  } else if (maxStakeNum > maxAllowed) {
    errors.maxStake = `Maximum stake cannot exceed ${maxAllowed.toLocaleString()} ${token}.`;
  } else if (
    !Number.isNaN(minStakeNum) &&
    minStakeNum > 0 &&
    maxStakeNum < minStakeNum
  ) {
    errors.maxStake = "Maximum stake must be greater than or equal to the minimum.";
  }

  // ── close time ────────────────────────────────────────────────────────────
  if (!values.closeTime) {
    errors.closeTime = "Pool close time is required.";
  } else {
    const closeMs = new Date(values.closeTime).getTime();
    const nowMs = Date.now();
    const minFutureMs = nowMs + MIN_CLOSE_MINUTES * 60 * 1000;

    if (Number.isNaN(closeMs)) {
      errors.closeTime = "Enter a valid date and time.";
    } else if (closeMs <= nowMs) {
      errors.closeTime = "Close time must be in the future.";
    } else if (closeMs < minFutureMs) {
      errors.closeTime = `Close time must be at least ${MIN_CLOSE_MINUTES} minutes from now.`;
    }
  }

  // ── terms ─────────────────────────────────────────────────────────────────
  if (!values.termsAccepted) {
    errors.termsAccepted = "You must accept the terms to create a pool.";
  }

  return errors;
}

/** Returns the minimum ISO datetime string usable in <input type="datetime-local">. */
export function minCloseTimeValue(): string {
  const d = new Date(Date.now() + MIN_CLOSE_MINUTES * 60 * 1000);
  // datetime-local format: "YYYY-MM-DDTHH:MM"
  return d.toISOString().slice(0, 16);
}

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

/** Form field values for pool creation. */
export interface CreatePoolFormValues {
  /** Human-readable pool name. */
  name: string;
  /** Short description of what is being predicted. */
  description: string;
  /** Category bucket this pool belongs to. */
  category: PoolCategory | "";
  /** Option A label (e.g. "Yes", "Team A wins"). */
  optionA: string;
  /** Option B label (e.g. "No", "Team B wins"). */
  optionB: string;
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
>;

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

  // ── options ───────────────────────────────────────────────────────────────
  const optionA = values.optionA.trim();
  const optionB = values.optionB.trim();

  if (!optionA) {
    errors.optionA = "Option A label is required.";
  } else if (optionA.length < 1) {
    errors.optionA = "Option A must not be empty.";
  } else if (optionA.length > 50) {
    errors.optionA = "Option A must be 50 characters or fewer.";
  }

  if (!optionB) {
    errors.optionB = "Option B label is required.";
  } else if (optionB.length < 1) {
    errors.optionB = "Option B must not be empty.";
  } else if (optionB.length > 50) {
    errors.optionB = "Option B must be 50 characters or fewer.";
  }

  if (optionA && optionB && optionA.toLowerCase() === optionB.toLowerCase()) {
    errors.optionB = "Option B must be different from Option A.";
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

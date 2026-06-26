/**
 * Practical email validation for client-side forms.
 *
 * This intentionally validates the common address shape without attempting to
 * reproduce the full RFC grammar. The backend must still validate addresses
 * before storing or sending mail.
 */
export const EMAIL_PATTERN =
  /^[a-z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?(?:\.[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?)+$/i;

export function isValidEmail(value: string): boolean {
  const email = value.trim();

  return (
    email.length > 0 &&
    email.length <= 254 &&
    !email.includes("..") &&
    EMAIL_PATTERN.test(email)
  );
}

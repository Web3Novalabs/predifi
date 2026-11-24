/**
 * The /components-demo page is an internal UI showcase. It is only served
 * outside production, unless NEXT_PUBLIC_ENABLE_COMPONENTS_DEMO=1 opts in.
 */
export function isComponentsDemoEnabled(
  env: { NODE_ENV?: string; NEXT_PUBLIC_ENABLE_COMPONENTS_DEMO?: string } = {
    NODE_ENV: process.env.NODE_ENV,
    NEXT_PUBLIC_ENABLE_COMPONENTS_DEMO:
      process.env.NEXT_PUBLIC_ENABLE_COMPONENTS_DEMO,
  },
): boolean {
  return (
    env.NODE_ENV !== "production" ||
    env.NEXT_PUBLIC_ENABLE_COMPONENTS_DEMO === "1"
  );
}

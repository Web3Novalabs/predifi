const nextJest = require("next/jest");

const createJestConfig = nextJest({
  dir: "./",
});

/** @type {import('jest').Config} */
const customJestConfig = {
  setupFilesAfterEnv: ["<rootDir>/jest.setup.js"],
  testEnvironment: "jest-environment-jsdom",
  moduleNameMapper: {
    "^@/(.*)$": "<rootDir>/$1",
  },
  testMatch: ["**/__tests__/**/*.[jt]s?(x)", "**/?(*.)+(spec|test).[jt]s?(x)"],

  // ── Coverage (Issue #1812) ────────────────────────────────────────────────
  //
  // `collectCoverageFrom` is explicit rather than left to Jest's default of
  // "files a test happened to import". The default reports coverage only for
  // code the tests already touch, so a component with no test at all is simply
  // absent from the report — which makes the number look far better than the
  // reality and hides exactly the gap this is meant to surface.
  collectCoverageFrom: [
    "app/**/*.{ts,tsx}",
    "components/**/*.{ts,tsx}",
    "lib/**/*.{ts,tsx}",
    "hooks/**/*.{ts,tsx}",
    // Generated, config, and type-only files carry no logic to cover and would
    // just dilute the percentage.
    "!**/*.d.ts",
    "!**/node_modules/**",
    "!**/.next/**",
    "!**/*.stories.{ts,tsx}",
    "!**/layout.tsx",
    "!**/loading.tsx",
    "!**/not-found.tsx",
  ],

  coverageDirectory: "coverage",

  // `text-summary` for the one-line CI figure, `text` for the per-file table
  // the workflow turns into the job summary, `json-summary` for machine reads,
  // and `lcov` for the HTML artifact.
  coverageReporters: ["text", "text-summary", "json-summary", "lcov"],

  // Deliberately NOT set: `coverageThreshold`.
  //
  // 5 test files against 51 components means any honest threshold would fail
  // on day one, and a red check nobody can fix gets marked non-blocking and
  // then ignored. The immediate goal is a visible number that makes the gap
  // impossible to miss. Once it has been driven up, set a floor here at
  // slightly below the achieved figure so it can only go up.
};

module.exports = createJestConfig(customJestConfig);

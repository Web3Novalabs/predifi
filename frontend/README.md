# PrediFi frontend

The PrediFi frontend is a Next.js application for browsing prediction pools,
viewing pool activity, and interacting with the PrediFi platform through its
backend API.

## Prerequisites

- Node.js and [pnpm](https://pnpm.io/installation)
- A running PrediFi backend when using live pool data. The local backend uses
  `http://localhost:8080` by default.

## Getting Started

Install dependencies and start the development server:

```bash
pnpm dev
```

Open [http://localhost:3000](http://localhost:3000) with your browser to see the result.

To install dependencies from a fresh checkout, run:

```bash
pnpm install
```

## Local backend

The frontend points to `http://localhost:8080` by default. To use a backend at
another URL, set `NEXT_PUBLIC_API_BASE_URL` before starting the dev server:

```bash
NEXT_PUBLIC_API_BASE_URL=http://localhost:8080 pnpm dev
```

In PowerShell, use `$env:NEXT_PUBLIC_API_BASE_URL = "http://localhost:8080"`
and then run `pnpm dev`.

## Environment variables

| Variable | Default | Description |
| --- | --- | --- |
| `NEXT_PUBLIC_API_BASE_URL` | `http://localhost:8080` | Base URL of the PrediFi backend API used by the data layer in `lib/api`. |

## Data fetching & caching

Server data is fetched through [SWR](https://swr.vercel.app). Global defaults
live in `components/providers/SWRProvider.tsx` and are tuned for the app's
largely-static data (responses are cached and deduplicated; no revalidation on
window focus or reconnect). Prediction pool data is exposed via the
`usePools()` hook (`lib/hooks/usePools.ts`), backed by the typed client in
`lib/api/pools.ts`.

This project uses [`next/font`](https://nextjs.org/docs/app/building-your-application/optimizing/fonts) to automatically optimize and load [Geist](https://vercel.com/font), a new font family for Vercel.

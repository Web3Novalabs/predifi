import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { fetchPoolById } from "@/lib/api/pools";
import { PoolDetailView } from "@/components/pool/PoolDetailView";
import { formatStake } from "@/lib/stakeFilters";

const SITE_URL = "https://predifi.app";

function outcomeLabel(outcome: number, labels?: string[]): string {
  return labels?.[outcome] ?? `Outcome ${outcome + 1}`;
}

interface PageProps {
  params: Promise<{ id: string }>;
  searchParams: Promise<{ outcome?: string }>;
}

/**
 * Parse the `?outcome=` query param used by shared "my prediction" links
 * (see `PredictionHistoryClient`). Returns `null` when absent or invalid so
 * callers fall back to the generic, pool-wide share/OG behaviour.
 */
function parseOutcomeParam(outcome: string | undefined): number | null {
  if (outcome === undefined) return null;
  const parsed = Number(outcome);
  return Number.isInteger(parsed) && parsed >= 0 ? parsed : null;
}

export async function generateMetadata({
  params,
  searchParams,
}: PageProps): Promise<Metadata> {
  const { id } = await params;
  const { outcome } = await searchParams;
  const pool = await fetchPoolById(id).catch(() => null);

  if (!pool) {
    return { title: "Pool Not Found | PrediFi" };
  }

  const userOutcome = parseOutcomeParam(outcome);
  const userPick = userOutcome !== null
    ? pool.odds.find((o) => o.outcome === userOutcome)
    : undefined;

  const title = userPick
    ? `My prediction: ${outcomeLabel(userPick.outcome, pool.outcome_descriptions)} — ${pool.name} | PrediFi`
    : `${pool.name} | PrediFi`;

  const topOutcome = [...pool.odds].sort((a, b) => b.stake - a.stake)[0];
  const description = userPick
    ? `I predicted ${outcomeLabel(userPick.outcome, pool.outcome_descriptions)} at ${userPick.odds.toFixed(2)}x odds on "${pool.name}" (${pool.category}) — join me on PrediFi.`
    : topOutcome
      ? `${pool.category} prediction pool on PrediFi — ${formatStake(String(pool.total_stake), pool.token)} staked. Leading: ${outcomeLabel(topOutcome.outcome, pool.outcome_descriptions)} at ${topOutcome.odds.toFixed(2)}x odds.`
      : `${pool.category} prediction pool on PrediFi — ${formatStake(String(pool.total_stake), pool.token)} staked.`;

  const url = userPick
    ? `${SITE_URL}/user/pool-market/${pool.pool_id}?outcome=${userPick.outcome}`
    : `${SITE_URL}/user/pool-market/${pool.pool_id}`;
  const image = userPick
    ? `${SITE_URL}/api/og/pool/${pool.pool_id}?outcome=${userPick.outcome}`
    : `${SITE_URL}/api/og/pool/${pool.pool_id}`;

  return {
    title,
    description,
    openGraph: {
      title,
      description,
      url,
      siteName: "PrediFi",
      images: [{ url: image, width: 1200, height: 630, alt: pool.name }],
      locale: "en_US",
      type: "website",
    },
    twitter: {
      card: "summary_large_image",
      title,
      description,
      images: [image],
    },
  };
}

export default async function PoolDetailPage({ params, searchParams }: PageProps) {
  const { id } = await params;
  const { outcome } = await searchParams;
  const pool = await fetchPoolById(id).catch(() => null);

  if (!pool) {
    notFound();
  }

  const userOutcome = parseOutcomeParam(outcome);
  const userPick = userOutcome !== null
    ? pool.odds.find((o) => o.outcome === userOutcome)
    : undefined;

  const shareUrl = userPick
    ? `${SITE_URL}/user/pool-market/${pool.pool_id}?outcome=${userPick.outcome}`
    : `${SITE_URL}/user/pool-market/${pool.pool_id}`;
  const shareTitle = userPick
    ? `I predicted ${outcomeLabel(userPick.outcome, pool.outcome_descriptions)} on "${pool.name}" — predict with me on PrediFi`
    : `${pool.name} — predict on PrediFi`;

  return (
    <PoolDetailView
      poolId={pool.pool_id}
      initialPool={pool}
      shareUrl={shareUrl}
      shareTitle={shareTitle}
      highlightedOutcome={userPick?.outcome}
    />
  );
}

import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { fetchPoolById } from "@/lib/api/pools";
import { ShareButton, CopyButton } from "@/components/ui";

const SITE_URL = "https://predifi.app";

function formatStake(amount: number, token: string): string {
  return `${amount.toLocaleString()} ${token}`;
}

function outcomeLabel(outcome: number, labels?: string[]): string {
  return labels?.[outcome] ?? `Outcome ${outcome + 1}`;
}

interface PageProps {
  params: Promise<{ id: string }>;
}

export async function generateMetadata({
  params,
}: PageProps): Promise<Metadata> {
  const { id } = await params;
  const pool = await fetchPoolById(id).catch(() => null);

  if (!pool) {
    return { title: "Pool Not Found | PrediFi" };
  }

  const title = `${pool.name} | PrediFi`;
  const topOutcome = [...pool.odds].sort((a, b) => b.stake - a.stake)[0];
  const description = topOutcome
    ? `${pool.category} prediction pool on PrediFi — ${formatStake(pool.total_stake, pool.token)} staked. Leading: ${outcomeLabel(topOutcome.outcome, pool.outcome_descriptions)} at ${topOutcome.odds.toFixed(2)}x odds.`
    : `${pool.category} prediction pool on PrediFi — ${formatStake(pool.total_stake, pool.token)} staked.`;
  const url = `${SITE_URL}/user/pool-market/${pool.pool_id}`;
  const image = `${SITE_URL}/api/og/pool/${pool.pool_id}`;

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

export default async function PoolDetailPage({ params }: PageProps) {
  const { id } = await params;
  const pool = await fetchPoolById(id).catch(() => null);

  if (!pool) {
    notFound();
  }

  const shareUrl = `${SITE_URL}/user/pool-market/${pool.pool_id}`;
  const shareTitle = `${pool.name} — predict on PrediFi`;

  return (
    <div className="min-h-screen bg-[#0A0A0A] p-6 lg:p-8">
      <div className="mx-auto max-w-3xl space-y-6">
        <div className="flex items-start justify-between gap-4">
          <div className="space-y-1">
            <p className="text-xs uppercase tracking-wide text-[#7DE3EC]">
              {pool.category}
            </p>
            <h1 className="text-3xl font-bold text-white">{pool.name}</h1>
            <p className="text-sm text-zinc-500">
              {formatStake(pool.total_stake, pool.token)} staked · {pool.state}
            </p>
          </div>
          <div className="flex items-center gap-2 shrink-0">
            <CopyButton
              text={shareUrl}
              size="md"
              className="rounded-md border border-zinc-700 p-2"
              aria-label="Copy pool link"
            />
            <ShareButton url={shareUrl} title={shareTitle} text={pool.name} />
          </div>
        </div>

        <div className="rounded-xl border border-zinc-800 bg-zinc-900 p-5 space-y-3">
          <h2 className="text-sm font-semibold text-white uppercase tracking-wide">
            Outcomes &amp; Odds
          </h2>
          <div className="space-y-2">
            {pool.odds.map((o) => (
              <div
                key={o.outcome}
                className="flex items-center justify-between rounded-lg border border-zinc-800 bg-zinc-800/40 px-4 py-3"
              >
                <span className="text-sm text-zinc-200">
                  {outcomeLabel(o.outcome, pool.outcome_descriptions)}
                  {pool.result != null &&
                    Number(pool.result) === o.outcome && (
                      <span className="ml-2 text-xs font-medium text-emerald-400">
                        Winner
                      </span>
                    )}
                </span>
                <span className="text-sm font-medium text-white">
                  {o.odds.toFixed(2)}x
                  <span className="ml-2 text-zinc-500">
                    ({formatStake(o.stake, pool.token)})
                  </span>
                </span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

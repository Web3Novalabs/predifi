import type { Metadata } from "next";
import { PoolDetailView } from "@/components/pool/PoolDetailView";

type PageProps = {
  params: Promise<{ poolId: string }>;
};

export async function generateMetadata({ params }: PageProps): Promise<Metadata> {
  const { poolId } = await params;
  return {
    title: `Pool #${poolId}`,
    description: `Live prediction pool #${poolId} on PrediFi — real-time stakes, odds, and countdown.`,
  };
}

export default async function PoolDetailPage({ params }: PageProps) {
  const { poolId: raw } = await params;
  const poolId = Number(raw);

  if (!Number.isFinite(poolId) || poolId < 0) {
    return (
      <div className="min-h-screen bg-[#0A0A0A] p-8 text-zinc-400">
        Invalid pool id.
      </div>
    );
  }

  return <PoolDetailView poolId={poolId} />;
}

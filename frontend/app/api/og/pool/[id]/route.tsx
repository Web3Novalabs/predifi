import { ImageResponse } from "next/og";
import { fetchPoolById } from "@/lib/api/pools";

export const runtime = "edge";

function outcomeLabel(outcome: number, labels?: string[]): string {
  return labels?.[outcome] ?? `Outcome ${outcome + 1}`;
}

export async function GET(
  _req: Request,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;
  const pool = await fetchPoolById(id).catch(() => null);

  if (!pool) {
    return new ImageResponse(
      (
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            width: "100%",
            height: "100%",
            background: "#0A0A0A",
            color: "#fff",
            fontSize: 48,
          }}
        >
          Pool not found
        </div>
      ),
      { width: 1200, height: 630 },
    );
  }

  const topOutcomes = [...pool.odds]
    .sort((a, b) => b.stake - a.stake)
    .slice(0, 3);

  return new ImageResponse(
    (
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          justifyContent: "space-between",
          width: "100%",
          height: "100%",
          padding: 64,
          background: "#0A0A0A",
          color: "#fff",
          fontFamily: "sans-serif",
        }}
      >
        <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
          <div style={{ fontSize: 28, color: "#7DE3EC", textTransform: "uppercase" }}>
            {pool.category} · PrediFi
          </div>
          <div style={{ fontSize: 56, fontWeight: 700, lineHeight: 1.15 }}>
            {pool.name}
          </div>
        </div>

        <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
          {topOutcomes.map((o) => (
            <div
              key={o.outcome}
              style={{
                display: "flex",
                justifyContent: "space-between",
                fontSize: 32,
                borderTop: "1px solid #27272a",
                paddingTop: 16,
              }}
            >
              <span>{outcomeLabel(o.outcome, pool.outcome_descriptions)}</span>
              <span style={{ color: "#37B7C3", fontWeight: 700 }}>
                {o.odds.toFixed(2)}x
              </span>
            </div>
          ))}
        </div>

        <div style={{ display: "flex", fontSize: 28, color: "#71717a" }}>
          {pool.total_stake.toLocaleString()} {pool.token} staked
        </div>
      </div>
    ),
    { width: 1200, height: 630 },
  );
}

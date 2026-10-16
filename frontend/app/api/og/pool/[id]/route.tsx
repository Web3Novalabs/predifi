import { ImageResponse } from "next/og";
import { fetchPoolById } from "@/lib/api/pools";

export const runtime = "edge";

function outcomeLabel(outcome: number, labels?: string[]): string {
  return labels?.[outcome] ?? `Outcome ${outcome + 1}`;
}

export async function GET(
  req: Request,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;
  const pool = await fetchPoolById(id).catch(() => null);

  // Issue #1931: an optional `outcome` query param lets a user's own
  // prediction card highlight the outcome they picked (e.g. shared from
  // `PredictionHistoryClient`), instead of always showing the pool-wide
  // leaderboard of outcomes.
  const outcomeParam = new URL(req.url).searchParams.get("outcome");
  const userOutcome =
    outcomeParam !== null && Number.isInteger(Number(outcomeParam))
      ? Number(outcomeParam)
      : null;

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

  const userPick = userOutcome !== null
    ? pool.odds.find((o) => o.outcome === userOutcome)
    : undefined;

  const topOutcomes = [...pool.odds]
    .sort((a, b) => b.stake - a.stake)
    .slice(0, 3);

  // Make sure the user's own pick is always rendered, even if it isn't
  // one of the top-3 by stake — otherwise a shared "my prediction" card
  // could silently omit the very prediction being shared.
  const displayedOutcomes =
    userPick && !topOutcomes.some((o) => o.outcome === userPick.outcome)
      ? [userPick, ...topOutcomes].slice(0, 4)
      : topOutcomes;

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
          {userPick && (
            <div style={{ fontSize: 30, color: "#FFC857", fontWeight: 600 }}>
              🎯 My prediction: {outcomeLabel(userPick.outcome, pool.outcome_descriptions)} at{" "}
              {userPick.odds.toFixed(2)}x
            </div>
          )}
        </div>

        <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
          {displayedOutcomes.map((o) => {
            const isUserPick = userPick?.outcome === o.outcome;
            return (
              <div
                key={o.outcome}
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  fontSize: 32,
                  borderTop: isUserPick ? "2px solid #FFC857" : "1px solid #27272a",
                  paddingTop: 16,
                }}
              >
                <span>
                  {outcomeLabel(o.outcome, pool.outcome_descriptions)}
                  {isUserPick ? " ⭐" : ""}
                </span>
                <span style={{ color: isUserPick ? "#FFC857" : "#37B7C3", fontWeight: 700 }}>
                  {o.odds.toFixed(2)}x
                </span>
              </div>
            );
          })}
        </div>

        <div style={{ display: "flex", fontSize: 28, color: "#71717a" }}>
          {pool.total_stake.toLocaleString()} {pool.token} staked
        </div>
      </div>
    ),
    { width: 1200, height: 630 },
  );
}

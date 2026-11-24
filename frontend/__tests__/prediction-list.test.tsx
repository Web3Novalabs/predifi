import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { SWRConfig } from "swr";
import { PredictionList } from "@/components/dashboard/PredictionList";
import {
  fetchUserPredictions,
  isActivePrediction,
  type UserPrediction,
} from "@/lib/api/predictions";

jest.mock("@/components/ui/copy-button", () => ({
  CopyButton: () => <button type="button" aria-label="Copy prediction ID" />,
}));

let mockAddress: string | undefined = "GADDRESS";
jest.mock("@/lib/hooks/useCurrentUserAddress", () => ({
  useCurrentUserAddress: () => mockAddress,
}));

jest.mock("@/lib/api/predictions", () => ({
  ...jest.requireActual("@/lib/api/predictions"),
  fetchUserPredictions: jest.fn(),
}));

const mockFetch = fetchUserPredictions as jest.MockedFunction<typeof fetchUserPredictions>;

const FUTURE = new Date(Date.now() + 86_400_000).toISOString();
const PAST = new Date(Date.now() - 86_400_000).toISOString();

function prediction(overrides: Partial<UserPrediction>): UserPrediction {
  return {
    prediction_id: 1,
    pool_id: 10,
    pool_name: "Pool",
    pool_category: "Crypto",
    pool_state: "active",
    pool_end_time: FUTURE,
    pool_total_stake: 50_000_000,
    pool_result: null,
    user_outcome: 1,
    user_amount: 10_000_000,
    prediction_created_at: PAST,
    is_winning_outcome: null,
    ...overrides,
  };
}

function respond(predictions: UserPrediction[]) {
  mockFetch.mockResolvedValue({
    address: "GADDRESS",
    predictions,
    limit: 20,
    offset: 0,
    total_predictions: predictions.length,
  });
}

function renderList() {
  return render(
    <SWRConfig value={{ provider: () => new Map(), dedupingInterval: 0 }}>
      <PredictionList />
    </SWRConfig>,
  );
}

describe("PredictionList", () => {
  beforeEach(() => {
    mockAddress = "GADDRESS";
    mockFetch.mockReset();
  });

  it("splits fetched predictions into active and past tabs", async () => {
    respond([
      prediction({ prediction_id: 1, pool_name: "Open pool" }),
      prediction({
        prediction_id: 2,
        pool_name: "Settled pool",
        pool_state: "settled",
        pool_end_time: PAST,
        is_winning_outcome: true,
      }),
    ]);
    renderList();

    expect(await screen.findByText("Open pool")).toBeInTheDocument();
    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining("/api/v1/users/GADDRESS/predictions"),
    );
    expect(screen.queryByText("Settled pool")).not.toBeVisible();
    expect(screen.getByRole("tab", { name: /Active Prediction/ })).toHaveTextContent("1");

    fireEvent.click(screen.getByRole("tab", { name: "Past Predictions" }));
    expect(screen.getByText("Settled pool")).toBeVisible();
    expect(screen.getByText("Won")).toBeInTheDocument();
  });

  it("shows a loading state", () => {
    mockFetch.mockReturnValue(new Promise(() => {}));
    renderList();
    expect(screen.getAllByRole("status")[0]).toHaveTextContent("Loading predictions");
  });

  it("shows an empty state", async () => {
    respond([]);
    renderList();
    expect(await screen.findByText("No active predictions yet")).toBeInTheDocument();
    expect(screen.getByText("No past predictions found")).toBeInTheDocument();
  });

  it("shows an error state and retries", async () => {
    mockFetch.mockRejectedValueOnce(new Error("boom"));
    renderList();
    const retry = await screen.findAllByRole("button", { name: "Try again" });

    respond([prediction({ pool_name: "Recovered" })]);
    fireEvent.click(retry[0]);
    await waitFor(() => expect(screen.getByText("Recovered")).toBeInTheDocument());
  });

  it("prompts to connect a wallet when none is connected", () => {
    mockAddress = undefined;
    renderList();
    expect(screen.getAllByText(/Connect your wallet/)[0]).toBeInTheDocument();
    expect(mockFetch).not.toHaveBeenCalled();
  });

  it("keeps tab keyboard navigation", async () => {
    respond([]);
    renderList();
    await screen.findByText("No active predictions yet");

    const activeTab = screen.getByRole("tab", { name: /Active Prediction/ });
    fireEvent.keyDown(activeTab, { key: "ArrowRight" });
    expect(screen.getByRole("tab", { name: "Past Predictions" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
  });
});

describe("isActivePrediction", () => {
  it("requires an active pool whose end time is in the future", () => {
    expect(isActivePrediction(prediction({}))).toBe(true);
    expect(isActivePrediction(prediction({ pool_end_time: PAST }))).toBe(false);
    expect(isActivePrediction(prediction({ pool_state: "settled" }))).toBe(false);
  });
});

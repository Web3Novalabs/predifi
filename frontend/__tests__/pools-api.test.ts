import { fetchPoolDetail } from "@/lib/api/pools";

let mockFetch: jest.Mock;

const poolPayload = {
  pool_id: 7,
  name: "Will it rain?",
  category: "Weather",
  total_stake: 0,
  end_time: "2030-01-01T00:00:00Z",
  created_at: "2025-01-01T00:00:00Z",
  state: "open",
  creator: "GABC123456789",
  token: "XLM",
  result: null,
  odds: [],
  outcome_descriptions: ["Yes", "No"],
};

describe("fetchPoolDetail", () => {
  beforeEach(() => {
    mockFetch = jest.fn();
    Object.defineProperty(global, "fetch", {
      configurable: true,
      value: mockFetch,
    });
  });

  afterEach(() => {
    Reflect.deleteProperty(global, "fetch");
  });

  it("preserves tags returned by the detail endpoint", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: async () => ({
        status: "success",
        data: { ...poolPayload, tags: ["weather", "local"] },
      }),
    } as Response);

    await expect(fetchPoolDetail("/api/v1/pools/7")).resolves.toMatchObject({
      pool_id: 7,
      tags: ["weather", "local"],
    });
  });

  it("defaults tags to an empty array when the endpoint omits them", async () => {
    mockFetch.mockResolvedValue({
      ok: true,
      json: async () => ({ status: "success", data: poolPayload }),
    } as Response);

    await expect(fetchPoolDetail("/api/v1/pools/7")).resolves.toMatchObject({
      pool_id: 7,
      tags: [],
    });
  });
});

import { cn } from "@/lib/utils";

describe("cn", () => {
  it("merges class name strings", () => {
    expect(cn("flex", "items-center", "gap-2")).toBe(
      "flex items-center gap-2"
    );
  });

  it("drops falsy values", () => {
    expect(cn("block", false && "hidden", null, undefined, "", "mt-2")).toBe(
      "block mt-2"
    );
  });

  it("lets a later Tailwind class win over an earlier conflicting one", () => {
    expect(cn("p-2", "p-4")).toBe("p-4");
  });
});

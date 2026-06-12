// R-D3 — React Testing Library + user-event component test for the review-UI
// approve/reject interaction (the UI seam DESIGN/STORY). Full WebDriver E2E is
// deferred; the end-to-end journey is proven by the Rust command-level STORY (L5).

import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ReviewPanel } from "./ReviewPanel";
import type { Bullet } from "./commands";

const bullets: Bullet[] = [
  { id: "exp_1_0_b0", description: "Cut p99 API latency by 38%", role: "Backend Engineer" },
  { id: "exp_1_0_b1", description: "Introduced contract tests", role: "Backend Engineer" },
];

describe("ReviewPanel", () => {
  it("shows each bullet with its evidence id (ledger is visible)", () => {
    render(<ReviewPanel bullets={bullets} commands={{ setDecision: vi.fn() }} />);
    expect(screen.getByText(/Cut p99 API latency/)).toBeInTheDocument();
    expect(screen.getByText(/evidence: exp_1_0_b0/)).toBeInTheDocument();
  });

  it("defaults all bullets to approved", () => {
    render(<ReviewPanel bullets={bullets} commands={{ setDecision: vi.fn() }} />);
    expect(screen.getByText(/2\/2 approved/)).toBeInTheDocument();
  });

  it("rejecting a bullet calls setDecision(id,false) and updates the count", async () => {
    const setDecision = vi.fn().mockResolvedValue(undefined);
    const user = userEvent.setup();
    render(<ReviewPanel bullets={bullets} commands={{ setDecision }} />);

    await user.click(screen.getByLabelText("Reject exp_1_0_b0"));

    expect(setDecision).toHaveBeenCalledWith("exp_1_0_b0", false);
    expect(screen.getByText(/1\/2 approved/)).toBeInTheDocument();
    expect(screen.getByTestId("bullet-exp_1_0_b0")).toHaveAttribute("data-decision", "rejected");
  });

  it("re-approving a rejected bullet calls setDecision(id,true)", async () => {
    const setDecision = vi.fn().mockResolvedValue(undefined);
    const user = userEvent.setup();
    render(<ReviewPanel bullets={bullets} commands={{ setDecision }} />);

    await user.click(screen.getByLabelText("Reject exp_1_0_b0"));
    await user.click(screen.getByLabelText("Approve exp_1_0_b0"));

    expect(setDecision).toHaveBeenLastCalledWith("exp_1_0_b0", true);
    expect(screen.getByText(/2\/2 approved/)).toBeInTheDocument();
  });

  it("export is disabled only when every bullet is rejected", async () => {
    const setDecision = vi.fn().mockResolvedValue(undefined);
    const onExport = vi.fn();
    const user = userEvent.setup();
    render(<ReviewPanel bullets={bullets} commands={{ setDecision }} onExport={onExport} />);

    const exportBtn = screen.getByLabelText("Export two PDFs");
    expect(exportBtn).toBeEnabled();
    await user.click(screen.getByLabelText("Reject exp_1_0_b0"));
    await user.click(screen.getByLabelText("Reject exp_1_0_b1"));
    expect(exportBtn).toBeDisabled();
  });

  it("export triggers onExport when bullets remain approved", async () => {
    const onExport = vi.fn();
    const user = userEvent.setup();
    render(
      <ReviewPanel bullets={bullets} commands={{ setDecision: vi.fn() }} onExport={onExport} />,
    );
    await user.click(screen.getByLabelText("Export two PDFs"));
    expect(onExport).toHaveBeenCalledOnce();
  });
});

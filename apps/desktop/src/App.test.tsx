// R-CVI-10 (UI seam) — the onboarding "Import résumé (PDF/DOCX)" option. The new
// option sits ALONGSIDE the existing "Import master CV" (JSON) button; importing a
// résumé file calls importResume(bytes, kind), then routes the returned review JSON
// through the existing importMasterCv install path and advances to the paste step.

import { describe, it, expect, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import type { Commands } from "./commands";

// jsdom's File does not implement arrayBuffer(); attach a deterministic one so the
// component's `await file.arrayBuffer()` resolves (the browser/Tauri runtime has it).
function fileWith(bytes: number[], name: string): File {
  const file = new File([new Uint8Array(bytes)], name);
  Object.defineProperty(file, "arrayBuffer", {
    value: () => Promise.resolve(new Uint8Array(bytes).buffer),
  });
  return file;
}

function stubCommands(over: Partial<Commands> = {}): Commands {
  return {
    importMasterCv: vi.fn().mockResolvedValue(undefined),
    importResume: vi.fn().mockResolvedValue('{"schemaVersion":"1.0.0","person":{"name":"Devin Voss"},"experience":[]}'),
    parseJob: vi.fn().mockResolvedValue(undefined),
    computeCoverage: vi.fn().mockResolvedValue({
      mustHave: [],
      niceToHave: [],
      mustHaveCoverage: 0.5,
      niceHaveCoverage: 0.5,
      fitScore: 0.5,
    }),
    tailoredBullets: vi.fn().mockResolvedValue([
      { id: "exp_1_0_b0", description: "Cut p99 API latency by 38%", role: "Backend Engineer" },
    ]),
    setDecision: vi.fn().mockResolvedValue(undefined),
    setAdvocateEnabled: vi.fn().mockResolvedValue(undefined),
    exportApplication: vi
      .fn()
      .mockResolvedValue({ cvPdfLen: 1, coverLetterPdfLen: 1, aiUsed: false }),
    // Item #6: ATS + keyword commands the review-step panels invoke.
    atsReport: vi.fn().mockResolvedValue({
      checks: [
        { id: "ColumnReliance", status: "Warn", message: "multi-column" },
        { id: "UnusualFont", status: "Pass", message: "liberation" },
      ],
    }),
    keywordCoverage: vi.fn().mockResolvedValue({
      found: [{ keyword: "Python", class: "MustHave", evidenceIds: ["exp_1_0"] }],
      missing: [{ keyword: "Cobol", class: "MustHave", evidenceIds: [] }],
    }),
    ...over,
  };
}

describe("App onboarding — résumé import option", () => {
  it("offers the résumé option alongside the JSON import", () => {
    render(<App commands={stubCommands()} />);
    expect(screen.getByLabelText("Import master CV")).toBeInTheDocument();
    expect(screen.getByLabelText("Import résumé (PDF/DOCX)")).toBeInTheDocument();
  });

  it("importing a résumé calls importResume then installs the review JSON and advances", async () => {
    const importResume = vi
      .fn()
      .mockResolvedValue('{"schemaVersion":"1.0.0","person":{"name":"Devin Voss"},"experience":[]}');
    const importMasterCv = vi.fn().mockResolvedValue(undefined);
    const user = userEvent.setup();
    render(<App commands={stubCommands({ importResume, importMasterCv })} />);

    const file = fileWith([1, 2, 3], "resume.docx");
    const input = screen.getByTestId("resume-file-input") as HTMLInputElement;
    await user.upload(input, file);

    await waitFor(() => expect(importResume).toHaveBeenCalledOnce());
    // bytes transported as a number[]; kind inferred from extension
    const [bytes, kind] = importResume.mock.calls[0];
    expect(Array.isArray(bytes)).toBe(true);
    expect(kind).toBe("docx");
    // review JSON routed through the existing install path
    await waitFor(() =>
      expect(importMasterCv).toHaveBeenCalledWith(
        expect.stringContaining("Devin Voss"),
      ),
    );
    // advanced past import (the JSON button is no longer shown)
    await waitFor(() =>
      expect(screen.queryByLabelText("Import résumé (PDF/DOCX)")).not.toBeInTheDocument(),
    );
  });

  it("infers pdf kind from a .pdf filename", async () => {
    const importResume = vi.fn().mockResolvedValue('{"schemaVersion":"1.0.0","person":{},"experience":[]}');
    const user = userEvent.setup();
    render(<App commands={stubCommands({ importResume })} />);
    await user.upload(
      screen.getByTestId("resume-file-input") as HTMLInputElement,
      fileWith([1], "cv.pdf"),
    );
    await waitFor(() => expect(importResume.mock.calls[0][1]).toBe("pdf"));
  });

  it("surfaces a typed import error without advancing", async () => {
    const importResume = vi.fn().mockRejectedValue("unsupported résumé kind: xlsx");
    const user = userEvent.setup();
    render(<App commands={stubCommands({ importResume })} />);
    await user.upload(
      screen.getByTestId("resume-file-input") as HTMLInputElement,
      fileWith([1], "resume.docx"),
    );
    await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent(/Import failed/));
    // still on the import step
    expect(screen.getByLabelText("Import résumé (PDF/DOCX)")).toBeInTheDocument();
  });
});

// Item #3 (R-ADV-13) — the Applicant Advocate opt-in toggle + "AI was used" badge.
describe("App — Applicant Advocate opt-in (item #3)", () => {
  // Navigate App to the review step: import master CV → paste JD → review.
  async function toReview(over: Partial<Commands> = {}) {
    const commands = stubCommands(over);
    const user = userEvent.setup();
    render(<App commands={commands} />);
    await user.click(screen.getByLabelText("Import master CV"));
    await user.click(screen.getByLabelText("Paste JD"));
    await waitFor(() =>
      expect(
        screen.getByLabelText("Use Applicant Advocate (AI) to rewrite bullets"),
      ).toBeInTheDocument(),
    );
    return { commands, user };
  }

  it("the advocate toggle is off by default in the review step", async () => {
    await toReview();
    const toggle = screen.getByLabelText(
      "Use Applicant Advocate (AI) to rewrite bullets",
    ) as HTMLInputElement;
    expect(toggle).not.toBeChecked();
  });

  it("turning the toggle on calls setAdvocateEnabled(true)", async () => {
    const setAdvocateEnabled = vi.fn().mockResolvedValue(undefined);
    const { user } = await toReview({ setAdvocateEnabled });
    await user.click(
      screen.getByLabelText("Use Applicant Advocate (AI) to rewrite bullets"),
    );
    expect(setAdvocateEnabled).toHaveBeenCalledWith(true);
    expect(
      screen.getByLabelText("Use Applicant Advocate (AI) to rewrite bullets"),
    ).toBeChecked();
  });

  it("shows the 'AI was used' badge after an export that returns aiUsed true", async () => {
    const exportApplication = vi
      .fn()
      .mockResolvedValue({ cvPdfLen: 1, coverLetterPdfLen: 1, aiUsed: true });
    const { user } = await toReview({ exportApplication });
    await user.click(screen.getByLabelText("Export two PDFs"));
    await waitFor(() =>
      expect(screen.getByLabelText("AI was used")).toBeInTheDocument(),
    );
  });

  it("does NOT show the 'AI was used' badge when aiUsed is false", async () => {
    const exportApplication = vi
      .fn()
      .mockResolvedValue({ cvPdfLen: 1, coverLetterPdfLen: 1, aiUsed: false });
    const { user } = await toReview({ exportApplication });
    await user.click(screen.getByLabelText("Export two PDFs"));
    await waitFor(() =>
      expect(screen.getByText(/Exported cv.pdf/)).toBeInTheDocument(),
    );
    expect(screen.queryByLabelText("AI was used")).not.toBeInTheDocument();
  });
});

// Item #6 — CV template selector + ATS panel + keyword panel in the review step.
describe("App — item #6 templates / ATS / keyword (review step)", () => {
  async function toReview(over: Partial<Commands> = {}) {
    const commands = stubCommands(over);
    const user = userEvent.setup();
    render(<App commands={commands} />);
    await user.click(screen.getByLabelText("Import master CV"));
    await user.click(screen.getByLabelText("Paste JD"));
    await waitFor(() =>
      expect(screen.getByLabelText("CV template")).toBeInTheDocument(),
    );
    return { commands, user };
  }

  it("offers a template selector defaulting to classic", async () => {
    await toReview();
    const select = screen.getByLabelText("CV template") as HTMLSelectElement;
    expect(select.value).toBe("classic");
  });

  it("export passes the selected template to exportApplication", async () => {
    const exportApplication = vi
      .fn()
      .mockResolvedValue({ cvPdfLen: 1, coverLetterPdfLen: 1, aiUsed: false });
    const { user } = await toReview({ exportApplication });
    await user.selectOptions(screen.getByLabelText("CV template"), "compact");
    await user.click(screen.getByLabelText("Export two PDFs"));
    await waitFor(() =>
      expect(exportApplication).toHaveBeenCalledWith("compact"),
    );
  });

  it("re-queries the ATS report when the template changes", async () => {
    const atsReport = vi.fn().mockResolvedValue({
      checks: [{ id: "ColumnReliance", status: "Pass", message: "single-column" }],
    });
    const { user } = await toReview({ atsReport });
    await waitFor(() => expect(atsReport).toHaveBeenCalledWith("classic"));
    await user.selectOptions(screen.getByLabelText("CV template"), "compact");
    await waitFor(() => expect(atsReport).toHaveBeenCalledWith("compact"));
  });

  it("renders ATS pass/warn rows", async () => {
    await toReview();
    await waitFor(() =>
      expect(screen.getByTestId("ats-ColumnReliance")).toHaveAttribute(
        "data-status",
        "Warn",
      ),
    );
    expect(screen.getByTestId("ats-UnusualFont")).toHaveAttribute(
      "data-status",
      "Pass",
    );
  });

  it("renders keyword found/missing rows with locations", async () => {
    await toReview();
    await waitFor(() =>
      expect(screen.getByTestId("kw-found-Python")).toBeInTheDocument(),
    );
    expect(screen.getByTestId("kw-found-Python")).toHaveTextContent("exp_1_0");
    expect(screen.getByTestId("kw-missing-Cobol")).toBeInTheDocument();
  });
});

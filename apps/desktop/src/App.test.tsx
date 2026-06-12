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
    computeCoverage: vi.fn(),
    tailoredBullets: vi.fn(),
    setDecision: vi.fn().mockResolvedValue(undefined),
    exportApplication: vi.fn(),
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

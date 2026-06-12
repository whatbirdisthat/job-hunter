// Item #5 — RTL + user-event component test for the tracker board / call-sheet / contact
// panel. LOCAL ONLY (the `ui` CI job is continue-on-error, issue #2); the end-to-end journey
// is proven by the Rust command-level STORY (L5) in the blocking rust-workspace job.

import { describe, it, expect, vi } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { TrackerBoard } from "./TrackerBoard";
import type { CallSheetRow, TrackedApplication, TrackerDate } from "./commands";

const today: TrackerDate = { year: 2026, month: 3, day: 20 };

const apps: TrackedApplication[] = [
  {
    id: "ap_0",
    job: { title: "Senior Archivist", company: "Northwind Archives" },
    documentIds: [],
    state: "Discovered",
    submitted: null,
    contactId: null,
    notes: [],
  },
];

function baseCommands(over: Partial<Parameters<typeof TrackerBoard>[0]["commands"]> = {}) {
  return {
    advanceApplication: vi.fn().mockResolvedValue(undefined),
    dailyCallSheet: vi.fn().mockResolvedValue([]),
    addContact: vi.fn().mockResolvedValue("ct_0"),
    linkContact: vi.fn().mockResolvedValue(undefined),
    listApplications: vi.fn().mockResolvedValue(apps),
    ...over,
  };
}

describe("TrackerBoard", () => {
  it("renders a card in the column for its state with an advance control", () => {
    render(<TrackerBoard commands={baseCommands()} applications={apps} today={today} />);
    const card = screen.getByTestId("card-ap_0");
    expect(within(card).getByText("Northwind Archives")).toBeInTheDocument();
    expect(screen.getByLabelText("Advance ap_0 to Tailored")).toBeInTheDocument();
  });

  it("advancing calls advanceApplication(appId, nextState, today)", async () => {
    const commands = baseCommands();
    const user = userEvent.setup();
    render(<TrackerBoard commands={commands} applications={apps} today={today} />);
    await user.click(screen.getByLabelText("Advance ap_0 to Tailored"));
    expect(commands.advanceApplication).toHaveBeenCalledWith("ap_0", "Tailored", today);
  });

  it("an illegal transition surfaces a role=alert error, not a silent failure", async () => {
    const commands = baseCommands({
      advanceApplication: vi.fn().mockRejectedValue(new Error("tracker failed: illegal transition")),
    });
    const user = userEvent.setup();
    render(<TrackerBoard commands={commands} applications={apps} today={today} />);
    await user.click(screen.getByLabelText("Advance ap_0 to Tailored"));
    expect(await screen.findByRole("alert")).toHaveTextContent(/illegal transition/);
  });

  it("building the call sheet renders every brief field per row", async () => {
    const row: CallSheetRow = {
      applicationId: "ap_0",
      company: "Northwind Archives",
      role: "Senior Archivist",
      applicationDate: { year: 2026, month: 3, day: 16 },
      followUpWindow: { opensDay: 3, closesDay: 5 },
      contact: { name: "Robin Quill", org: "Northwind Archives", channel: "LinkedIn" },
      suggestedChannel: "LinkedIn",
      nextAction: "FirstFollowUp",
      draftMessage: "Hi Robin Quill, I wanted to follow up ...",
      priorityScore: 54,
    };
    const commands = baseCommands({ dailyCallSheet: vi.fn().mockResolvedValue([row]) });
    const user = userEvent.setup();
    render(<TrackerBoard commands={commands} applications={apps} today={today} />);
    await user.click(screen.getByLabelText("Build call sheet"));
    const li = await screen.findByTestId("row-ap_0");
    expect(within(li).getByText(/Northwind Archives/)).toBeInTheDocument();
    expect(within(li).getByText(/Senior Archivist/)).toBeInTheDocument();
    // "Robin Quill" appears in both the contact line and the draft greeting.
    expect(within(li).getAllByText(/Robin Quill/).length).toBeGreaterThanOrEqual(1);
    expect(within(li).getByText(/priority 54/)).toBeInTheDocument();
    expect(within(li).getByText(/I wanted to follow up/)).toBeInTheDocument();
  });

  it("adding a contact then linking it calls addContact and linkContact", async () => {
    const commands = baseCommands();
    const user = userEvent.setup();
    render(<TrackerBoard commands={commands} applications={apps} today={today} />);
    await user.type(screen.getByLabelText("Contact name"), "Robin Quill");
    await user.selectOptions(screen.getByLabelText("Contact channel"), "LinkedIn");
    await user.click(screen.getByLabelText("Add contact"));
    expect(commands.addContact).toHaveBeenCalledWith("Robin Quill", "", "", "LinkedIn");
    await user.click(await screen.findByLabelText("Link ct_0 to ap_0"));
    expect(commands.linkContact).toHaveBeenCalledWith("ap_0", "ct_0");
  });
});

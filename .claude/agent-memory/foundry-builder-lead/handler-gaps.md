---
name: handler-gaps
description: FOUNDRY roster has no dedicated Tauri/SQLCipher handler; the agreed mapping for job-hunter
metadata:
  type: project
---

The FOUNDRY agent roster has **no dedicated `handler-tauri` / SQLCipher value-handler**. For the
job-hunter (Applicant Advocate) build the agreed mapping is: **Rust core + Tauri command layer →
handler-rust**, **React/TS UI → handler-react**.

**Why:** the Tauri surface in slice 1 is a thin command-binding layer over `crates/core` with no
Tauri-specific domain logic, so handler-rust is the closest viable mapping and adequate. Recorded as
FOUNDER finding F-1 / self-improvement flag SI-1, not improvised into a new handler.

**How to apply:** when planning later slices, if the Tauri/IPC/SQLCipher layer grows substantial
native logic, propose a dedicated `handler-tauri` under the KAIZEN covenant rather than overloading
handler-rust silently. See [[crate-dependency-rule]].

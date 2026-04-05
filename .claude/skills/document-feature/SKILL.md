---
name: document-feature
description: Update project documentation (release notes, README, CLAUDE.md, design docs) to reflect recently implemented feature work
disable-model-invocation: true
---

You have just finished implementing a feature or bug fix. Your job is to update all relevant documentation files to reflect the changes.

## Recent Changes

Here is the git context for what was implemented:

**Recent commits:**
!`git log --oneline -20`

**Summary of changes:**
!`git diff main --stat`

**Full diff:**
!`git diff main`

## Instructions

Read each documentation file listed below, then update only the ones that need changes. Be minimal and targeted — only change what's necessary to accurately reflect the new work.

### 1. RELEASE_NOTES.md (always update)

- Add concise, user-facing bullet points under a `## Unreleased` section at the top of the file
- Create the `## Unreleased` section if it doesn't exist (insert it above the first existing version heading)
- Focus on what changed from the user's perspective, not implementation details
- Match the tone and style of existing entries

### 2. README.md (update only if needed)

Read `README.md` and update it only if the feature changes user-facing behavior, such as:
- New or changed CLI flags/options
- New input formats or data sources
- Changed defaults or behavior
- New setup steps or dependencies

Do not update the README for internal refactors or implementation-only changes.

### 3. CLAUDE.md (update only if needed)

Read `CLAUDE.md` and update it only if:
- Module responsibilities changed (new modules, renamed modules, changed purpose)
- Key design decisions were added or changed
- Architecture or data flow changed significantly
- New commands or testing patterns were introduced

Do not update CLAUDE.md for minor changes that don't affect how a developer reasons about the codebase.

### 4. Design docs in docs/ (update only if relevant)

List the files in `docs/` and read any design doc that relates to the implemented feature. If a design doc described the feature that was just implemented:
- Update it to reflect the actual implementation where it deviates from the original design
- Mark planned/future sections as implemented where appropriate
- Correct any details that differ from what was actually built (API shapes, data structures, flag names, etc.)

If no design doc relates to the implemented work, skip this step.

## Guidelines

- Read each file before editing — do not guess at current contents
- Keep changes minimal; do not rewrite sections that are already accurate
- Do not add implementation details to user-facing docs (RELEASE_NOTES.md, README.md)
- Preserve existing formatting and style in each file

# Domain docs

This repository uses a single context: `GLOSSARY.md` at the root and decisions in `docs/adr/`.

## Before exploring

Read `GLOSSARY.md` and the ADRs relevant to the area being explored. For feature behavior, also read `docs/design.md`; for Rust implementation, review, tooling, or CI work, follow `docs/development.md` as required by `AGENTS.md`.

If a referenced glossary or ADR directory does not exist, proceed silently. Create domain documentation lazily through the domain-modeling skill when terms or consequential decisions are resolved.

## Use the glossary's vocabulary

Use the canonical terms from `GLOSSARY.md` in issues, designs, code, tests, and reviews. Respect its avoided synonyms. If a needed concept is absent, reconsider whether it belongs to this domain; bring real vocabulary gaps to domain-modeling.

## Flag ADR conflicts

If a proposal contradicts an existing ADR, identify the decision explicitly and explain why reopening it is justified. Do not silently override it.

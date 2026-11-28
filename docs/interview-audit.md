# Interview answer audit

Scope: reconcile every question already asked in this conversation with the user's answers and the saved design. This checks design agreement, not implementation or runtime behavior.

## Result

All 35 questions have answers: 26 in the original interview, six in the album-credit follow-up, and three in the Rust tooling follow-up. No previously asked design question remains unanswered.

The question numbers overlap across interviews. The groups below identify which interview each number belongs to.

| Interview | Questions | Topics | User answer |
| --- | --- | --- | --- |
| Original | Q1–Q2 | Album-first browsing; Omarchy integration | Yes to each |
| Original | Q3 | Playback after closing the TUI | Explicitly changed the recommendation: closing stops playback |
| Original | Q4 | Read-only local collection scope | Yes |
| Original | Q5–Q9 | Tag fallback; album/queue actions; session restore; navigation; playback controls | Yes to all |
| Original | Q10–Q15 | Collection roots; scanning; search; restored playback position; errors; queue editing | Yes to all |
| Original | Q16–Q22 | Album grouping; duplicate files; shuffle; single session; theme/layout; architecture; installation target | Yes to all |
| Original | Q23–Q26 | Separate editions; unavailable roots; previous/clear behavior; final shared understanding and fixture verification | Yes to all |
| Album credits | Q1–Q3 | Keep albums intact; display track credits; Appears on navigation | Yes to all |
| Album credits | Q4–Q6 | Missing album artist; conflicting album artists; explicit contributors versus combined credit strings | Yes to all when these questions were repeated after the tooling discussion |
| Rust tooling | Q7–Q9 | Default formatting; local/CI check gates; pinned stable toolchain | Yes to all |

The album-credit Q4–Q6 answers were not inferred from the earlier tooling answer. They were explicitly requested again and subsequently answered.

## Saved decisions

- [Design and acceptance criteria](design.md): original interview and album-credit decisions.
- [Glossary](../GLOSSARY.md): agreed music, album-credit, and session vocabulary.
- [Rust development policy](development.md): official style/naming references and agreed tooling rules.
- [Architecture decision](adr/0001-application-owned-playback-session.md): playback ownership and lifecycle rationale.
- [Agent instructions](../AGENTS.md): requires reading the development policy for Rust implementation, review, tooling, or CI work.

## Remaining work, not unanswered questions

- The actual music collection path has not been supplied. Configurable roots and generated MP3 fixtures allow implementation to proceed; real-collection verification remains pending.
- The exact stable Rust release will be selected and pinned at implementation time under the agreed policy.
- Code, toolchain configuration, CI, packaging, and runtime verification have not been performed. Agreement on behavior does not establish that it works yet.

No additional user approval question is introduced by this audit. Revisit consequential new trade-offs if they arise during implementation.

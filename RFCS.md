# RFCs

Significant changes to farol go through the RFC (Request for Comments) process.

## When to Write an RFC

- New subsystems or major features
- Breaking changes to the public API or plugin interface
- Changes to the build output format
- Architecture-level refactors

## Process

1. **Propose** — Open an issue with label `type:rfc` describing the problem and high-level approach.
2. **Write** — Create `rfcs/NNNN-title.md` using the template below.
3. **Discuss** — Allow 7 days minimum for community feedback.
4. **Decide** — Maintainer accepts, rejects, or requests revisions.
5. **Implement** — Once accepted, implementation PRs reference the RFC.

## Template

Copy [`rfcs/0000-template.md`](rfcs/0000-template.md) to `rfcs/NNNN-title.md`
and fill in each section. Every RFC answers, at minimum: Summary, Motivation,
Detailed design, Drawbacks, Alternatives, and Unresolved questions.

## Directory

RFCs live in the `rfcs/` directory at the repo root.

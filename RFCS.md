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

```markdown
# RFC NNNN: Title

## Summary

One paragraph explanation.

## Motivation

Why are we doing this? What problem does it solve?

## Design

Detailed design. Include examples, API sketches, directory structures.

## Alternatives Considered

What other approaches were evaluated and why were they rejected?

## Unresolved Questions

What remains to be figured out during implementation?
```

## Directory

RFCs live in the `rfcs/` directory at the repo root.

# Architecture Decision Records

This directory records the significant technical decisions made on farol, using
[Michael Nygard's ADR format](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions).

An ADR captures a single decision: the context that forced it, the choice made,
and the consequences that follow. ADRs are immutable once accepted — if a
decision is reversed, we add a new ADR that supersedes the old one rather than
editing history.

## When to write one

- Choosing or replacing a core dependency or engine (parser, search, template).
- A decision that constrains the public API, plugin contract, or build output.
- Anything a future contributor would otherwise ask "why on earth is it like
  this?" about.

Lighter, forward-looking design proposals go through the [RFC process](../RFCS.md)
instead. An RFC proposes; an ADR records what was decided.

## Index

| # | Title | Status |
|---|-------|--------|
| [0001](0001-benchmark-suite-and-incremental-rebuild-cost.md) | Adopt a criterion benchmark suite | Accepted |

## Format

Copy [`0000-template.md`](0000-template.md) to `NNNN-short-title.md` and fill it
in. Number sequentially.

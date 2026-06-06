# ADR-0001: Adopt a criterion benchmark suite

- **Status:** accepted
- **Date:** 2026-06-06
- **Deciders:** ferrumio

## Context

farol's README and design docs make concrete speed claims — "10k pages under 3
seconds cold, under 200ms warm", "the browser refreshes in milliseconds". Until
now we had no way to verify any of them, and no way to notice if a change made
the engine slower. Performance was an assertion, not a measurement.

The build pipeline has several plausibly-expensive stages (file walk, hashing,
markdown parse, builtin plugins, search indexing, render, write) and a
persistent dependency graph + cache (`redb`) whose entire reason to exist is to
make warm rebuilds fast. Without numbers we could neither prove the cache pays
off nor see where the time actually goes.

## Decision

We will maintain a [`criterion`](https://github.com/bheisler/criterion.rs)
benchmark suite in `crates/farol-core/benches/`, exercising the public API:

- **build.rs** — full build at 100 and 1000 pages, incremental rebuild with one
  page edited against a warm cache, and a no-plugins build to separate engine
  cost from builtin-plugin cost.
- **markdown.rs** — `markdown::parse` over pages of increasing size.
- **config.rs** — `farol.toml` parsing.

Benchmarks **do not gate pull requests** — shared CI runners are too noisy for
that to be anything but a source of false alarms. Instead a dedicated
`bench.yml` workflow runs them weekly and on demand, archiving the criterion
report as an artifact so we can watch the trend. Gating on regressions is
explicitly deferred to a later decision once we have a stable baseline.

## Consequences

**Good.** We now have a reproducible baseline and can catch regressions before
they ship. Anyone can run `cargo bench -p farol-core` locally. The suite already
earned its keep on day one (see below).

**The headline finding.** The first baseline (local macOS, informational)
exposed a real scaling problem the cache was supposed to prevent:

| Benchmark | Time |
|---|---|
| full build, 100 pages | ~166 ms |
| full build, 1000 pages | ~2.0 s |
| incremental, 100 pages, one edit | ~246 ms |
| **incremental, 1000 pages, one edit** | **~1.9 s** |
| no-plugins build, 100 pages | ~113 ms |

Editing a single file in a 1000-page site costs ~1.9 s — essentially the same
as a full 2.0 s rebuild. The cache correctly skips per-page *rendering*, but
some O(n)-in-total-pages work still runs on every build. The prime suspects are
search (tantivy) re-indexing every page regardless of what changed, and the full
re-walk + re-hash of every source file. This directly contradicts the "<200ms
warm" promise.

**Follow-up work this creates.** Investigate and fix the per-build O(n) cost so
incremental rebuild scales with the size of the change, not the size of the
site. Until that lands, the public "<200ms warm" / "10k pages in 3s" claims are
unproven and should not be advertised as fact. Tracked as the performance
work item.

**Cost.** `criterion` and its transitive deps are added as a dev-dependency
(build-time only; not shipped in the wheel). Benchmarks add CI minutes on the
weekly schedule.

## Alternatives considered

- **`#[bench]` / `cargo bench` built-in** — requires nightly and gives only
  mean ± noise. Criterion is stable-compatible and does statistical analysis,
  outlier detection, and HTML reports.
- **Gating PRs on benchmark regressions now** — rejected as premature: runner
  noise would dominate, and we have no baseline to compare against yet. Revisit
  once the numbers are stable.
- **No benchmarks, rely on manual timing (`farol build --timings`)** — useful
  for ad-hoc inspection but not reproducible, not tracked over time, and easy to
  forget. It complements the suite rather than replacing it.

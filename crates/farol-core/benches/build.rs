//! Full and incremental site builds — the headline performance numbers.
//!
//! These build real sites on a temp dir, so they exercise the whole pipeline:
//! walk, frontmatter, markdown, plugins, graph, cache, render, and write.

use std::fs;

use criterion::{Criterion, criterion_group, criterion_main};
use farol_core::plugins::{ChainedHost, core as builtins};
use farol_core::{BuildOptions, Config, NoOpHost, PluginHost, build_with};
use tempfile::TempDir;

/// One page of representative docs content.
fn page_body(i: usize, total: usize) -> String {
    // Link to the next page so the link resolver and graph have real edges.
    let next = (i + 1) % total;
    format!(
        "---\ntitle: Page {i}\nweight: {i}\n---\n\n\
         # Page {i}\n\n\
         Intro paragraph with **bold** and a [next page](./page-{next}.md).\n\n\
         ## Details\n\n\
         - alpha\n- beta\n- gamma\n\n\
         ```rust\nfn page_{i}() -> u32 {{ {i} }}\n```\n\n\
         > A note worth calling out.\n\n\
         More prose to give the renderer something to chew on.\n"
    )
}

/// Materialize a site of `n` pages under a fresh temp dir.
fn make_site(n: usize) -> TempDir {
    let tmp = TempDir::new().unwrap();
    let docs = tmp.path().join("docs");
    fs::create_dir_all(&docs).unwrap();
    fs::write(docs.join("index.md"), "# Home\n\nWelcome.\n").unwrap();
    for i in 0..n {
        fs::write(docs.join(format!("page-{i}.md")), page_body(i, n)).unwrap();
    }
    tmp
}

fn host() -> ChainedHost {
    let mut hosts: Vec<Box<dyn PluginHost>> = vec![Box::new(NoOpHost)];
    hosts.extend(builtins::all());
    ChainedHost::from_boxes(hosts)
}

/// Cold build: no cache, fresh output. Measures raw build throughput.
fn bench_full_build(c: &mut Criterion) {
    let cfg = Config { site_url: Some("https://example.com".into()), ..Config::default() };
    let h = host();
    let mut group = c.benchmark_group("build/full");
    group.sample_size(10);
    for n in [100usize, 1000] {
        group.bench_function(format!("{n}_pages"), |b| {
            b.iter_batched(
                || make_site(n),
                |site| {
                    let opts = BuildOptions { no_cache: true, ..Default::default() };
                    build_with(&cfg, site.path(), &opts, &h).unwrap()
                },
                criterion::BatchSize::PerIteration,
            );
        });
    }
    group.finish();
}

/// Warm rebuild: one page edited, everything else served from cache. This is
/// the hot path that backs `farol serve`'s live reload.
fn bench_incremental_build(c: &mut Criterion) {
    let cfg = Config { site_url: Some("https://example.com".into()), ..Config::default() };
    let h = host();
    let mut group = c.benchmark_group("build/incremental");
    group.sample_size(10);
    for n in [100usize, 1000] {
        group.bench_function(format!("{n}_pages_one_edit"), |b| {
            b.iter_batched(
                || {
                    // Prime the cache with a full build, then hand the warm
                    // site to the measured closure.
                    let site = make_site(n);
                    let opts = BuildOptions::default();
                    build_with(&cfg, site.path(), &opts, &h).unwrap();
                    site
                },
                |site| {
                    // Edit a single page, then rebuild against the warm cache.
                    let target = site.path().join("docs/page-0.md");
                    let mut body = page_body(0, n);
                    body.push_str("\nEdited for the incremental benchmark.\n");
                    fs::write(&target, body).unwrap();
                    let opts = BuildOptions::default();
                    build_with(&cfg, site.path(), &opts, &h).unwrap()
                },
                criterion::BatchSize::PerIteration,
            );
        });
    }
    group.finish();
}

fn bench_render_only(c: &mut Criterion) {
    // A no-op host isolates the engine cost from the builtin-plugin cost.
    let cfg = Config { site_url: Some("https://example.com".into()), ..Config::default() };
    let mut group = c.benchmark_group("build/no_plugins");
    group.sample_size(10);
    group.bench_function("100_pages", |b| {
        b.iter_batched(
            || make_site(100),
            |site| {
                let opts = BuildOptions { no_cache: true, ..Default::default() };
                build_with(&cfg, site.path(), &opts, &NoOpHost).unwrap()
            },
            criterion::BatchSize::PerIteration,
        );
    });
    group.finish();
}

criterion_group!(benches, bench_full_build, bench_incremental_build, bench_render_only);
criterion_main!(benches);

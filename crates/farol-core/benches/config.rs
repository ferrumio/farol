//! Config (`farol.toml`) parsing.

use std::hint::black_box;
use std::path::Path;

use criterion::{Criterion, criterion_group, criterion_main};
use farol_core::Config;

const SAMPLE: &str = r#"
site_name        = "My Docs"
site_url         = "https://docs.example.com"
site_description = "Docs for the example project."
repo_url         = "https://github.com/example/project"
edit_uri         = "edit/main/docs/"

[theme]
name    = "default"
palette = "slate"
primary = "indigo"
accent  = "pink"

[plugins]
enabled  = ["search", "sitemap", "admonitions", "code-copy"]
disabled = ["rss"]
"#;

fn bench_config(c: &mut Criterion) {
    let path = Path::new("farol.toml");
    c.bench_function("config::from_str", |b| {
        b.iter(|| Config::from_str(black_box(SAMPLE), black_box(path)).unwrap());
    });
}

criterion_group!(benches, bench_config);
criterion_main!(benches);

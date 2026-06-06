//! Markdown rendering, isolated from the rest of the build pipeline.

use std::hint::black_box;
use std::path::Path;

use criterion::{Criterion, criterion_group, criterion_main};
use farol_core::markdown;

/// A page with a representative mix of prose, headings, lists, code, tables,
/// and links — the kind of content the parser sees in real docs.
fn sample_page(sections: usize) -> String {
    let mut s = String::from("# Title\n\nIntro paragraph with **bold**, _italic_, and `code`.\n\n");
    for i in 0..sections {
        s.push_str(&format!("## Section {i}\n\n"));
        s.push_str("Some prose with a [link](./other.md) and more text to parse.\n\n");
        s.push_str("- item one\n- item two\n- item three\n\n");
        s.push_str("```rust\nfn main() {\n    println!(\"hello\");\n}\n```\n\n");
        s.push_str("| col a | col b |\n|-------|-------|\n| 1     | 2     |\n\n");
        s.push_str("> a blockquote line\n\n");
    }
    s
}

fn bench_markdown(c: &mut Criterion) {
    let path = Path::new("bench.md");
    let mut group = c.benchmark_group("markdown::parse");
    for sections in [4usize, 20, 100] {
        let text = sample_page(sections);
        group.throughput(criterion::Throughput::Bytes(text.len() as u64));
        group.bench_function(format!("{sections}_sections"), |b| {
            b.iter(|| markdown::parse(black_box(&text), black_box(path)).unwrap());
        });
    }
    group.finish();
}

criterion_group!(benches, bench_markdown);
criterion_main!(benches);

use tempfile::TempDir;

use farol_core::Cache;

#[test]
fn round_trips_entries() {
    let tmp = TempDir::new().unwrap();
    let cache = Cache::open(tmp.path().join("cache.redb")).unwrap();

    assert!(cache.get("node-a", b"hash1").unwrap().is_none());

    cache.put("node-a", b"hash1", b"output-v1").unwrap();
    let got = cache.get("node-a", b"hash1").unwrap();
    assert_eq!(got.as_deref(), Some(&b"output-v1"[..]));
}

#[test]
fn different_ids_do_not_collide() {
    let tmp = TempDir::new().unwrap();
    let cache = Cache::open(tmp.path().join("cache.redb")).unwrap();

    cache.put("node-a", b"hash", b"value-a").unwrap();
    cache.put("node-b", b"hash", b"value-b").unwrap();

    assert_eq!(cache.get("node-a", b"hash").unwrap().as_deref(), Some(&b"value-a"[..]));
    assert_eq!(cache.get("node-b", b"hash").unwrap().as_deref(), Some(&b"value-b"[..]));
}

#[test]
fn persists_across_reopens() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("cache.redb");
    {
        let cache = Cache::open(&db_path).unwrap();
        cache.put("n", b"h", b"v").unwrap();
    }
    let cache = Cache::open(&db_path).unwrap();
    assert_eq!(cache.get("n", b"h").unwrap().as_deref(), Some(&b"v"[..]));
}

#[test]
fn clear_drops_entries() {
    let tmp = TempDir::new().unwrap();
    let cache = Cache::open(tmp.path().join("cache.redb")).unwrap();
    cache.put("n", b"h", b"v").unwrap();
    cache.clear().unwrap();
    assert!(cache.get("n", b"h").unwrap().is_none());
}

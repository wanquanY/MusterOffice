use mo_video_model::ObjectMap;
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

struct Counted {
    value: usize,
    clones: Arc<AtomicUsize>,
}
impl Clone for Counted {
    fn clone(&self) -> Self {
        self.clones.fetch_add(1, Ordering::Relaxed);
        Self {
            value: self.value,
            clones: self.clones.clone(),
        }
    }
}

#[test]
fn snapshot_of_large_map_and_one_edit_clone_only_the_edited_record() {
    let clones = Arc::new(AtomicUsize::new(0));
    let base: ObjectMap<_, _> = (0..100_000)
        .map(|i| {
            (
                i,
                Counted {
                    value: i,
                    clones: clones.clone(),
                },
            )
        })
        .collect();
    let mut next = base.clone();
    assert_eq!(clones.load(Ordering::Relaxed), 0);
    next.get_mut(&50_000).unwrap().value = 7;
    assert_eq!(clones.load(Ordering::Relaxed), 1);
    assert_eq!(base.get(&50_000).unwrap().value, 50_000);
    assert_eq!(next.get(&50_000).unwrap().value, 7);
    next.remove(&42);
    assert!(base.contains_key(&42));
}

#[test]
fn persistent_map_has_plain_ordered_json_and_can_cross_threads() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<ObjectMap<String, String>>();
    let plain = BTreeMap::from([("b", 2), ("a", 1)]);
    let persistent: ObjectMap<_, _> = plain.iter().map(|(k, v)| (*k, *v)).collect();
    let json = serde_json::to_string(&persistent).unwrap();
    assert_eq!(json, serde_json::to_string(&plain).unwrap());
    let decoded: ObjectMap<String, i32> = serde_json::from_str(&json).unwrap();
    assert_eq!(
        decoded.keys().map(String::as_str).collect::<Vec<_>>(),
        vec!["a", "b"]
    );
}

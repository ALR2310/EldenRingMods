//! A list that is worked out for the whole world and kept for a moment, so the minimap does not
//! check every row's flags on every frame.

use std::time::{Duration, Instant};

pub struct Refreshed<K, T> {
    key: Option<K>,
    at: Option<Instant>,
    items: Vec<T>,
}

impl<K: PartialEq + Copy, T> Refreshed<K, T> {
    pub const fn new() -> Self {
        Refreshed { key: None, at: None, items: Vec::new() }
    }

    /// The items for `key`: the ones kept if they were built for the same key less than `max_age`
    /// ago, else what `build` returns. `build` returns `None` when it cannot yet (then nothing
    /// is kept and the next call asks it again).
    pub fn get(&mut self, key: K, max_age: Duration, build: impl FnOnce() -> Option<Vec<T>>) -> &[T] {
        let fresh = self.key == Some(key) && self.at.is_some_and(|at| at.elapsed() < max_age);
        if !fresh {
            match build() {
                Some(items) => {
                    self.items = items;
                    self.key = Some(key);
                    self.at = Some(Instant::now());
                }
                None => {
                    self.items = Vec::new();
                    self.key = None;
                    self.at = None;
                }
            }
        }
        &self.items
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_are_kept_for_the_same_key_and_rebuilt_for_another() {
        let mut cache = Refreshed::new();
        let mut builds = 0;
        let long = Duration::from_secs(60);
        assert_eq!(cache.get(1, long, || { builds += 1; Some(vec![1]) }), [1]);
        assert_eq!(cache.get(1, long, || { builds += 1; Some(vec![2]) }), [1]);
        assert_eq!(cache.get(2, long, || { builds += 1; Some(vec![3]) }), [3]);
        assert_eq!(builds, 2);
    }

    #[test]
    fn items_are_rebuilt_once_they_are_too_old() {
        let mut cache = Refreshed::new();
        assert_eq!(cache.get(1, Duration::ZERO, || Some(vec![1])), [1]);
        assert_eq!(cache.get(1, Duration::ZERO, || Some(vec![2])), [2]);
    }

    #[test]
    fn a_build_that_cannot_yet_is_asked_again() {
        let mut cache: Refreshed<i32, i32> = Refreshed::new();
        assert!(cache.get(1, Duration::from_secs(60), || None).is_empty());
        assert_eq!(cache.get(1, Duration::from_secs(60), || Some(vec![7])), [7]);
    }
}

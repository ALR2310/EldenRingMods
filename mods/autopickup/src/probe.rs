//! `Logging.PickupProbe`: logs which interactions the mod answered, to find the ID of one it does
//! not know. The hook only pushes into a ring of atomics; the per-frame tick drains it into
//! `Probe: picked <id> <group> xN` (at most once a second per ID), `Probe: skipped <id> <group>`
//! and `Probe: other <id>` (once per ID).

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use shared::logger;

use crate::groups::classify;

const RING_LEN: usize = 256;
/// Set on every written slot, so an empty (zero) slot is never read as ID 0.
const VALID: u64 = 1 << 40;
const HANDLED: u64 = 1 << 32;

const PICKED_INTERVAL: Duration = Duration::from_secs(1);

/// If the tick falls more than [`RING_LEN`] entries behind, the oldest entries are lost.
pub struct Ring {
    slots: [AtomicU64; RING_LEN],
    head: AtomicUsize,
}

impl Ring {
    pub const fn new() -> Self {
        Ring { slots: [const { AtomicU64::new(0) }; RING_LEN], head: AtomicUsize::new(0) }
    }

    pub fn push(&self, id: u32, handled: bool) {
        let at = self.head.fetch_add(1, Ordering::Relaxed);
        let entry = VALID | if handled { HANDLED } else { 0 } | u64::from(id);
        self.slots[at % RING_LEN].store(entry, Ordering::Relaxed);
    }

    pub fn drain(&self, tail: &mut usize) -> Vec<(u32, bool)> {
        let head = self.head.load(Ordering::Relaxed);
        let start = (*tail).max(head.saturating_sub(RING_LEN));
        *tail = head;
        (start..head)
            .filter_map(|at| {
                let entry = self.slots[at % RING_LEN].load(Ordering::Relaxed);
                (entry & VALID != 0).then_some((entry as u32, entry & HANDLED != 0))
            })
            .collect()
    }
}

static RING: Ring = Ring::new();
static ENABLED: AtomicBool = AtomicBool::new(false);

pub fn set_enabled(enabled: bool) {
    ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn record(id: u32, handled: bool) {
    if ENABLED.load(Ordering::Relaxed) {
        RING.push(id, handled);
    }
}

#[derive(Default)]
pub struct Drainer {
    tail: usize,
    seen: HashSet<u32>,
    last_picked: HashMap<u32, Instant>,
}

impl Drainer {
    pub fn tick(&mut self) {
        if !ENABLED.load(Ordering::Relaxed) {
            // Skip what was pushed before the switch was turned off.
            self.tail = RING.head.load(Ordering::Relaxed);
            return;
        }
        let entries = RING.drain(&mut self.tail);
        for line in self.lines(entries, Instant::now()) {
            logger::log(&line);
        }
    }

    fn lines(&mut self, entries: Vec<(u32, bool)>, now: Instant) -> Vec<String> {
        let mut picked: Vec<(u32, usize)> = Vec::new();
        let mut lines = Vec::new();
        for (id, handled) in entries {
            if handled {
                match picked.iter_mut().find(|(seen, _)| *seen == id) {
                    Some((_, count)) => *count += 1,
                    None => picked.push((id, 1)),
                }
            } else if self.seen.insert(id) {
                lines.push(match classify(id) {
                    Some(group) => format!("Probe: skipped {id} {group:?}"),
                    None => format!("Probe: other {id}"),
                });
            }
        }
        for (id, count) in picked {
            if self
                .last_picked
                .get(&id)
                .is_some_and(|last| now.duration_since(*last) < PICKED_INTERVAL)
            {
                continue;
            }
            self.last_picked.insert(id, now);
            let group = classify(id).map_or_else(|| "?".to_string(), |group| format!("{group:?}"));
            lines.push(format!("Probe: picked {id} {group} x{count}"));
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ring_hands_over_what_was_pushed_in_order() {
        let ring = Ring::new();
        let mut tail = 0;
        assert!(ring.drain(&mut tail).is_empty());
        ring.push(7800, true);
        ring.push(6100, false);
        ring.push(0, true);
        assert_eq!(ring.drain(&mut tail), vec![(7800, true), (6100, false), (0, true)]);
        assert!(ring.drain(&mut tail).is_empty(), "each entry is handed over once");
    }

    #[test]
    fn a_ring_that_wrapped_keeps_the_newest() {
        let ring = Ring::new();
        let mut tail = 0;
        for id in 0..(RING_LEN as u32 + 10) {
            ring.push(id, false);
        }
        let drained = ring.drain(&mut tail);
        assert_eq!(drained.len(), RING_LEN);
        assert_eq!(drained.last(), Some(&(RING_LEN as u32 + 9, false)));
    }

    #[test]
    fn picked_ids_are_counted_and_throttled() {
        let mut drainer = Drainer::default();
        let t0 = Instant::now();
        let lines = drainer.lines(vec![(4250, true), (4250, true), (7800, true)], t0);
        assert_eq!(lines, vec!["Probe: picked 4250 Items x2", "Probe: picked 7800 Materials x1"]);
        let again = drainer.lines(vec![(4250, true)], t0 + Duration::from_millis(200));
        assert!(again.is_empty(), "same ID within a second is not logged again");
        let later = drainer.lines(vec![(4250, true)], t0 + Duration::from_secs(2));
        assert_eq!(later, vec!["Probe: picked 4250 Items x1"]);
    }

    #[test]
    fn unhandled_ids_are_reported_once() {
        let mut drainer = Drainer::default();
        let now = Instant::now();
        let lines = drainer.lines(vec![(6100, false), (4300, false), (6100, false)], now);
        assert_eq!(lines, vec!["Probe: other 6100", "Probe: skipped 4300 Chests"]);
        assert!(drainer.lines(vec![(6100, false)], now).is_empty());
    }
}

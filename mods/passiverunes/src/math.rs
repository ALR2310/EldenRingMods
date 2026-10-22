//! The pure parts of the passive-rune tick: milestones, the three amounts and
//! the clock. Nothing here touches the game, so it is unit tested. The
//! formulas and why they are what they are: `docs/rune_math.md`.

/// The game caps held runes at 999,999,999; `saturating_add` alone would only
/// stop at `u32::MAX`.
pub const MAX_HELD_RUNES: u32 = 999_999_999;

pub const DEFAULT_MILESTONES: &str =
    "1800:5000,3600:10000,7200:25000,10800:40000,21600:100000,43200:500000,86400:1000000";

pub struct Milestone {
    pub at_ms: f64,
    /// Negative takes runes away, like every other amount.
    pub bonus: i64,
}

/// Parses `seconds:bonus` pairs separated by commas into milestones sorted by
/// time (so "the next one due" is an advancing index, whatever order the ini
/// lists them in). Entries that don't parse are skipped, so an empty value
/// gives an empty list; a negative time is skipped too, it would fire the
/// instant the player enters the world.
pub fn parse_milestones(spec: &str) -> Vec<Milestone> {
    let mut result: Vec<Milestone> = spec
        .split(',')
        .filter_map(|entry| {
            let (secs, bonus) = entry.trim().split_once(':')?;
            let at_seconds: f64 = secs.trim().parse().ok()?;
            if at_seconds < 0.0 {
                return None;
            }
            Some(Milestone { at_ms: at_seconds * 1000.0, bonus: bonus.trim().parse().ok()? })
        })
        .collect();
    result.sort_by(|a, b| a.at_ms.total_cmp(&b.at_ms));
    result
}

/// `held` after adding `delta` (negative takes runes away), kept within
/// `0..=MAX_HELD_RUNES`.
pub fn add_runes(held: u32, delta: i64) -> u32 {
    (held as i64 + delta).clamp(0, MAX_HELD_RUNES as i64) as u32
}

/// Runes needed to go from `level` to `level + 1`. The game has no param for
/// it: a formula hardcoded in the exe, worked out by the community.
pub fn level_up_cost(level: u32) -> f64 {
    let l = level as f64 + 81.0;
    let x = ((l - 92.0) * 0.02).max(0.0);
    ((x + 0.1) * l * l + 1.0).floor()
}

/// `Rune.Passive.Percent`: `percent`% of `cost` (the next level-up cost),
/// truncated toward 0 but at least 1 rune whenever `percent != 0`, so a tiny
/// percent at a low level never rounds down to nothing.
pub fn percent_amount(cost: f64, percent: f64) -> i64 {
    if percent == 0.0 {
        return 0;
    }
    (((cost * percent.abs() / 100.0).floor() as i64).max(1)) * percent.signum() as i64
}

/// Carried state of `Rune.Passive.Interest` between ticks.
#[derive(Default)]
pub struct InterestState {
    /// Fractional runes left over: 0.1% of 1,500 is 1.5, and truncating every
    /// tick would pay 1 forever instead of compounding. Same sign as the
    /// `Interest` that produced it.
    carry: f64,
    /// Held runes right after the last grant. Held dropping below this means
    /// runes were spent or lost (death), which resets `carry`.
    pub last_held: Option<u32>,
}

impl InterestState {
    /// `Rune.Passive.Interest`: `percent`% of the `held` runes (compound
    /// interest; a negative percent is compound decay), carrying the
    /// fraction over. No cap and no minimum: holding 0 runes earns 0.
    pub fn amount(&mut self, held: u32, percent: f64) -> i64 {
        if percent == 0.0 {
            self.carry = 0.0;
            return 0;
        }
        // Also reset on a sign flip (a reload from +x to -x) so leftover
        // positive fractions don't offset the first negative ticks.
        if self.last_held.is_some_and(|last| held < last) || self.carry * percent < 0.0 {
            self.carry = 0.0;
        }
        let exact = held as f64 * percent / 100.0 + self.carry;
        let runes = exact.trunc();
        self.carry = exact - runes;
        runes as i64
    }
}

/// The two independent clocks of the tick. Both only advance while the player
/// is in the world, and pause (not reset) when they leave it.
#[derive(Default)]
pub struct Clock {
    /// Never resets: drives the milestones.
    session_ms: f64,
    /// Resets each time it reaches `Rune.Passive.Interval`.
    interval_ms: f64,
    next_milestone: usize,
}

impl Clock {
    pub fn advance_session(&mut self, dt_ms: f64) {
        self.session_ms += dt_ms;
    }

    /// The next milestone whose time has been reached, not yet marked done.
    /// Reaching or passing it counts, so a frame hitch or an interval that
    /// doesn't divide the time can't skip a bonus.
    pub fn due_milestone<'a>(&self, milestones: &'a [Milestone]) -> Option<&'a Milestone> {
        milestones.get(self.next_milestone).filter(|m| self.session_ms >= m.at_ms)
    }

    pub fn milestone_done(&mut self) {
        self.next_milestone += 1;
    }

    /// Advances the interval clock and says whether a tick is due. An
    /// interval of 0 (or less) disables the tick and leaves the clock alone.
    pub fn interval_due(&mut self, dt_ms: f64, interval_ms: f64) -> bool {
        if interval_ms <= 0.0 {
            return false;
        }
        self.interval_ms += dt_ms;
        if self.interval_ms < interval_ms {
            return false;
        }
        self.interval_ms = 0.0;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_milestones_parse_sorted() {
        let milestones = parse_milestones(DEFAULT_MILESTONES);
        assert_eq!(milestones.len(), 7);
        assert_eq!((milestones[0].at_ms, milestones[0].bonus), (1_800_000.0, 5_000));
        assert!(milestones.windows(2).all(|w| w[0].at_ms < w[1].at_ms));
    }

    #[test]
    fn milestones_are_sorted_and_bad_entries_skipped() {
        let milestones = parse_milestones(" 3600:10 , 60:-5, x:1, 120, -30:9, 90:y ,");
        let got: Vec<(f64, i64)> = milestones.iter().map(|m| (m.at_ms, m.bonus)).collect();
        assert_eq!(got, vec![(60_000.0, -5), (3_600_000.0, 10)]);
        assert!(parse_milestones("").is_empty());
    }

    #[test]
    fn runes_stay_within_zero_and_the_game_cap() {
        assert_eq!(add_runes(100, 50), 150);
        assert_eq!(add_runes(100, -500), 0);
        assert_eq!(add_runes(999_999_990, 100), MAX_HELD_RUNES);
        assert_eq!(add_runes(0, i64::MIN / 2), 0);
    }

    #[test]
    fn level_up_cost_matches_the_in_game_screen() {
        assert_eq!(level_up_cost(1), 673.0); // level 1 -> 2
        assert_eq!(level_up_cost(100), 61_591.0);
    }

    #[test]
    fn percent_of_the_next_level_is_at_least_one_rune() {
        assert_eq!(percent_amount(673.0, 0.25), 1); // 1.68 floors to 1
        assert_eq!(percent_amount(100.0, 0.1), 1); // 0.1 floors to 0, raised to 1
        assert_eq!(percent_amount(100.0, -0.1), -1);
        assert_eq!(percent_amount(60_000.0, 1.0), 600);
        assert_eq!(percent_amount(60_000.0, 0.0), 0);
    }

    #[test]
    fn interest_carries_the_fraction_instead_of_truncating() {
        let mut state = InterestState::default();
        assert_eq!(state.amount(1_500, 0.1), 1); // 1.5 -> pays 1, carries 0.5
        assert_eq!(state.amount(1_500, 0.1), 2); // 1.5 + 0.5 = 2
        assert_eq!(state.amount(0, 0.1), 0); // holding nothing earns nothing
    }

    #[test]
    fn interest_carry_resets_when_runes_are_spent_or_the_sign_flips() {
        let mut state = InterestState::default();
        state.amount(1_500, 0.1);
        state.last_held = Some(1_500);
        // Runes were spent: the 0.5 carried over is dropped.
        assert_eq!(state.amount(1_000, 0.1), 1);
        // A reload from interest to decay: leftover positive fraction is dropped.
        let mut state = InterestState::default();
        state.amount(1_500, 0.1);
        assert_eq!(state.amount(1_500, -0.1), -1);
        assert_eq!(InterestState::default().amount(1_500, 0.0), 0);
    }

    #[test]
    fn milestones_fire_when_reached_even_after_a_hitch() {
        let milestones = parse_milestones("10:1,20:2,30:3");
        let mut clock = Clock::default();
        clock.advance_session(5_000.0);
        assert!(clock.due_milestone(&milestones).is_none());
        // One big frame passes two thresholds: both are still granted, in order.
        clock.advance_session(20_000.0);
        let mut granted = Vec::new();
        while let Some(m) = clock.due_milestone(&milestones) {
            granted.push(m.bonus);
            clock.milestone_done();
        }
        assert_eq!(granted, vec![1, 2]);
    }

    #[test]
    fn a_milestone_not_marked_done_is_retried() {
        let milestones = parse_milestones("10:7");
        let mut clock = Clock::default();
        clock.advance_session(11_000.0);
        assert_eq!(clock.due_milestone(&milestones).map(|m| m.bonus), Some(7));
        assert_eq!(clock.due_milestone(&milestones).map(|m| m.bonus), Some(7)); // grant failed: still due
    }

    #[test]
    fn the_interval_ticks_once_per_period_and_zero_disables_it() {
        let mut clock = Clock::default();
        assert!(!clock.interval_due(4_000.0, 10_000.0));
        assert!(!clock.interval_due(4_000.0, 10_000.0));
        assert!(clock.interval_due(4_000.0, 10_000.0)); // 12s elapsed
        assert!(!clock.interval_due(4_000.0, 10_000.0)); // reset to 0, then 4s
        assert!(!clock.interval_due(60_000.0, 0.0));
        assert!(!clock.interval_due(60_000.0, -5.0));
    }
}

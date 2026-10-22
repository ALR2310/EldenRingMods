//! Heal amounts and applying them to the main player's HP/FP/Stamina through
//! `CSChrDataModule`. The amounts are pure functions (tested); only
//! [with_stat_mut] touches the game.

use eldenring::cs::WorldChrMan;
use fromsoftware_shared::FromStatic;

#[derive(Clone, Copy)]
pub enum HealField {
    Hp,
    Fp,
    Stamina,
}

/// How much a per-hit heal restores: `flat_amount` plus `percent_fraction` of
/// `max` (0.01 = 1%), at least 1 point when the percent alone would round to
/// 0, clamped to `max`. 0 if it is a no-op or the stat is already full.
fn hit_restore(current: i32, max: i32, flat_amount: i32, percent_fraction: f64) -> i32 {
    let percent_heal = if percent_fraction > 0.0 {
        ((percent_fraction * max as f64) as i32).max(1)
    } else {
        0
    };
    let heal = flat_amount + percent_heal;
    if heal <= 0 || current >= max {
        return 0;
    }
    (current + heal).min(max) - current
}

/// One `Regen.PerTick` heal amount by `value_type`: 0 = `value` flat points,
/// 1 = `value`% of max, 2 = `value`% of the missing amount (heals fast while
/// low, tapers off near full). Percent modes restore at least 1 point while
/// `value` is positive.
fn tick_heal(value_type: i32, value: f64, current: i32, max: i32) -> i32 {
    match value_type {
        1 if value > 0.0 => ((value / 100.0 * max as f64) as i32).max(1),
        2 if value > 0.0 => ((value / 100.0 * (max - current) as f64) as i32).max(1),
        1 | 2 => 0,
        _ => value.round() as i32,
    }
}

/// How much one tick restores: [tick_heal], never past `cap_pct` percent of
/// `max` (0 once already at or above the cap).
fn tick_restore(current: i32, max: i32, value_type: i32, value: f64, cap_pct: f64) -> i32 {
    let cap = (((cap_pct / 100.0) * max as f64) as i32).clamp(0, max);
    if current >= cap {
        return 0;
    }
    let heal = tick_heal(value_type, value, current, max);
    if heal <= 0 {
        return 0;
    }
    (current + heal).min(cap) - current
}

/// Gives `f` mutable access to `field`'s `(current, max)` on the main player.
/// `None` if the player isn't resolved or is dead (nothing is touched).
fn with_stat_mut<R>(field: HealField, f: impl FnOnce(&mut i32, i32) -> R) -> Option<R> {
    let world_chr_man = unsafe { WorldChrMan::instance_mut() }.ok()?;
    let data = &mut world_chr_man.main_player.as_mut()?.chr_ins.modules.data;
    if data.hp <= 0 {
        return None;
    }
    // The max is read first: `data` derefs, so a field can't be borrowed
    // mutably while another is read.
    let (max, current) = match field {
        HealField::Hp => (data.max_hp, &mut data.hp),
        HealField::Fp => (data.max_fp, &mut data.fp),
        HealField::Stamina => (data.max_stamina, &mut data.stamina),
    };
    Some(f(current, max))
}

/// Applies a per-hit heal (`Regen.PerHit`) and returns the amount restored.
pub fn heal_main_player(field: HealField, flat_amount: i32, percent_fraction: f64) -> i32 {
    with_stat_mut(field, |current, max| {
        let restored = hit_restore(*current, max, flat_amount, percent_fraction);
        *current += restored;
        restored
    })
    .unwrap_or(0)
}

/// Applies one `Regen.PerTick` heal and returns the amount restored. `cap_pct`
/// (`Regen.PerTick.Cap`) is a ceiling ticks never restore past, whatever
/// `value_type` is.
pub fn heal_main_player_tick(field: HealField, value_type: i32, value: f64, cap_pct: f64) -> i32 {
    with_stat_mut(field, |current, max| {
        let restored = tick_restore(*current, max, value_type, value, cap_pct);
        *current += restored;
        restored
    })
    .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_restore_is_clamped_and_never_below_one_for_a_percent() {
        assert_eq!(hit_restore(100, 1000, 50, 0.0), 50);
        assert_eq!(hit_restore(990, 1000, 50, 0.0), 10); // clamped to max
        assert_eq!(hit_restore(1000, 1000, 50, 0.0), 0); // already full
        assert_eq!(hit_restore(100, 1000, 0, 0.01), 10); // 1% of max
        assert_eq!(hit_restore(100, 50, 0, 0.001), 0); // current above max: nothing
        assert_eq!(hit_restore(10, 100, 0, 0.001), 1); // percent rounds to 0: still 1
        assert_eq!(hit_restore(100, 1000, 0, 0.0), 0);
        assert_eq!(hit_restore(100, 1000, -5, 0.0), 0);
    }

    #[test]
    fn tick_heal_by_value_type() {
        assert_eq!(tick_heal(0, 12.4, 0, 1000), 12);
        assert_eq!(tick_heal(0, -5.0, 0, 1000), -5); // flat keeps its sign; the caller drops it
        assert_eq!(tick_heal(1, 2.0, 0, 1000), 20); // 2% of max
        assert_eq!(tick_heal(1, 0.01, 0, 1000), 1); // at least 1
        assert_eq!(tick_heal(2, 10.0, 400, 1000), 60); // 10% of the missing 600
        assert_eq!(tick_heal(2, 10.0, 999, 1000), 1);
        assert_eq!(tick_heal(1, 0.0, 0, 1000), 0);
        assert_eq!(tick_heal(2, -1.0, 0, 1000), 0);
    }

    #[test]
    fn tick_restore_never_passes_the_cap() {
        assert_eq!(tick_restore(100, 1000, 0, 50.0, 100.0), 50);
        assert_eq!(tick_restore(480, 1000, 0, 50.0, 50.0), 20); // capped at 500
        assert_eq!(tick_restore(500, 1000, 0, 50.0, 50.0), 0); // at the cap
        assert_eq!(tick_restore(700, 1000, 0, 50.0, 50.0), 0); // above the cap: no-op
        assert_eq!(tick_restore(100, 1000, 0, -5.0, 100.0), 0);
        assert_eq!(tick_restore(100, 1000, 0, 5.0, 0.0), 0);
    }
}

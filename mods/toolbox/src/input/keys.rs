//! Key names to DirectInput scan codes (DIK_*), from `keys.txt` (also read by
//! the MCP, so the two never drift apart).

use std::collections::HashMap;
use std::sync::OnceLock;

const TABLE: &str = include_str!("keys.txt");

fn table() -> &'static HashMap<&'static str, usize> {
    static MAP: OnceLock<HashMap<&'static str, usize>> = OnceLock::new();
    MAP.get_or_init(|| {
        TABLE
            .lines()
            .filter(|line| !line.starts_with('#'))
            .filter_map(|line| {
                let (name, code) = line.split_once(' ')?;
                Some((name, usize::from_str_radix(code.trim().strip_prefix("0x")?, 16).ok()?))
            })
            .collect()
    })
}

pub fn dik(name: &str) -> Option<usize> {
    let key = name.to_ascii_lowercase();
    table().get(key.as_str()).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_keys_the_game_and_the_mods_need_are_there() {
        assert_eq!(dik("minus"), Some(0x0C));
        assert_eq!(dik("-"), Some(0x0C));
        assert_eq!(dik("PLUS"), Some(0x0D));
        assert_eq!(dik("numpadplus"), Some(0x4E));
        assert_eq!(dik("e"), Some(0x12));
        assert_eq!(dik("down"), Some(0xD0));
        assert_eq!(dik("nope"), None);
    }

    #[test]
    fn every_line_parses_and_names_are_unique() {
        let lines: Vec<&str> = TABLE.lines().filter(|l| !l.is_empty() && !l.starts_with('#')).collect();
        assert_eq!(lines.len(), table().len(), "a line did not parse or a name is repeated");
        assert!(table().values().all(|&code| (1..=0xFF).contains(&code)));
    }
}

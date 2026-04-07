//! Keyboard key name parsing and mapping.
//!
//! Converts key names (e.g., "Enter", "Control+a") to CDP key event parameters.

/// Parse a key combination string into individual keys and modifier flags.
///
/// Returns (key, modifiers) where modifiers is a bitmask:
/// - Bit 0: Alt
/// - Bit 1: Ctrl
/// - Bit 2: Meta (Cmd on macOS)
/// - Bit 3: Shift
pub fn parse_key_combination(combo: &str) -> (String, u32) {
    let parts: Vec<&str> = combo.split('+').collect();
    let mut modifiers: u32 = 0;
    let mut key = String::new();

    for part in parts {
        match part.to_lowercase().as_str() {
            "alt" | "option" => modifiers |= 1,
            "control" | "ctrl" => modifiers |= 2,
            "meta" | "command" | "cmd" => modifiers |= 4,
            "shift" => modifiers |= 8,
            _ => key = part.to_string(),
        }
    }

    (key, modifiers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_key() {
        let (key, mods) = parse_key_combination("Enter");
        assert_eq!(key, "Enter");
        assert_eq!(mods, 0);
    }

    #[test]
    fn test_ctrl_key() {
        let (key, mods) = parse_key_combination("Control+a");
        assert_eq!(key, "a");
        assert_eq!(mods, 2);
    }

    #[test]
    fn test_multi_modifier() {
        let (key, mods) = parse_key_combination("Control+Shift+Delete");
        assert_eq!(key, "Delete");
        assert_eq!(mods, 10); // Ctrl (2) + Shift (8)
    }
}

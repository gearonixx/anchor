use super::*;

#[test]
fn accepts_single_and_repeated_emoji() {
    assert!(is_emoji("👍"));
    assert!(is_emoji("👍👍👍"));
    assert!(is_emoji("😂 🔥 💀"));
    assert!(is_emoji("  ❤️  "));
}

#[test]
fn accepts_multi_scalar_sequences() {
    assert!(is_emoji("👍🏽"));
    assert!(is_emoji("👨‍👩‍👧‍👦"));
    assert!(is_emoji("🇷🇺"));
    assert!(is_emoji("1️⃣"));
    assert!(is_emoji("↩️"));
    assert!(is_emoji("🏳️‍🌈"));
    assert!(is_emoji("❤️‍🔥"));
}

#[test]
fn accepts_minimally_qualified_spellings() {
    assert!(is_emoji("❤"));
    assert!(is_emoji("☺"));
    assert!(is_emoji("1⃣"));
}

#[test]
fn rejects_text_and_mixed_content() {
    assert!(!is_emoji("hi"));
    assert!(!is_emoji("hi 👍"));
    assert!(!is_emoji("👍 nice"));
    assert!(!is_emoji("hello"));
    assert!(!is_emoji(":)"));
    assert!(!is_emoji("))"));
}

#[test]
fn rejects_scalars_outside_the_emoji_table() {
    assert!(!is_emoji("→"));
    assert!(!is_emoji("1"));
    assert!(!is_emoji("2026"));
}

#[test]
fn accepts_selectorless_symbols_telegram_renders_large() {
    assert!(is_emoji("▪"));
    assert!(is_emoji("⬛"));
    assert!(is_emoji("➡"));
}

#[test]
fn rejects_empty_and_whitespace() {
    assert!(!is_emoji(""));
    assert!(!is_emoji("   "));
    assert!(!is_emoji("\n"));
}

#[test]
fn rejects_runs_longer_than_telegram_renders_large() {
    assert!(is_emoji("🔥🔥🔥"));
    assert!(!is_emoji("🔥🔥🔥🔥"));
    assert_eq!(emoji_cluster_count("🔥🔥🔥🔥"), Some(4));
}

#[test]
fn every_emoji_in_the_unicode_table_is_recognized() {
    let missed: Vec<&str> = emojis::iter()
        .map(|e| e.as_str())
        .filter(|e| !is_emoji(e))
        .collect();
    assert!(missed.is_empty(), "unrecognized: {missed:?}");
}

#[test]
fn every_emoji_survives_selector_stripping() {
    let missed: Vec<String> = emojis::iter()
        .map(|e| e.as_str().replace(VARIATION_SELECTOR, ""))
        .filter(|e| !is_emoji(e))
        .collect();
    assert!(missed.is_empty(), "unrecognized unqualified: {missed:?}");
}

#[test]
fn every_emoji_stays_one_grapheme_cluster() {
    let split: Vec<&str> = emojis::iter()
        .map(|e| e.as_str())
        .filter(|e| emoji_cluster_count(e) != Some(1))
        .collect();
    assert!(split.is_empty(), "split into several clusters: {split:?}");
}

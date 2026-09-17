use unicode_segmentation::UnicodeSegmentation;

const VARIATION_SELECTOR: char = '\u{FE0F}';
const ZWJ: char = '\u{200D}';
const KEYCAP: char = '\u{20E3}';

pub const ISOLATED_EMOJI_LIMIT: usize = 3;

pub fn is_emoji(text: &str) -> bool {
    emoji_cluster_count(text).is_some_and(|n| n > 0 && n <= ISOLATED_EMOJI_LIMIT)
}

fn emoji_cluster_count(text: &str) -> Option<usize> {
    let mut count = 0;

    for cluster in text.trim().graphemes(true) {
        if cluster.chars().all(char::is_whitespace) {
            continue;
        }
        if !is_emoji_inner(cluster) {
            return None;
        }
        count += 1;
    }

    Some(count)
}

fn is_emoji_inner(cluster: &str) -> bool {
    if is_keycap_base_without_keycap(cluster) {
        return false;
    }

    if emojis::get(cluster).is_some() {
        return true;
    }

    let unqualified: String = cluster.chars().filter(|&c| c != VARIATION_SELECTOR).collect();
    if emojis::get(&unqualified).is_some() {
        return true;
    }

    let mut qualified = String::with_capacity(unqualified.len() + 3);
    for c in unqualified.chars() {
        qualified.push(c);
        if !matches!(c, ZWJ | KEYCAP) && !is_skin_tone(c) && !is_regional_indicator(c) {
            qualified.push(VARIATION_SELECTOR);
        }
    }
    emojis::get(&qualified).is_some()
}

fn is_keycap_base_without_keycap(cluster: &str) -> bool {
    !cluster.contains(KEYCAP)
        && cluster
            .chars()
            .any(|c| c.is_ascii_digit() || c == '#' || c == '*')
}

fn is_skin_tone(c: char) -> bool {
    matches!(c, '\u{1F3FB}'..='\u{1F3FF}')
}

fn is_regional_indicator(c: char) -> bool {
    matches!(c, '\u{1F1E6}'..='\u{1F1FF}')
}

#[cfg(test)]
#[path = "../../tests/unit/helpers/emoji_test.rs"]
mod tests;

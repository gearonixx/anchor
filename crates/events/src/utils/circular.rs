// quiet = [false, true, true, true, false, ...]
//                  ↑    ↑    ↑
//
// find the longest consecutive run of true
pub(crate) fn longest_circular_run(flags: &[bool]) -> Option<(usize, usize)> {
    let anchor = flags.iter().position(|&flag| !flag)?;
    let mut best: Option<(usize, usize)> = None;
    let mut current: Option<(usize, usize)> = None;

    for step in 1..=flags.len() {
        let index = (anchor + step) % flags.len();
        if !flags[index] {
            current = None;
            continue;
        }
        let run = current.get_or_insert((index, 0));
        run.1 += 1;
        if best.is_none_or(|(_, best_len)| run.1 > best_len) {
            best = Some(*run);
        }
    }
    best
}

#[cfg(test)]
#[path = "../../tests/unit/utils/circular_test.rs"]
mod tests;

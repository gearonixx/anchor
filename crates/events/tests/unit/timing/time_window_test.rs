use crate::lever::{DAY_START_WINDOW, DAY_END_WINDOW};

use super::*;

#[test]
fn every_minute_including_window_edges_is_equally_likely() {
    let date: LocalDate = "2026-01-01".parse().unwrap();

    for (window, kind) in [
        (DAY_START_WINDOW, Event::DayStart),
        (DAY_END_WINDOW, Event::DayEnd),
    ] {
        let mut counts = vec![0; (window.end - window.start) as usize + 1];
        let sample_count = counts.len() * 1_000;

        for peer_id in 1..=sample_count as i64 {
            let minute = window.pick_random_minute_in_window(peer_id, date, kind);
            assert!((window.start..=window.end).contains(&minute));
            assert_eq!(minute.fract(), 0.0);
            counts[(minute - window.start) as usize] += 1;
        }

        for (minute, count) in counts.into_iter().enumerate() {
            assert!(
                (800..=1_200).contains(&count),
                "kind={kind:?} minute_index={minute} count={count}"
            );
        }
    }
}

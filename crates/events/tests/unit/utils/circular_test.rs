use super::*;

#[test]
fn the_longest_quiet_run_wins_even_across_midnight() {
    let mut flags = [false; 24];
    for hour in [22, 23, 0, 1, 2, 7, 8] {
        flags[hour] = true;
    }
    assert_eq!(longest_circular_run(&flags), Some((22, 5)));
    assert_eq!(longest_circular_run(&[true; 24]), None);
}

#[test]
fn a_day_that_is_never_quiet_has_no_run_to_report() {
    assert_eq!(longest_circular_run(&[false; 24]), None);
    assert_eq!(longest_circular_run(&[]), None);
}

#[test]
fn a_single_quiet_hour_is_still_a_run() {
    let mut flags = [false; 24];
    flags[5] = true;
    assert_eq!(longest_circular_run(&flags), Some((5, 1)));
}

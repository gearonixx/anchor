use super::*;

const USER_ID: i64 = 12223;
const FRIEND_ID: i64 = 45567;

#[test]
fn rejects_service_and_low_ids() {
    assert!(!is_real_user(777_000, &[]));
    assert!(!is_real_user(333_000, &[]));
    assert!(!is_real_user(42_777, &[USER_ID]));
    assert!(!is_real_user(0, &[]));
    assert!(!is_real_user(999, &[USER_ID]));
    assert!(!is_real_user(-1, &[]));
}

#[test]
fn empty_peer_ids_is_public() {
    assert!(is_real_user(USER_ID, &[]));
    assert!(is_real_user(1_234_567_890, &[]));
}

#[test]
fn peer_ids_gate_everyone_else() {
    assert!(is_real_user(USER_ID, &[USER_ID]));
    assert!(is_real_user(FRIEND_ID, &[USER_ID, FRIEND_ID]));
    assert!(!is_real_user(1_234_567_890, &[USER_ID, FRIEND_ID]));
    assert!(!is_real_user(777_000, &[777_000]));
}

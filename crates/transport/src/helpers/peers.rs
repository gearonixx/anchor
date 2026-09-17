const SERVICE_IDS: [i64; 3] = [777_000, 333_000, 42_777];

pub fn is_real_user(id: i64, peers: &[i64]) -> bool {
    if id < 1000 || SERVICE_IDS.contains(&id) {
        return false;
    }
    peers.is_empty() || peers.contains(&id)
}

#[cfg(test)]
#[path = "../../tests/unit/helpers/peers_test.rs"]
mod tests;

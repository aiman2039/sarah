// sarah-allow-file(large_file): fixture for tests/cli.rs

// sarah-allow(large_function): one flat table, splitting hurts reading
#[inline]
pub fn allowed(x: u32) -> u32 {
    let mut total = x;
    total += 1;
    total += 2;
    total += 3;
    total
}

pub fn reported(x: u32) -> u32 {
    let mut total = x;
    total += 1;
    total += 2;
    total += 3;
    total
}

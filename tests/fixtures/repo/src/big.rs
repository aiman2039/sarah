// A file with one long function and one short one.

pub fn long_one(x: u32) -> u32 {
    let mut total = 0;
    total += x;
    total += x * 2;
    total += x * 3;
    total += x * 4;
    total += x * 5;
    total += x * 6;
    total += x * 7;
    total += x * 8;
    total += x * 9;
    total += x * 10;
    total += x * 11;
    total += x * 12;
    total
}

pub fn short_one() -> u32 {
    1
}

fn popcount_naive(x: u32) -> u32 {
    let mut count = 0;
    let mut temp = x;
    for _ in 0..32 {
        if temp & 1 != 0 {
            count += 1;
        }
        temp >>= 1;
    }
    count
}

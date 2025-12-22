/// Computes the greatest common divisor (GCD) of two unsigned integers. This
/// function implements the classic Euclidean algorithm.
#[inline]
pub fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let temp = a % b;
        a = b;
        b = temp;
    }
    a
}

/// Returns the integer nearest to `base` that is evenly divisible by
/// `denominator`. This function is constant-time.
#[inline]
pub fn find_nearest_to(base: u32, denominator: u32) -> u32 {
    let remainder = base % denominator;
    if remainder * 2 <= denominator {
        base - remainder
    } else {
        base - remainder + denominator
    }
}

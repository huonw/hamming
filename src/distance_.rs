use std::{mem, slice};

fn naive(x: &[u8], y: &[u8]) -> u64 {
    assert_eq!(x.len(), y.len());
    x.iter().zip(y).fold(0, |a, (b, c)| a + (*b ^ *c).count_ones() as u64)
}

/// Computes the bitwise [Hamming
/// distance](https://en.wikipedia.org/wiki/Hamming_distance) between
/// `x` and `y`, that is, the number of bits where `x` and `y` differ,
/// or, the number of set bits in the xor of `x` and `y`.
///
/// This is a SIMD-enabled version, which performs significantly better than the
/// optimized version on large workloads. Without SIMD, it's slightly more
/// performant than the naiive algorithm.
#[cfg(feature = "simd")]
pub fn distance_simd(x: &[u8], y: &[u8]) -> u64 {
    use faster::*;

    assert_eq!(x.len(), y.len());

    (x.simd_iter(u8s(0)), y.simd_iter(u8s(0))).zip()
        .simd_reduce(0, |acc, (a, b)| acc + (a ^ b).count_ones()) as u64

}

#[derive(Debug, PartialEq, Eq, Ord, PartialOrd, Hash, Clone)]
pub struct DistanceError {
    _x: ()
}

/// Computes the bitwise [Hamming
/// distance](https://en.wikipedia.org/wiki/Hamming_distance) between
/// `x` and `y`, that is, the number of bits where `x` and `y` differ,
/// or, the number of set bits in the xor of `x` and `y`.
///
/// This is a highly optimised version of the following naive version:
///
/// ```rust
/// fn naive(x: &[u8], y: &[u8]) -> u64 {
///     x.iter().zip(y).fold(0, |a, (b, c)| a + (*b ^ *c).count_ones() as u64)
/// }
/// ```
///
/// This function requires that `x` and `y` have the same 8-byte
/// alignment. If not, `Err` is returned. If sub-optimal performance
/// can be tolerated, consider using `distance` which incorporates a
/// fallback to a slower but less restrictive algorithm.
///
/// It is essentially guaranteed that `x` and `y` will have the same
/// 8-byte alignment if they are both just `Vec<u8>`s of non-trivial
/// length (e.g. larger than 8) as in the example below.
///
/// This is implemented using the same tree-merging approach as
/// `weight`, see there for details.
///
/// # Panics
///
/// `x` and `y` must have the same length, or else `distance_fast` panics.
///
/// # Performance Comparison
///
/// | length | `naive` (ns) | `distance_fast` (ns) | `naive`/`distance_fast` |
/// |--:|--:|--:|--:|
/// | 1 | 5  | 6  | 0.83 |
/// | 10 | 44  | 45  | 0.97 |
/// | 100 | 461  | 473  | 0.97 |
/// | 1,000 | 4,510  | 397  | 11 |
/// | 10,000 | 46,700  | 2,740  | 17 |
/// | 100,000 | 45,600  | 20,400  | 22 |
/// | 1,000,000 | 4,590,000  | 196,000  | 23 |
///
/// # Examples
///
/// ```rust
/// let x = vec![0xFF; 1000];
/// let y = vec![0; 1000];
/// assert_eq!(hamming::distance_fast(&x, &y), Ok(8 * 1000));
///
/// // same alignment, but moderately complicated
/// assert_eq!(hamming::distance_fast(&x[1..1000 - 8], &y[8 + 1..]), Ok(8 * (1000 - 8 - 1)));
///
/// // differing alignments
/// assert!(hamming::distance_fast(&x[1..], &y[..999]).is_err());
/// ```
pub fn distance_fast(x: &[u8], y: &[u8]) -> Result<u64, DistanceError> {
    assert_eq!(x.len(), y.len());

    const M1: u64 = 0x5555555555555555;
    const M2: u64 = 0x3333333333333333;
    const M4: u64 = 0x0F0F0F0F0F0F0F0F;
    const M8: u64 = 0x00FF00FF00FF00FF;

    type T30 = [u64; 30];
    let size = mem::size_of::<T30>();
    let alignment = mem::align_of::<T30>();

    let ptr1 = x.as_ptr() as usize;
    let ptr2 = y.as_ptr() as usize;

    // round up to the nearest multiple
    let aligned1 = (ptr1 + alignment - 1) / alignment * alignment;
    let aligned2 = (ptr2 + alignment - 1) / alignment * alignment;
    let distance1 = aligned1 - ptr1;
    let distance2 = aligned2 - ptr2;

    // can't fit a single T30 in
    if x.len() < size + distance1 {
        return Ok(naive(x, y))
    }

    if distance1 != distance2 {
        return Err(DistanceError { _x: () });
    }

    let (head1, middle1) = x.split_at(distance1);
    let (head2, middle2) = y.split_at(distance2);

    assert!(middle1.as_ptr() as usize % alignment == 0);
    assert!(middle2.as_ptr() as usize % alignment == 0);
    let (thirty1, thirty2) = unsafe {
        (slice::from_raw_parts(middle1.as_ptr() as *const T30,
                               middle1.len() / size),
         slice::from_raw_parts(middle2.as_ptr() as *const T30,
                               middle2.len() / size))
    };
    debug_assert_eq!(thirty1.len(), thirty2.len());

    let tail1 = &middle1[thirty1.len() * size..];
    let tail2 = &middle2[thirty2.len() * size..];
    let mut count = naive(head1, head2) + naive(tail1, tail2);
    for (array1, array2) in thirty1.iter().zip(thirty2) {
        let mut acc = 0;
        for j_ in 0..10 {
            let j = j_ * 3;
            let mut count1 = array1[j] ^ array2[j];
            let mut count2 = array1[j + 1] ^ array2[j + 1];
            let mut half1 = array1[j + 2] ^ array2[j + 2];
            let mut half2 = half1;
            half1 &= M1;
            half2 = (half2 >> 1) & M1;
            count1 -= (count1 >> 1) & M1;
            count2 -= (count2 >> 1) & M1;
            count1 += half1;
            count2 += half2;
            count1 = (count1 & M2) + ((count1 >> 2) & M2);
            count1 += (count2 & M2) + ((count2 >> 2) & M2);
            acc += (count1 & M4) + ((count1 >> 4) & M4);
        }
        acc = (acc & M8) + ((acc >> 8) & M8);
        acc =  acc       +  (acc >> 16);
        acc =  acc       +  (acc >> 32);
        count += acc & 0xFFFF;
    }
    Ok(count)
}

/// Computes the bitwise [Hamming
/// distance](https://en.wikipedia.org/wiki/Hamming_distance) between
/// `x` and `y`, that is, the number of bits where `x` and `y` differ,
/// or, the number of set bits in the xor of `x` and `y`.
///
/// When `x` and `y` have the same 8-byte alignment, this uses
/// `distance_fast`, a highly optimised version of the following naive
/// version:
///
/// ```rust
/// fn naive(x: &[u8], y: &[u8]) -> u64 {
///     x.iter().zip(y).fold(0, |a, (b, c)| a + (*b ^ *c).count_ones() as u64)
/// }
/// ```
///
/// If alignments differ, a slower but less restrictive algorithm is
/// used.
///
/// It is essentially guaranteed that `x` and `y` will have the same
/// 8-byte alignment if they are both just `Vec<u8>`s of non-trivial
/// length (e.g. larger than 8) as in the example below.
///
/// # Panics
///
/// `x` and `y` must have the same length, or else `distance` panics.
///
/// # Performance Comparison
///
/// | length | `naive` (ns) | `distance` (ns) | `naive`/`distance` |
/// |--:|--:|--:|--:|
/// | 1 | 5  | 6  | 0.83 |
/// | 10 | 44  | 45  | 0.97 |
/// | 100 | 461  | 473  | 0.97 |
/// | 1,000 | 4,510  | 397  | 11 |
/// | 10,000 | 46,700  | 2,740  | 17 |
/// | 100,000 | 45,600  | 20,400  | 22 |
/// | 1,000,000 | 4,590,000  | 196,000  | 23 |
///
/// The benchmarks ensured that `x` and `y` had the same alignment.
///
/// # Examples
///
/// ```rust
/// let x = vec![0xFF; 1000];
/// let y = vec![0; 1000];
/// assert_eq!(hamming::distance(&x, &y), 8 * 1000);
/// ```
pub fn distance(x: &[u8], y: &[u8]) -> u64 {
    distance_fast(x, y)
        .ok()
        .unwrap_or_else(|| naive(x, y))
}

#[cfg(all(test, feature = "unstable"))]
fn distance_(x: &[u8], y: &[u8]) -> u64 {
    distance_fast(x, y).unwrap()
}

#[cfg(test)]
mod tests {
    use quickcheck as qc;
    use rand;
    #[test]
    fn naive_smoke() {
        let tests: &[(&[u8], &[u8], u64)] = &[
            (&[], &[], 0),
            (&[0], &[0], 0),
            (&[0], &[0xFF], 8),
            (&[0b10101010], &[0b01010101], 8),
            (&[0b11111010], &[0b11110101], 4),
            (&[0; 10], &[0; 10], 0),
            (&[0xFF; 10], &[0x0F; 10], 4 * 10),
            (&[0x3B; 10000], &[0x3B; 10000], 0),
            (&[0x77; 10000], &[0x3B; 10000], 3 * 10000),
            ];
        for &(x, y, expected) in tests {
            assert_eq!(super::naive(x, y), expected);
        }
    }
    #[test]
    fn distance_fast_qc() {
        fn prop(v: Vec<u8>, w: Vec<u8>, misalign: u8) -> qc::TestResult {
            let l = ::std::cmp::min(v.len(), w.len());
            if l < misalign as usize {
                return qc::TestResult::discard()
            }

            let x = &v[misalign as usize..l];
            let y = &w[misalign as usize..l];
            qc::TestResult::from_bool(super::distance_fast(x, y).unwrap() == super::naive(x, y))
        }
        qc::QuickCheck::new()
            .gen(qc::StdGen::new(rand::thread_rng(), 10_000))
            .quickcheck(prop as fn(Vec<u8>,Vec<u8>,u8) -> qc::TestResult)
    }
    #[test]
    fn distance_fast_smoke_huge() {
        let v = vec![0b1001_1101; 10234567];
        let w = vec![0b1111_1111; v.len()];

        assert_eq!(super::distance_fast(&v, &v).unwrap(), 0);
        assert_eq!(super::distance_fast(&v, &w).unwrap(), 3 * w.len() as u64);
    }
    #[test]
    fn distance_smoke() {
        let v = vec![0; 10000];
        let w = vec![0xFF; v.len()];
        for len_ in 0..99 {
            let len = len_ * 10;
            for i in 0..8 {
                for j in 0..8 {
                    assert_eq!(super::distance(&v[i..i+len], &w[j..j+len]),
                               len as u64 * 8)
                }
            }
        }
    }
    #[test]
    #[cfg(feature = "simd")]
    fn distance_simd_smoke() {
        let v = vec![0b0000_0000; 10234567];
        let w = vec![0b1111_1111; v.len()];
        let x = vec![0b0000_1111; 10234567];
        let y = vec![0b1100_0000; 10234567];
        let z = vec![0b0011_0000; 10234567];


        assert_eq!(super::distance_simd(&v, &v), 0);
        assert_eq!(super::distance_simd(&w, &w), 0);
        assert_eq!(super::distance_simd(&v, &w), 8 * w.len() as u64);
        assert_eq!(super::distance_simd(&v, &x), 4 * w.len() as u64);
        assert_eq!(super::distance_simd(&v, &y), 2 * w.len() as u64);
        assert_eq!(super::distance_simd(&v, &z), 2 * w.len() as u64);
    }
}

#[cfg(all(test, feature = "unstable"))]
mod benches {
    use test;

    fn bench<F: FnMut(&[u8], &[u8]) -> u64>(b: &mut test::Bencher, n: usize, mut f: F) {
        let data = vec![0xFF; n];
        b.iter(|| {
            let d1 = test::black_box(&data);
            let d2 = test::black_box(&data);
            f(d1, d2)
        })
    }

    macro_rules! test_mod {
        ($name: ident) => {
            mod $name {
                use test;
                use super::bench;
                use super::super::$name;
                #[bench]
                fn _0000001(b: &mut test::Bencher) { bench(b, 1, $name) }
                #[bench]
                fn _0000010(b: &mut test::Bencher) { bench(b, 10, $name) }
                #[bench]
                fn _0000100(b: &mut test::Bencher) { bench(b, 100, $name) }
                #[bench]
                fn _0001000(b: &mut test::Bencher) { bench(b, 1000, $name) }
                #[bench]
                fn _0010000(b: &mut test::Bencher) { bench(b, 10000, $name) }
                #[bench]
                fn _0100000(b: &mut test::Bencher) { bench(b, 100000, $name) }
                #[bench]
                fn _1000000(b: &mut test::Bencher) { bench(b, 1000000, $name) }
            }
        }
    }
    test_mod!(naive);
    test_mod!(distance_);
    #[cfg(feature = "simd")]
    test_mod!(distance_simd);
}

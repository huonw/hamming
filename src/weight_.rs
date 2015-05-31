use std::{slice, mem};

fn naive(x: &[u8]) -> u64 {
    x.iter().fold(0, |a, b| a + b.count_ones() as u64)
}
/// Computes the [Hamming
/// weight](https://en.wikipedia.org/wiki/Hamming_weight) of `x`, that
/// is, the population count, or number of 1.
///
/// This is a highly optimised version of the following naive version:
///
/// ```rust
/// fn naive(x: &[u8]) -> u64 {
///     x.iter().fold(0, |a, b| a + b.count_ones() as u64)
/// }
/// ```
///
/// This uses Lauradoux Cédric's [tree-merging
/// approach](http://web.archive.org/web/20120411185540/http://perso.citi.insa-lyon.fr/claurado/hamming.html)
/// (as implemented by Kim Walisch in
/// [primesieve](http://primesieve.org/)) and achieves on the order of
/// 1-2 cycles per byte.
///
/// # Performance Comparison
///
/// | length | `naive` (ns) | `weight` (ns) | `naive`/`weight` |
/// |--:|--:|--:|--:|
/// | 1  | 5  | 16  | 0.31 |
/// | 10  | 29  | 51  | 0.56 |
/// | 100  | 284  | 392  | 0.72 |
/// | 1,000  | 2,780 | 340  | 8.2 |
/// | 10,000  | 27,700  | 2,300  | 12 |
/// | 100,000  | 276,000  | 17,900  | 15 |
/// | 1,000,000  | 2,770,000  | 172,000  | 16 |
///
/// # Example
///
/// ```rust
/// assert_eq!(hamming::weight(&[1, 0xFF, 1, 0xFF]), 1 + 8 + 1 + 8);
/// ```
pub fn weight(x: &[u8]) -> u64 {
    const M1: u64 = 0x5555555555555555;
    const M2: u64 = 0x3333333333333333;
    const M4: u64 = 0x0F0F0F0F0F0F0F0F;
    const M8: u64 = 0x00FF00FF00FF00FF;

    type T30 = [u64; 30];
    let size = mem::size_of::<T30>();
    let alignment = mem::align_of::<T30>();

    let ptr = x.as_ptr() as usize;
    // round up to the nearest multiple
    let aligned = (ptr + alignment - 1) / alignment * alignment;
    let distance = aligned - ptr;

    // can't fit a single T30 in
    if x.len() < size + distance {
        return naive(x)
    }

    let (head, middle) = x.split_at(distance);

    assert!(middle.as_ptr() as usize % alignment == 0);
    let thirty = unsafe {
        slice::from_raw_parts(middle.as_ptr() as *const T30,
                              middle.len() / size)
    };
    let tail = &middle[thirty.len() * size..];
    let mut count = naive(head) + naive(tail);
    for array in thirty {
        let mut acc = 0;
        for j_ in 0..10 {
            let j = j_ * 3;
            let mut count1 = array[j];
            let mut count2 = array[j + 1];
            let mut half1 = array[j + 2];
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
    count
}

#[cfg(test)]
mod tests {
    use quickcheck as qc;
    use rand;
    #[test]
    fn naive_smoke() {
        let tests = [(&[0u8] as &[u8], 0),
                     (&[1], 1),
                     (&[0xFF], 8),
                     (&[0xFF; 10], 8 * 10),
                     (&[1; 1000], 1000)];
        for &(v, expected) in &tests {
            assert_eq!(super::naive(v), expected);
        }
    }
    #[test]
    fn weight_qc() {
        fn prop(v: Vec<u8>, misalign: u8) -> bool {
            let data = &v[misalign as usize..];
            super::weight(data) == super::naive(data)
        }
        qc::QuickCheck::new()
            .gen(qc::StdGen::new(rand::thread_rng(), 10_000))
            .quickcheck(prop as fn(Vec<u8>,u8) -> bool)
    }
    #[test]
    fn weight_huge() {
        let v = vec![0b1001_1101; 10234567];
        assert_eq!(super::weight(&v),
                   v[0].count_ones() as u64 * v.len() as u64);
    }
}

#[cfg(all(test, feature = "unstable"))]
mod benches {
    use test;
    fn bench<F: FnMut(&[u8]) -> u64>(b: &mut test::Bencher, n: usize, mut f: F) {
        let data = vec![0xFF; n];
        b.iter(|| f(test::black_box(&data)))
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
    test_mod!(weight);
}

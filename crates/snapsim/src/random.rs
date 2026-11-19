//! CPython's integer-seeded MT19937 and the sampling methods used by the engine.

const N: usize = 624;
const M: usize = 397;

/// Random operations consumed by game rules and policies.
///
/// Implementations define their own stream; parity uses `PythonRandom`.
pub trait GameRng {
    fn getrandbits(&mut self, bits: u32) -> u128;
    fn random(&mut self) -> f64;
    fn randbelow(&mut self, n: usize) -> usize;
    fn randrange(&mut self, start: i64, stop: i64, step: i64) -> i64;
    fn randint(&mut self, a: i64, b: i64) -> i64;
    fn choice<'a, T>(&mut self, values: &'a [T]) -> &'a T
    where
        Self: Sized,
    {
        &values[self.randbelow(values.len())]
    }
    fn shuffle<T>(&mut self, values: &mut [T])
    where
        Self: Sized,
    {
        for i in (1..values.len()).rev() {
            values.swap(i, self.randbelow(i + 1));
        }
    }
    fn sample_indices(&mut self, n: usize, k: usize) -> Vec<usize>;
}

/// The version-3 CPython state: 624 words, a cursor, and the Gaussian cache.
#[derive(Clone, Debug, PartialEq)]
pub struct RandomState {
    pub words: [u32; N],
    pub index: usize,
    pub gaussian: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct PythonRandom {
    state: RandomState,
}

impl PythonRandom {
    /// Seed as CPython does for an integer, using its absolute value.
    pub fn seed(seed: i128) -> Self {
        let value = seed.unsigned_abs();
        let key = [
            value as u32,
            (value >> 32) as u32,
            (value >> 64) as u32,
            (value >> 96) as u32,
        ];
        Self::seed_words(&key)
    }

    /// Seed from the little-endian 32-bit words of an integer's absolute value.
    /// Leading zero words are ignored, so this also accepts integers wider than i128.
    pub fn seed_words(words: &[u32]) -> Self {
        let end = words
            .iter()
            .rposition(|&word| word != 0)
            .map_or(1, |i| i + 1);
        let zero = [0];
        let key = if words.is_empty() {
            &zero[..]
        } else {
            &words[..end]
        };
        let mut words = [0u32; N];
        words[0] = 19_650_218;
        for i in 1..N {
            words[i] = 1_812_433_253u32
                .wrapping_mul(words[i - 1] ^ (words[i - 1] >> 30))
                .wrapping_add(i as u32);
        }
        let (mut i, mut j) = (1, 0);
        for _ in 0..N.max(key.len()) {
            words[i] = (words[i] ^ (words[i - 1] ^ (words[i - 1] >> 30)).wrapping_mul(1_664_525))
                .wrapping_add(key[j])
                .wrapping_add(j as u32);
            i += 1;
            j += 1;
            if i >= N {
                words[0] = words[N - 1];
                i = 1;
            }
            if j >= key.len() {
                j = 0;
            }
        }
        for _ in 0..N - 1 {
            words[i] = (words[i]
                ^ (words[i - 1] ^ (words[i - 1] >> 30)).wrapping_mul(1_566_083_941))
            .wrapping_sub(i as u32);
            i += 1;
            if i >= N {
                words[0] = words[N - 1];
                i = 1;
            }
        }
        words[0] = 0x8000_0000;
        Self {
            state: RandomState {
                words,
                index: N,
                gaussian: None,
            },
        }
    }

    pub fn getstate(&self) -> RandomState {
        self.state.clone()
    }

    pub fn setstate(&mut self, state: RandomState) {
        assert!(state.index <= N, "invalid MT19937 index");
        self.state = state;
    }

    fn word(&mut self) -> u32 {
        if self.state.index >= N {
            for i in 0..N {
                let y = (self.state.words[i] & 0x8000_0000)
                    | (self.state.words[(i + 1) % N] & 0x7fff_ffff);
                self.state.words[i] = self.state.words[(i + M) % N]
                    ^ (y >> 1)
                    ^ if y & 1 != 0 { 0x9908_b0df } else { 0 };
            }
            self.state.index = 0;
        }
        let mut y = self.state.words[self.state.index];
        self.state.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^ (y >> 18)
    }

    /// Equivalent to `getrandbits(k)` for 0..=128 bits.
    pub fn getrandbits(&mut self, bits: u32) -> u128 {
        assert!(bits <= 128);
        if bits == 0 {
            return 0;
        }
        let mut result = 0u128;
        let mut remaining = bits;
        let mut shift = 0;
        while remaining > 0 {
            let take = remaining.min(32);
            let word = self.word() >> (32 - take);
            result |= u128::from(word) << shift;
            remaining -= take;
            shift += 32;
        }
        result
    }

    pub fn random(&mut self) -> f64 {
        let a = u64::from(self.word() >> 5);
        let b = u64::from(self.word() >> 6);
        ((a << 26) + b) as f64 * (1.0 / 9_007_199_254_740_992.0)
    }

    pub fn randbelow(&mut self, n: usize) -> usize {
        assert!(n > 0);
        let bits = usize::BITS - n.leading_zeros();
        loop {
            let value = self.getrandbits(bits) as usize;
            if value < n {
                return value;
            }
        }
    }

    pub fn randrange(&mut self, start: i64, stop: i64, step: i64) -> i64 {
        assert_ne!(step, 0);
        let width = i128::from(stop) - i128::from(start);
        let step128 = i128::from(step);
        let count = if step > 0 {
            (width + step128 - 1) / step128
        } else {
            (width + step128 + 1) / step128
        };
        assert!(
            count > 0 && count <= usize::MAX as i128,
            "empty or oversized range"
        );
        (i128::from(start) + i128::try_from(self.randbelow(count as usize)).unwrap() * step128)
            as i64
    }

    pub fn randint(&mut self, a: i64, b: i64) -> i64 {
        assert!(a <= b);
        let width = i128::from(b) - i128::from(a) + 1;
        assert!(width <= usize::MAX as i128);
        (i128::from(a) + i128::try_from(self.randbelow(width as usize)).unwrap()) as i64
    }

    pub fn choice<'a, T>(&mut self, values: &'a [T]) -> &'a T {
        &values[self.randbelow(values.len())]
    }

    pub fn shuffle<T>(&mut self, values: &mut [T]) {
        for i in (1..values.len()).rev() {
            values.swap(i, self.randbelow(i + 1));
        }
    }

    /// Indices selected by CPython's `sample(range(n), k)`, in draw order.
    pub fn sample_indices(&mut self, n: usize, k: usize) -> Vec<usize> {
        assert!(k <= n);
        let mut result = Vec::with_capacity(k);
        let mut setsize = 21usize;
        if k > 5 {
            let mut power = 1usize;
            while power < k.saturating_mul(3) {
                power = power.saturating_mul(4);
                if power == usize::MAX {
                    break;
                }
            }
            setsize = setsize.saturating_add(power);
        }
        if n <= setsize {
            let mut pool: Vec<_> = (0..n).collect();
            for i in 0..k {
                let j = self.randbelow(n - i);
                result.push(pool[j]);
                pool[j] = pool[n - i - 1];
            }
        } else {
            let mut selected = std::collections::HashSet::with_capacity(k);
            for _ in 0..k {
                loop {
                    let j = self.randbelow(n);
                    if selected.insert(j) {
                        result.push(j);
                        break;
                    }
                }
            }
        }
        result
    }
}

impl GameRng for PythonRandom {
    fn getrandbits(&mut self, bits: u32) -> u128 {
        PythonRandom::getrandbits(self, bits)
    }
    fn random(&mut self) -> f64 {
        PythonRandom::random(self)
    }
    fn randbelow(&mut self, n: usize) -> usize {
        PythonRandom::randbelow(self, n)
    }
    fn randrange(&mut self, start: i64, stop: i64, step: i64) -> i64 {
        PythonRandom::randrange(self, start, stop, step)
    }
    fn randint(&mut self, a: i64, b: i64) -> i64 {
        PythonRandom::randint(self, a, b)
    }
    fn sample_indices(&mut self, n: usize, k: usize) -> Vec<usize> {
        PythonRandom::sample_indices(self, n, k)
    }
}

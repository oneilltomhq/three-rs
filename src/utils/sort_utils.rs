//! Port of `three.js/examples/jsm/utils/SortUtils.js` — the hybrid radix sort
//! `webgpu_mesh_batch`'s `sortFunction` hands `BatchedMesh`' draw list to.
//!
//! The algorithm is reproduced step for step, including the JS `>>> 0`
//! (`ToUint32`) the keys go through, because the ordering it produces is what
//! ends up in `_indirectTexture` and therefore in `@builtin(instance_index)`.

const POWER: u32 = 3;
const BIT_MAX: u32 = 32;
const BIN_BITS: u32 = 1 << POWER;
const BIN_SIZE: usize = 1 << BIN_BITS;
const BIN_MAX: usize = BIN_SIZE - 1;
const ITERATIONS: usize = (BIT_MAX / BIN_BITS) as usize;

/// ECMAScript `ToUint32`: truncate toward zero, then reduce modulo 2^32.
/// `a >>> b` applies this to `a` before shifting, which is how a float depth
/// scaled by `(2 ** 32 - 1) / camera.far` becomes a radix key.
pub fn to_uint32(value: f64) -> u32 {
    if !value.is_finite() {
        return 0;
    }
    let truncated = value.trunc();
    let wrapped = truncated.rem_euclid(4294967296.0);
    wrapped as u32
}

struct Sorter<'a, T> {
    data: [Vec<T>; 2],
    /// `bins`: `ITERATIONS + 1` counting arrays, shared in three.js through one
    /// `ArrayBuffer`.
    bins: Vec<Vec<u32>>,
    get: &'a dyn Fn(&T) -> u32,
    reversed: bool,
}

impl<T: Clone> Sorter<'_, T> {
    /// `data[ depth & 1 ]` and `data[ ( depth + 1 ) & 1 ]`.
    fn pair(&mut self, depth: usize) -> (&mut Vec<T>, &mut Vec<T>) {
        let (x, y) = self.data.split_at_mut(1);
        if depth & 1 == 0 {
            (&mut x[0], &mut y[0])
        } else {
            (&mut y[0], &mut x[0])
        }
    }

    /// `compare( a, b )`: `a > b` forwards, `a < b` reversed.
    fn compare(&self, a: u32, b: u32) -> bool {
        if self.reversed {
            a < b
        } else {
            a > b
        }
    }

    fn accumulate(&mut self, index: usize) {
        let bin = &mut self.bins[index];
        if self.reversed {
            for j in (0..BIN_SIZE - 1).rev() {
                bin[j] += bin[j + 1];
            }
        } else {
            for j in 1..BIN_SIZE {
                bin[j] += bin[j - 1];
            }
        }
    }

    fn insertion_sort_block(&mut self, depth: usize, start: usize, len: usize) {
        for j in start + 1..start + len {
            let p = self.data[depth & 1][j].clone();
            let t = (self.get)(&p);
            let mut i = j;
            while i > start {
                let prev = (self.get)(&self.data[depth & 1][i - 1]);
                if self.compare(prev, t) {
                    let v = self.data[depth & 1][i - 1].clone();
                    self.data[depth & 1][i] = v;
                    i -= 1;
                } else {
                    break;
                }
            }
            self.data[depth & 1][i] = p;
        }

        if depth & 1 == 1 {
            for i in start..start + len {
                let v = self.data[1][i].clone();
                self.data[0][i] = v;
            }
        }
    }

    fn recurse(&mut self, cache: Vec<u32>, depth: usize, start: usize) {
        let mut prev = 0u32;
        let order: Vec<usize> = if self.reversed {
            (0..=BIN_MAX).rev().collect()
        } else {
            (0..BIN_SIZE).collect()
        };
        for j in order {
            let cur = cache[j];
            let diff = cur.wrapping_sub(prev);
            if diff != 0 {
                if diff > 32 {
                    self.radix_sort_block(depth + 1, start + prev as usize, diff as usize);
                } else {
                    self.insertion_sort_block(depth + 1, start + prev as usize, diff as usize);
                }
                prev = cur;
            }
        }
    }

    fn radix_sort_block(&mut self, depth: usize, start: usize, len: usize) {
        let shift = (3 - depth as u32) << POWER;
        let end = start + len;

        self.bins[depth + 1].iter_mut().for_each(|v| *v = 0);

        for j in start..end {
            let key = ((self.get)(&self.data[depth & 1][j]) >> shift) as usize & BIN_MAX;
            self.bins[depth + 1][key] += 1;
        }

        self.accumulate(depth + 1);

        // `cache.set( bin )` — the accumulated counts are kept for `recurse`,
        // which runs after the scatter loop below has decremented `bin`.
        let cache = self.bins[depth + 1].clone();
        self.bins[depth] = cache.clone();

        for j in (start..end).rev() {
            let key = ((self.get)(&self.data[depth & 1][j]) >> shift) as usize & BIN_MAX;
            self.bins[depth + 1][key] -= 1;
            let target = start + self.bins[depth + 1][key] as usize;
            let v = self.data[depth & 1][j].clone();
            let (_, b) = self.pair(depth);
            b[target] = v;
        }

        if depth == ITERATIONS - 1 {
            return;
        }

        self.recurse(cache, depth, start);
    }
}

/// `radixSort( arr, { get, aux, reversed } )`. Expects unsigned 32-bit keys.
pub fn radix_sort<T: Clone>(arr: &mut Vec<T>, get: &dyn Fn(&T) -> u32, reversed: bool) {
    let len = arr.len();
    if len == 0 {
        return;
    }
    let aux = arr.clone();
    let mut sorter = Sorter {
        data: [std::mem::take(arr), aux],
        bins: vec![vec![0u32; BIN_SIZE]; ITERATIONS + 1],
        get,
        reversed,
    };
    sorter.radix_sort_block(0, 0, len);
    let [first, _] = sorter.data;
    *arr = first;
}

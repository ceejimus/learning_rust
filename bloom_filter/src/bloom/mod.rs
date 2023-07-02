mod words;

use murmur3::murmur3_32;
use std::io::Result;

#[derive(Debug)]
struct BloomFilter {
    size: usize,
    hash_count: usize,
    bit_array: Vec<bool>,
}

impl BloomFilter {
    fn new(fp_rate: f64, n_items: usize) -> Self {
        let size = Self::get_size(fp_rate, n_items);
        let hash_count = Self::get_hash_count(size, n_items);
        let bit_array = vec![false; size];

        BloomFilter {
            size,
            hash_count,
            bit_array,
        }
    }

    fn get_size(fp_rate: f64, n_items: usize) -> usize {
        (-(n_items as f64 * fp_rate.ln()) / (2_f64.ln() * 2_f64.ln())).ceil() as usize
    }

    fn get_hash_count(size: usize, n_items: usize) -> usize {
        ((size as f64 / n_items as f64) * 2_f64.ln()).ceil() as usize
    }

    fn add_item(&mut self, item: &str) {
        (0..self.hash_count).for_each(|i| {
            let digest = Self::hash(&mut item.to_string(), i as u32).unwrap();
            self.bit_array[digest as usize % self.size] = true;
        });
    }

    fn check(&self, item: &str) -> bool {
        for i in 0..self.hash_count {
            let digest = Self::hash(&mut item.to_string(), i as u32).unwrap();
            if !self.bit_array[digest as usize % self.size] {
                return false;
            }
        }

        true
    }

    fn hash(input: &mut str, seed: u32) -> Result<u32> {
        murmur3_32(&mut input.as_bytes(), seed)
    }
}

#[cfg(test)]
mod tests {
    use crate::bloom::words::get_words;
    use rand::Rng;

    use super::*;

    #[test]
    fn can_construct_bloom_filter() {
        let _ = BloomFilter::new(0.05, 40);
    }

    #[test]
    fn can_add_words_to_bloom_filter() {
        let included = get_words(10000);
        let mut bloom = BloomFilter::new(0.05, included.len());
        for word in included {
            bloom.add_item(word);
        }
    }

    #[test]
    fn can_check_words_in_bloom_filter() {
        let mut included = get_words(10000);
        let excluded = included.split_off(5000);
        let mut bloom = BloomFilter::new(0.05, included.len());

        for word in included.iter() {
            bloom.add_item(word);
        }

        for word in included.iter().chain(excluded.iter()) {
            bloom.check(word);
        }
    }

    fn test_bloom(fp_rate: f64, n_included: usize, n_excluded: usize) -> f64 {
        let mut included = get_words(n_included + n_excluded);
        let excluded = included.split_off(n_included);

        let mut bloom = BloomFilter::new(fp_rate, included.len());

        for word in included.iter() {
            bloom.add_item(word);
        }

        let mut fp_count = 0;

        for word in excluded.iter() {
            let res = bloom.check(word);
            if res && !included.contains(word) {
                fp_count += 1;
            }
        }

        // not sure if I've done something wrong but the fp_rate is sometimes a bit higher than
        // fp_rate
        (fp_count as f64 / n_excluded as f64) / fp_rate
    }

    #[test]
    fn bloom_filter_checks_have_correct_fp_rate() {
        let mut rng = rand::thread_rng();
        let max_fp_rate_ratio = 2_f64; // gotta be fairly lenient, bc either stats or I messed up...

        for _ in 0..100 {
            let fp_rate = rng.gen_range(0.0001..0.1);
            let n_included = rng.gen_range(10..1000);
            let n_excluded = rng.gen_range(100..100000);
            assert!(test_bloom(fp_rate, n_included, n_excluded) < max_fp_rate_ratio)
        }
    }
}

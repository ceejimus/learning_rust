#[cfg(test)]
use rand::seq::SliceRandom;
#[cfg(test)]
use rand::thread_rng;

#[cfg(test)]
const WORDS: &str = include_str!("words.txt");

// #[cfg(test)]
// pub fn get_words(n: usize) -> Vec<&'static str> {
//     let words: Vec<_> = WORDS.lines().collect();
//
//     let rng = thread_rng();
//     let words: Vec<&str> = words.choose_multiple(&mut rng, n).collect();
//
//     todo!()
// }

#[cfg(test)]
pub fn get_words(n: usize) -> Vec<&'static str> {
    let words: Vec<&str> = WORDS.lines().collect();
    let mut rng = thread_rng();
    let random_words: Vec<_> = words
        .as_slice()
        .choose_multiple(&mut rng, n)
        .cloned()
        .collect();
    random_words
}

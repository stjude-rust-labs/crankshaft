//! Name generation services.

use growable_bloom_filter::GrowableBloom;
use rand::Rng;
use rand::rngs::ThreadRng;

/// A name generator.
pub trait Generator {
    /// Generates a new name.
    fn generate(&mut self, rng: &mut impl Rng) -> String;
}

/// A unique alphanumeric name generator.
#[derive(Debug)]
pub struct UniqueAlphanumeric {
    /// The length of the randomized portion of the name.
    length: usize,
    /// Bloom filter responsible for ensuring uniqueness of these names
    bloom_filter: GrowableBloom,
}

impl Generator for UniqueAlphanumeric {
    fn generate(&mut self, rng: &mut impl Rng) -> String {
        loop {
            let random: String = rng
                .sample_iter(&rand::distr::Alphanumeric)
                .take(self.length)
                .map(char::from)
                .collect();

            if self.bloom_filter.insert(&random) {
                return random;
            }
        }
    }
}

impl UniqueAlphanumeric {
    /// Default construction for a UniqueAlphanumeric generator with a given
    /// estimated amount of generations it will need to complete.
    pub fn default_with_expected_generations(expected: usize) -> Self {
        Self {
            length: 12,
            bloom_filter: GrowableBloom::new(0.001, expected),
        }
    }
}

/// An iterator over some generic generator
#[derive(Debug)]
pub struct GeneratorIterator<G: Generator> {
    /// The underlying generator
    generator: G,

    /// The buffer holding generated data
    buffer: Vec<String>,
}

impl<G: Generator> Iterator for GeneratorIterator<G> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(pregenerated) = self.buffer.pop() {
            Some(pregenerated)
        } else {
            self.rehydrate();
            self.buffer.pop()
        }
    }
}

impl<G: Generator> GeneratorIterator<G> {
    /// Creates a new [`GeneratorIterator`] with the provided capacity.
    pub fn new(generator: G, capacity: usize) -> Self {
        Self {
            generator,
            buffer: Vec::with_capacity(capacity),
        }
    }

    /// Rehydrates the underlying buffer by repeatedly generating unique
    /// alphanumeric strings.
    fn rehydrate(&mut self) {
        let mut rng = ThreadRng::default();
        for _ in 0..self.buffer.capacity() {
            let generated = self.generator.generate(&mut rng);
            self.buffer.push(generated);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::GeneratorIterator;
    use super::UniqueAlphanumeric;

    #[test]
    fn unique_alphanumeric_generator_is_truly_unique() {
        let alphanumeric: UniqueAlphanumeric =
            UniqueAlphanumeric::default_with_expected_generations(100_000);
        let mut generator: GeneratorIterator<_> = GeneratorIterator::new(alphanumeric, 100_000);
        let _ = generator.next();
        let unique = HashSet::<&String>::from_iter(generator.buffer.iter());

        assert_eq!(generator.buffer.len(), unique.len())
    }

    #[test]
    fn unique_alphanumeric_generator_rehydrates_when_empty() {
        let alphanumeric: UniqueAlphanumeric =
            UniqueAlphanumeric::default_with_expected_generations(10);
        let mut generator: GeneratorIterator<_> = GeneratorIterator::new(alphanumeric, 10);

        for _ in 0..10 {
            assert!(generator.next().is_some())
        }

        assert!(generator.next().is_some())
    }
}

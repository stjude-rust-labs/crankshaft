//! Name generation services.

use rand::rngs::ThreadRng;
use rand::Rng as _;

/// A name generator.
pub trait Generator {
    /// Generates a new name.
    fn generate(&self) -> String;
}

/// An alphanumeric name generator.
pub struct Alphanumeric {
    /// The length of the randomized portion of the name.
    length: usize,
}

impl Default for Alphanumeric {
    fn default() -> Self {
        Self { length: 12 }
    }
}

impl Generator for Alphanumeric {
    fn generate(&self) -> String {
        let mut rng = ThreadRng::default();

        let random: String = (&mut rng)
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(self.length) // Generate 12 alphanumeric characters
            .map(char::from)
            .collect();

        format!("job-{}", random)
    }
}

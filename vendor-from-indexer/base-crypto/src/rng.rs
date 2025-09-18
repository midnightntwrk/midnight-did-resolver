//! Extension traits for specific [Rng]s.

use rand::rngs::{OsRng, StdRng};
use rand::{Rng, RngCore, SeedableRng};

/// A [Rng] that can be split. This is *not* the same as [Clone], as the
/// resulting instance is guarnateed to produce independant random values from
/// `self`.
///
/// Due to trait limitations, this does not have a blanket implementation for
/// and [SeedableRng], but may be implemented for any as needed.
pub trait SplittableRng: Rng {
    /// Generates a separate instance of `Self` from a random number generator,
    /// which is guaranteed to produce data that is independent of the data
    /// `self` generates in the future.
    fn split(&mut self) -> Self;
}

impl SplittableRng for OsRng {
    fn split(&mut self) -> Self {
        OsRng
    }
}

trait SplittableMarker {}

impl SplittableMarker for StdRng {}

impl<R: SeedableRng + RngCore + SplittableMarker> SplittableRng for R {
    fn split(&mut self) -> Self {
        Self::from_rng(self).expect("Rng must survive splitting!")
    }
}

//! Circuit for the RSA challenge.

#![deny(missing_docs)]

use halo2_proofs::{
    circuit::{Layouter, Value},
    halo2curves::bn256::Fr,
    plonk::{Circuit, ConstraintSystem, Error},
    standard_plonk::StandardPlonk,
};

#[cfg(test)]
mod tests;
pub mod utils;

/// Circuit representing the RSA challenge.
///
/// There are two advices (private inputs): `p`, and `q` and two instances (public inputs): `n` and `account`. The
/// values should satisfy: `p * q = n`. The `account` instance is artificially included in the circuit to prevent
/// front running attacks.
///
/// Since `account` is usually 256 bits long (`[u8 ;32]`), we need to split it into two 128 bit chunks, so that we have
/// certainty that both can be safely decoded as `Fr` elements.
#[derive(Default)]
pub struct RsaChallenge {
    /// First prime factor of the challenge.
    p: Option<Fr>,
    /// Second prime factor of the challenge.
    q: Option<Fr>,
}

impl Circuit<Fr> for RsaChallenge {
    type Config = <StandardPlonk as Circuit<Fr>>::Config;
    type FloorPlanner = <StandardPlonk as Circuit<Fr>>::FloorPlanner;

    fn without_witnesses(&self) -> Self {
        RsaChallenge::default()
    }

    fn configure(meta: &mut ConstraintSystem<Fr>) -> Self::Config {
        StandardPlonk::configure(meta)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<Fr>,
    ) -> Result<(), Error> {
        layouter.assign_region(
            || "",
            |mut region| {
                region.assign_advice(|| "", config.a, 0, || Value::known(self.p.unwrap()))?;
                region.assign_advice(|| "", config.b, 0, || Value::known(self.q.unwrap()))?;
                region.assign_fixed(|| "", config.q_ab, 0, || Value::known(-Fr::one()))?;
                Ok(())
            },
        )
    }
}

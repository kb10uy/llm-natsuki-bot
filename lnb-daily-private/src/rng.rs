use std::{convert::Infallible, marker::PhantomData};

use rand::{Rng, SeedableRng, TryRng, rngs::StdRng};
use sha2::{Digest, Sha256};

/// 乱数列のシードを一意に定められる粒度。
pub trait RngGranularity {
    /// この粒度を一意に定めるバイト列。
    fn seed_source(&self) -> impl AsRef<[u8]>;
}

/// 粒度 `G` に固定された乱数生成器。
///
/// `G` の値からしか生成できず、`SaltedRng<G>` を要求する API に別の粒度の乱数生成器を
/// 渡すこともできない。これによって「この乱数列は `G` だけで決まる」ことを型で表明する。
pub struct SaltedRng<G> {
    inner: StdRng,
    granularity: PhantomData<fn() -> G>,
}

impl<G: RngGranularity> SaltedRng<G> {
    /// ソルトと粒度の値からシードを導出する。
    pub fn new(salt: &str, granularity: &G) -> SaltedRng<G> {
        let mut hasher = Sha256::new();
        hasher.update(salt);
        hasher.update(granularity.seed_source());
        SaltedRng {
            inner: StdRng::from_seed(hasher.finalize().into()),
            granularity: PhantomData,
        }
    }
}

impl<G> TryRng for SaltedRng<G> {
    type Error = Infallible;

    fn try_next_u32(&mut self) -> Result<u32, Infallible> {
        Ok(self.inner.next_u32())
    }

    fn try_next_u64(&mut self) -> Result<u64, Infallible> {
        Ok(self.inner.next_u64())
    }

    fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Infallible> {
        self.inner.fill_bytes(dst);
        Ok(())
    }
}

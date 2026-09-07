use std::{convert::Infallible, marker::PhantomData};

use rand::{Rng, SeedableRng, TryRng, rngs::StdRng};
use sha2::{Digest, Sha256};

/// 乱数列のシードを一意に定められる粒度。
pub trait RngGranularity {
    /// 粒度の種別を表す名前。
    const NAME: &'static str;

    /// この粒度を一意に定めるバイト列。
    fn seed_source(&self) -> impl AsRef<[u8]>;
}

/// 乱数を引く側のドメイン。
///
/// ドメインが違えば独立した乱数列になるので、あるドメインで引く回数が変わっても
/// 他のドメインの結果には影響しない。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RngDomain {
    Schedule,
    Menstruation,
    Temperature,
    Masturbation,
    Underwear,
}

/// 粒度 `G` に固定された乱数の源。
///
/// `G` の値からしか生成できず、`RngSource<G>` を要求する API に別の粒度の源を
/// 渡すこともできない。これによって「ここから導出される乱数列は `G` だけで決まる」ことを
/// 型で表明する。
pub struct RngSource<G> {
    seeded: Sha256,
    granularity: PhantomData<fn() -> G>,
}

/// [`RngSource`] からドメインごとに導出された乱数生成器。
pub struct SaltedRng<G> {
    inner: StdRng,
    granularity: PhantomData<fn() -> G>,
}

impl RngDomain {
    fn as_bytes(&self) -> &'static [u8] {
        match self {
            RngDomain::Schedule => b"schedule",
            RngDomain::Menstruation => b"menstruation",
            RngDomain::Temperature => b"temperature",
            RngDomain::Masturbation => b"masturbation",
            RngDomain::Underwear => b"underwear",
        }
    }
}

impl<G: RngGranularity> RngSource<G> {
    /// ソルトと粒度の値から乱数の源を作る。
    pub fn new(salt: &str, granularity: &G) -> RngSource<G> {
        let mut seeded = Sha256::new();
        seeded.update(salt);
        seeded.update(G::NAME);
        seeded.update(granularity.seed_source());
        RngSource {
            seeded,
            granularity: PhantomData,
        }
    }

    /// ドメインに対応する乱数生成器を導出する。
    pub fn derive(&self, domain: RngDomain) -> SaltedRng<G> {
        let mut hasher = self.seeded.clone();
        hasher.update(domain.as_bytes());
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

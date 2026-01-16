use std::hash::BuildHasher;

pub trait NamedHasher: BuildHasher {
    const NAME: &'static str;
}

type SipHashBuilder = std::collections::hash_map::RandomState;
pub struct Sip(pub SipHashBuilder);
impl BuildHasher for Sip {
    type Hasher = <SipHashBuilder as BuildHasher>::Hasher;
    fn build_hasher(&self) -> Self::Hasher {
        self.0.build_hasher()
    }
}
impl NamedHasher for Sip {
    const NAME: &'static str = "SipHash";
}

type AHashBuilder = ahash::RandomState;
pub struct AHash(pub AHashBuilder);
impl BuildHasher for AHash {
    type Hasher = <AHashBuilder as BuildHasher>::Hasher;
    fn build_hasher(&self) -> Self::Hasher {
        self.0.build_hasher()
    }
}
impl NamedHasher for AHash {
    const NAME: &'static str = "AHash";
}

type FxHashBuilder = fxhash::FxBuildHasher;
pub struct FxHash(pub FxHashBuilder);
impl BuildHasher for FxHash {
    type Hasher = <FxHashBuilder as BuildHasher>::Hasher;
    fn build_hasher(&self) -> Self::Hasher {
        self.0.build_hasher()
    }
}
impl NamedHasher for FxHash {
    const NAME: &'static str = "FxHash";
}

type NoHashU64Builder = nohash_hasher::BuildNoHashHasher<u64>;
pub struct NoHashU64(pub NoHashU64Builder);
impl BuildHasher for NoHashU64 {
    type Hasher = <NoHashU64Builder as BuildHasher>::Hasher;
    fn build_hasher(&self) -> Self::Hasher {
        self.0.build_hasher()
    }
}
impl NamedHasher for NoHashU64 {
    const NAME: &'static str = "NoHashU64";
}

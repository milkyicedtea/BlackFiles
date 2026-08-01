use rand::random;
use sha2::{Digest, Sha256};

pub(crate) fn random_hex<const N: usize>() -> String
where
    rand::distributions::Standard: rand::distributions::Distribution<[u8; N]>,
{
    hex::encode(random::<[u8; N]>())
}

pub(crate) fn sha256_hex(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    hex::encode(hasher.finalize())
}

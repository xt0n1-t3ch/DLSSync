use crate::CatalogError;
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgo {
    Sha256,
    Md5,
}

impl HashAlgo {
    pub fn from_hex_len(s: &str) -> Option<Self> {
        if !s.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        match s.len() {
            64 => Some(HashAlgo::Sha256),
            32 => Some(HashAlgo::Md5),
            _ => None,
        }
    }
}

pub fn hash_file_with(path: &Path, algo: HashAlgo) -> Result<String, CatalogError> {
    match algo {
        HashAlgo::Sha256 => hex_sha256_file(path),
        HashAlgo::Md5 => hex_md5_file(path),
    }
}

pub fn hex_md5_file(path: &Path) -> Result<String, CatalogError> {
    use md5::{Digest as _, Md5};
    let mut f = std::fs::File::open(path)?;
    let mut h = Md5::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    let digest = h.finalize();
    let mut s = String::with_capacity(digest.len() * 2);
    for b in digest {
        let _ = write!(s, "{:02x}", b);
    }
    Ok(s)
}

pub fn hex_md5(bytes: &[u8]) -> String {
    use md5::{Digest as _, Md5};
    let mut h = Md5::new();
    h.update(bytes);
    let digest = h.finalize();
    let mut s = String::with_capacity(digest.len() * 2);
    for b in digest {
        let _ = write!(s, "{:02x}", b);
    }
    s
}

pub fn hex_sha256(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    let digest = h.finalize();
    let mut s = String::with_capacity(digest.len() * 2);
    for b in digest {
        let _ = write!(s, "{:02x}", b);
    }
    s
}

pub fn hex_sha256_file(path: &Path) -> Result<String, CatalogError> {
    let mut f = std::fs::File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    let digest = h.finalize();
    let mut s = String::with_capacity(digest.len() * 2);
    for b in digest {
        let _ = write!(s, "{:02x}", b);
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_algo_from_hex_len() {
        assert_eq!(
            HashAlgo::from_hex_len(&"a".repeat(64)),
            Some(HashAlgo::Sha256)
        );
        assert_eq!(HashAlgo::from_hex_len(&"a".repeat(32)), Some(HashAlgo::Md5));
        assert_eq!(HashAlgo::from_hex_len(&"a".repeat(40)), None);
        assert_eq!(HashAlgo::from_hex_len(""), None);
        assert_eq!(HashAlgo::from_hex_len(&"z".repeat(64)), None);
    }

    #[test]
    fn hex_sha256_matches_known_vector() {
        assert_eq!(
            hex_sha256(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn hex_md5_matches_known_vector() {
        assert_eq!(hex_md5(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
    }
}

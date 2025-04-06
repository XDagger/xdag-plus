use ripemd::{Digest, Ripemd160};
use sha2::Sha256;

// generate address bytes from public key
pub fn hash160(input: &[u8]) -> [u8; 20] {
    let mut hasher = Sha256::new();
    hasher.update(input);
    let hash = hasher.finalize();

    let mut hasher = Ripemd160::new();
    hasher.update(hash);
    let ripemd_hash = hasher.finalize();
    // let mut hash160 = [0_u8; 20];
    // for (i, b) in ripemd_hash.into_iter().enumerate() {
    //     hash160[i] = b;
    // }
    let hash160: [u8; 20] = ripemd_hash
        .as_slice()
        .try_into()
        .expect("Invalid length for hash160");

    hash160
}

pub fn sha256(input: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(input);
    let sha1 = hasher.finalize();
    // let mut hash = [0_u8; 32];
    // for (i, b) in sha1.into_iter().enumerate() {
    //     hash[i] = b;
    // }
    let hash: [u8; 32] = sha1
        .as_slice()
        .try_into()
        .expect("Invalid length for sha256");

    hash
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_hash160() {
        let input = [0_u8; 33];
        let h = hash160(&input);
        println!("{:?}", h);
    }
    #[test]
    fn test_sha256() {
        let input = b"hello";
        let h = sha256(input);
        h.iter().for_each(|b| print!("{b:02x}"));
    }
}

pub fn keccak256(message: &[u8]) -> [u8; 32] {
    use sha3::{Digest, Keccak256};

    let mut hasher = Keccak256::new();
    hasher.update(message);
    hasher.finalize().into()
}

pub fn blake2b256(message: &[u8]) -> [u8; 32] {
    use blake2::{
        digest::{Update, VariableOutput},
        VarBlake2b,
    };
    use std::convert::TryInto;

    let mut hasher = VarBlake2b::new(32).unwrap();
    hasher.update(message);
    let mut arr = BytesMut::with_capacity(32);
    hasher.finalize_variable(|res| arr.put_slice(&res));
    arr
}

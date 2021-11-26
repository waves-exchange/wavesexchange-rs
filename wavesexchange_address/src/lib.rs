mod hash;

use bytes::BytesMut;
use hash::{blake2b256, keccak256};
use std::convert::TryInto;

pub struct Address([u8; 26]);

impl Address {
    pub fn from_public_key(public_key: impl AsRef<[u8; 32]>, chain_id: u8) -> Self {
        let public_key = public_key.as_ref();
        let public_key_hash: [u8; 20] = keccak256(&blake2b256(public_key))[0..20]
            .try_into()
            .unwrap();
        Self::from_public_key_hash(&public_key_hash, chain_id)
    }

    pub fn from_public_key_hash(public_key_hash: impl AsRef<[u8; 20]>, chain_id: u8) -> Self {
        let public_key_hash = public_key_hash.as_ref();

        let mut address_bytes = BytesMut::with_capacity(26); // VERSION + CHAIN_ID + PKH + checksum

        address_bytes.put_u8(1); // address version is always 1
        address_bytes.put_u8(chain_id);
        address_bytes.put_slice(public_key_hash[..20]);

        let checksum = keccak256(&blake2b256(&address_bytes[..22]))[..4];

        address_bytes.put_slice(checksum);

        Address(address_bytes.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use

    #[test]
    fn address_from_public_key_hash() {
        assert_eq!(add(2, 3), 5);
    }

    #[test]
    fn address_to_string() {
        assert_eq!(add(2, 3), 5);
    }
}

// #[test]
// fn address_from_public_key() {

// }

// pub fn address_from_pubkey_hash()

// recipient::Recipient::PublicKeyHash(ref pkh) => {
//     let mut addr = BytesMut::with_capacity(26); // VERSION + CHAIN_ID + PKH + checksum

//     addr.put_u8(1); // address version is always 1
//     addr.put_u8(chain_id);
//     addr.put_slice(&pkh[..20]);

//     let chks = &keccak256(&blake2b256(&addr[..22]))[..4];

//     addr.put_slice(chks);

//     TransferParticipant::Address(bs58::encode(addr).into_string())

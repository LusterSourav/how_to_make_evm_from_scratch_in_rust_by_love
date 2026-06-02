use alloc::vec::Vec;

use bare_metal_evm_rlp::{decode_strict, encode_list, encode_str, encode_u256, RlpItem};
use bare_metal_evm_types::U256;

/// `keccak256(b"")` — hash of empty EVM bytecode.
pub const EMPTY_CODE_HASH: [u8; 32] = [
    0xc5, 0xd2, 0x46, 0x01, 0x86, 0xf7, 0x23, 0x3c, 0x92, 0x7e, 0x7d, 0xb2, 0xdc, 0xc7, 0x03, 0xc0,
    0xe5, 0x00, 0xb6, 0x53, 0xca, 0x82, 0x27, 0x3b, 0x7b, 0xfa, 0xd8, 0x04, 0x5d, 0x85, 0xa4, 0x70,
];

/// An Ethereum account as stored in the state trie.
///
/// RLP encoding: `[nonce, balance, storage_root, code_hash]`
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Account {
    pub nonce: U256,
    pub balance: U256,
    pub storage_root: [u8; 32],
    pub code_hash: [u8; 32],
}

impl Account {
    /// Create a new empty account.
    #[must_use]
    pub const fn new_empty() -> Self {
        Self {
            nonce: U256::zero(),
            balance: U256::zero(),
            storage_root: bare_metal_evm_trie::EMPTY_ROOT_HASH,
            code_hash: EMPTY_CODE_HASH,
        }
    }
}

impl Default for Account {
    fn default() -> Self {
        Self::new_empty()
    }
}

impl Account {
    /// Encode this account as RLP bytes.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let nonce_rlp = encode_u256(&self.nonce);
        let balance_rlp = encode_u256(&self.balance);
        let storage_root_rlp = encode_str(&self.storage_root);
        let code_hash_rlp = encode_str(&self.code_hash);
        encode_list(&[&nonce_rlp, &balance_rlp, &storage_root_rlp, &code_hash_rlp])
    }

    /// Decode an account from RLP bytes.
    #[allow(clippy::result_unit_err, clippy::missing_errors_doc)]
    pub fn decode(data: &[u8]) -> Result<Self, ()> {
        let item = decode_strict(data).map_err(|_| ())?;
        match item {
            RlpItem::List(items) if items.len() == 4 => {
                let nonce = rlp_to_u256(&items[0])?;
                let balance = rlp_to_u256(&items[1])?;
                let storage_root = match &items[2] {
                    RlpItem::Str(s) if s.len() == 32 => {
                        let mut arr = [0u8; 32];
                        arr.copy_from_slice(s);
                        arr
                    }
                    RlpItem::Str(_) | RlpItem::List(_) => return Err(()),
                };
                let code_hash = match &items[3] {
                    RlpItem::Str(s) if s.len() == 32 => {
                        let mut arr = [0u8; 32];
                        arr.copy_from_slice(s);
                        arr
                    }
                    RlpItem::Str(_) | RlpItem::List(_) => return Err(()),
                };
                Ok(Self {
                    nonce,
                    balance,
                    storage_root,
                    code_hash,
                })
            }
            RlpItem::Str(_) | RlpItem::List(_) => Err(()),
        }
    }
}

/// Decode an RLP-encoded U256 value.
fn rlp_to_u256(item: &RlpItem) -> Result<U256, ()> {
    match item {
        RlpItem::Str(s) => {
            if s.len() > 32 {
                return Err(());
            }
            // Reject leading zeros (non-canonical RLP)
            if s.len() > 1 && s[0] == 0 {
                return Err(());
            }
            let mut bytes = [0u8; 32];
            bytes[32 - s.len()..].copy_from_slice(s);
            Ok(U256::from_bytes_be(bytes))
        }
        RlpItem::List(_) => Err(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use bare_metal_evm_keccak::keccak256;

    #[test]
    fn empty_account_roundtrip() {
        let acc = Account::new_empty();
        let encoded = acc.encode();
        let decoded = Account::decode(&encoded).unwrap();
        assert_eq!(acc, decoded);
    }

    #[test]
    fn account_with_values_roundtrip() {
        let acc = Account {
            nonce: U256::from_u64(7),
            balance: U256::from_u64(1000),
            storage_root: [0xabu8; 32],
            code_hash: [0xcdu8; 32],
        };
        let encoded = acc.encode();
        let decoded = Account::decode(&encoded).unwrap();
        assert_eq!(acc, decoded);
    }

    #[test]
    fn decode_invalid_list_length() {
        let encoded = encode_list(&[&encode_str(b"")]);
        assert!(Account::decode(&encoded).is_err());
    }

    #[test]
    fn empty_code_hash_constant() {
        let computed = keccak256(b"");
        assert_eq!(computed, EMPTY_CODE_HASH);
    }

    #[test]
    fn decode_invalid_storage_root_type() {
        let nonce_rlp = encode_u256(&U256::zero());
        let balance_rlp = encode_u256(&U256::zero());
        let bad_storage_root = encode_list(&[&encode_str(b"")]);
        let code_hash_rlp = encode_str(&[0xcdu8; 32]);
        let encoded = encode_list(&[&nonce_rlp, &balance_rlp, &bad_storage_root, &code_hash_rlp]);
        assert!(Account::decode(&encoded).is_err());
    }

    #[test]
    fn rlp_to_u256_edge_cases() {
        let acc = Account {
            nonce: bare_metal_evm_types::U256_MAX,
            balance: U256::zero(),
            storage_root: [0xabu8; 32],
            code_hash: [0xcdu8; 32],
        };
        let encoded = acc.encode();
        let decoded = Account::decode(&encoded).unwrap();
        assert_eq!(acc, decoded);
    }

    #[test]
    fn rlp_to_u256_rejects_leading_zeros() {
        use bare_metal_evm_rlp::encode_list;

        let nonce_rlp = encode_u256(&U256::zero());
        // Manually construct balance with leading zero byte 0x82,0x00,0x01
        let balance_rlp = vec![0x82, 0x00, 0x01];
        let storage_root = [0xabu8; 32];
        let storage_root_rlp = encode_str(&storage_root);
        let code_hash = [0xcdu8; 32];
        let code_hash_rlp = encode_str(&code_hash);
        let encoded = encode_list(&[&nonce_rlp, &balance_rlp, &storage_root_rlp, &code_hash_rlp]);
        assert!(Account::decode(&encoded).is_err());
    }
}

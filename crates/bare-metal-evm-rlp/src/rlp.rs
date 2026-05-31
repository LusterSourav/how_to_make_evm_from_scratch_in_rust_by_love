// Bare Metal EVM — RLP Encoding/Decoding (Layer 1)
// Recursive Length Prefix — Ethereum's universal serialization format.
//
// Encoding rules (five prefix ranges):
//   0x00–0x7f : Single byte (self-encoding)
//   0x80–0xb7 : Short string (0–55 bytes)
//   0xb8–0xbf : Long string (55+ bytes)
//   0xc0–0xf7 : Short list (payload 0–55 bytes)
//   0xf8–0xff : Long list (payload 55+ bytes)
//
// Strict minimalism: integers must be big-endian with no leading zeros.
// Reference: Ethereum Yellow Paper, Appendix B

use alloc::vec::Vec;
use core::fmt;

// RLP item — decode result

/// A decoded RLP item — either a string (byte slice) or a list of sub-items.
#[derive(Clone, Debug, PartialEq)]
pub enum RlpItem<'a> {
    Str(&'a [u8]),
    List(Vec<RlpItem<'a>>),
}

// Errors

/// Errors that can occur during RLP decoding.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum RlpError {
    /// Input is too short to contain the declared payload.
    Truncated,
    /// Malformed length prefix.
    InvalidLength,
    /// Leading zeros in an integer encoding (strict minimalism violation).
    LeadingZeros,
    /// Input contains trailing data after the decoded item.
    TrailingData,
    /// Decoding exceeded maximum nesting depth.
    TooDeep,
}

impl core::error::Error for RlpError {}

impl fmt::Display for RlpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RlpError::Truncated => write!(f, "RLP input truncated"),
            RlpError::InvalidLength => write!(f, "RLP invalid length prefix"),
            RlpError::LeadingZeros => write!(f, "RLP leading zeros (non-minimal encoding)"),
            RlpError::TrailingData => write!(f, "RLP trailing data after decoded item"),
            RlpError::TooDeep => write!(f, "RLP maximum nesting depth exceeded"),
        }
    }
}

// Encoding helpers

/// Encode the length prefix for an RLP item.
///
/// `prefix` is the base prefix (0x80 for strings, 0xc0 for lists).
fn encode_length(len: usize, prefix: u8, out: &mut Vec<u8>) {
    if len < 56 {
        // Short form: single byte prefix + length
        out.push(prefix + len as u8);
    } else {
        // Long form: prefix + 55 + len_of_len || len
        let (len_bytes, len_len) = usize_to_be_bytes(len);
        out.push(prefix + 55 + len_len as u8);
        out.extend_from_slice(&len_bytes[8 - len_len..]);
    }
}

/// Convert a `usize` to its big-endian byte representation with no leading zeros.
/// Returns (bytes_array, length) to avoid heap allocation.
fn usize_to_be_bytes(mut n: usize) -> ([u8; 8], usize) {
    if n == 0 {
        return ([0u8; 8], 1);
    }
    let mut bytes = [0u8; 8];
    let mut i = 8;
    while n > 0 {
        i -= 1;
        bytes[i] = (n & 0xFF) as u8;
        n >>= 8;
    }
    (bytes, 8 - i)
}

// Encoding — public API

/// Encode a byte string using RLP.
#[must_use]
pub fn encode_str(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();

    if bytes.len() == 1 && bytes[0] <= 0x7f {
        // Single byte in range [0x00, 0x7f] — self-encoding
        out.push(bytes[0]);
        return out;
    }

    // String encoding
    encode_length(bytes.len(), 0x80, &mut out);
    out.extend_from_slice(bytes);
    out
}

/// Encode a list of pre-encoded items using RLP.
#[must_use]
pub fn encode_list(items: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();

    // Calculate total payload length
    let total_len: usize = items.iter().map(|item| item.len()).sum();

    // List header
    encode_length(total_len, 0xc0, &mut out);

    // Concatenate items
    for item in items {
        out.extend_from_slice(item);
    }

    out
}

/// Encode a list from an iterator of (encoded) byte slices.
#[must_use]
pub fn encode_list_from_iter<'a>(items: impl IntoIterator<Item = &'a [u8]>) -> Vec<u8> {
    let mut collected = Vec::new();
    let mut total_len = 0usize;
    for item in items {
        total_len += item.len();
        collected.push(item);
    }
    let mut out = Vec::with_capacity(1 + 8 + total_len);
    encode_length(total_len, 0xc0, &mut out);
    for item in &collected {
        out.extend_from_slice(item);
    }
    out
}

// Decoding — public API

/// Maximum allowed nesting depth for RLP decoding.
const MAX_DECODE_DEPTH: usize = 128;

/// Decode a single RLP item from `input`.
///
/// Returns an error if trailing data remains after decoding.
pub fn decode_strict(input: &[u8]) -> Result<RlpItem, RlpError> {
    let (item, consumed) = decode_item(input, 0, 0)?;
    if consumed != input.len() {
        return Err(RlpError::TrailingData);
    }
    Ok(item)
}

/// Decode a single RLP item from `input`.
///
/// Silently ignores any trailing data after the decoded item.
/// For consensus-critical parsing, use `decode_strict` instead.
pub fn decode(input: &[u8]) -> Result<RlpItem, RlpError> {
    decode_item(input, 0, 0).map(|(item, _consumed)| item)
}

/// Internal recursive decoder. Returns (item, bytes_consumed).
fn decode_item(input: &[u8], offset: usize, depth: usize) -> Result<(RlpItem, usize), RlpError> {
    if depth >= MAX_DECODE_DEPTH {
        return Err(RlpError::TooDeep);
    }

    if offset >= input.len() {
        return Err(RlpError::Truncated);
    }

    let prefix = input[offset];

    // Single byte (0x00–0x7f): self-encoding
    if prefix <= 0x7f {
        return Ok((RlpItem::Str(&input[offset..offset + 1]), 1));
    }

    // String: short form (0x80–0xb7)
    if prefix <= 0xb7 {
        let len = (prefix - 0x80) as usize;
        let end = offset.checked_add(1 + len).ok_or(RlpError::InvalidLength)?;
        if end > input.len() {
            return Err(RlpError::Truncated);
        }
        // Strict minimalism: single byte in [0x00, 0x7f] must not use string form
        if len == 1 && input[offset + 1] <= 0x7f {
            return Err(RlpError::LeadingZeros);
        }
        return Ok((RlpItem::Str(&input[offset + 1..offset + 1 + len]), 1 + len));
    }

    // String: long form (0xb8–0xbf)
    if prefix <= 0xbf {
        let len_of_len = (prefix - 0xb7) as usize;
        if offset + 1 + len_of_len > input.len() {
            return Err(RlpError::Truncated);
        }
        let len = be_bytes_to_usize(&input[offset + 1..offset + 1 + len_of_len]);
        if len < 56 {
            // Must use short form for lengths < 56
            return Err(RlpError::InvalidLength);
        }
        let payload_end = (offset + 1 + len_of_len)
            .checked_add(len)
            .ok_or(RlpError::InvalidLength)?;
        if payload_end > input.len() {
            return Err(RlpError::Truncated);
        }
        return Ok((
            RlpItem::Str(&input[offset + 1 + len_of_len..payload_end]),
            1 + len_of_len + len,
        ));
    }

    // List: short form (0xc0–0xf7)
    if prefix <= 0xf7 {
        let payload_len = (prefix - 0xc0) as usize;
        let end = offset.checked_add(1 + payload_len).ok_or(RlpError::InvalidLength)?;
        if end > input.len() {
            return Err(RlpError::Truncated);
        }
        let payload = &input[offset + 1..offset + 1 + payload_len];
        let depth = depth + 1;
        let mut items = Vec::new();
        let mut inner_offset = 0;
        while inner_offset < payload.len() {
            let (item, consumed) = decode_item(payload, inner_offset, depth)?;
            items.push(item);
            inner_offset += consumed;
        }
        debug_assert_eq!(inner_offset, payload.len());
        return Ok((RlpItem::List(items), 1 + payload_len));
    }

    // List: long form (0xf8–0xff)
    // prefix is at least 0xf8 since previous checks passed
    let len_of_len = (prefix - 0xf7) as usize;
    if offset + 1 + len_of_len > input.len() {
        return Err(RlpError::Truncated);
    }
    let payload_len = be_bytes_to_usize(&input[offset + 1..offset + 1 + len_of_len]);
    if payload_len < 56 {
        return Err(RlpError::InvalidLength);
    }
    let payload_end = (offset + 1 + len_of_len)
        .checked_add(payload_len)
        .ok_or(RlpError::InvalidLength)?;
    if payload_end > input.len() {
        return Err(RlpError::Truncated);
    }
    let payload = &input[offset + 1 + len_of_len..payload_end];
    let depth = depth + 1;
    let mut items = Vec::new();
    let mut inner_offset = 0;
    while inner_offset < payload.len() {
        let (item, consumed) = decode_item(payload, inner_offset, depth)?;
        items.push(item);
        inner_offset += consumed;
    }
    debug_assert_eq!(inner_offset, payload.len());
    Ok((RlpItem::List(items), 1 + len_of_len + payload_len))
}

// Big-endian byte conversion

/// Convert a big-endian byte slice to a usize.
///
/// On 32-bit targets, silently saturates at `usize::MAX` if the input exceeds
/// 4 bytes (theoretical; Ethereum RLP never approaches such sizes).
fn be_bytes_to_usize(bytes: &[u8]) -> usize {
    let mut result = 0usize;
    for &b in bytes {
        result = result.saturating_mul(256).saturating_add(b as usize);
    }
    result
}

// Convenience: encode a U256 as RLP (big-endian, no leading zeros)

use bare_metal_evm_types::U256;

/// Encode a `U256` value to its RLP string representation.
///
/// Integers are encoded as big-endian byte strings with no leading zeros.
/// A zero value encodes as the empty string (`0x80`).
#[must_use]
pub fn encode_u256(val: &U256) -> Vec<u8> {
    let be_bytes = val.to_bytes_be();
    // Strip leading zeros
    let significant_start = be_bytes
        .iter()
        .position(|&b| b != 0)
        .unwrap_or(be_bytes.len());
    if significant_start == be_bytes.len() {
        // Value is zero — encode as empty string
        encode_str(b"")
    } else {
        encode_str(&be_bytes[significant_start..])
    }
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        hex::decode(s).unwrap()
    }

    // String encoding

    #[test]
    fn encode_single_byte_self_encoding() {
        // Single byte in [0x00, 0x7f] encodes as itself
        for b in 0x00..=0x7f {
            let result = encode_str(&[b]);
            assert_eq!(result, alloc::vec![b], "failed for byte 0x{b:02x}");
        }
    }

    #[test]
    fn encode_empty_string() {
        let result = encode_str(b"");
        assert_eq!(result, alloc::vec![0x80]);
    }

    #[test]
    fn encode_dog() {
        let result = encode_str(b"dog");
        assert_eq!(result, hex("83646f67"));
    }

    #[test]
    fn encode_short_string_boundary_55() {
        // 55-byte string: short form (0x80 + 55 = 0xb7 prefix)
        let input = [0xABu8; 55];
        let result = encode_str(&input);
        assert_eq!(result.len(), 1 + 55);
        assert_eq!(result[0], 0xb7);
        assert_eq!(&result[1..], &input);
    }

    #[test]
    fn encode_long_string_56() {
        // 56-byte string: long form (0xb8 + 1 byte of length)
        let input = [0xCDu8; 56];
        let result = encode_str(&input);
        assert_eq!(result.len(), 1 + 1 + 56);
        assert_eq!(result[0], 0xb8);
        assert_eq!(result[1], 56);
        assert_eq!(&result[2..], &input);
    }

    #[test]
    fn encode_long_string_257() {
        // 257-byte string: long form (0xb9 + 2 bytes of length)
        let input = [0xEFu8; 257];
        let result = encode_str(&input);
        assert_eq!(result.len(), 1 + 2 + 257);
        assert_eq!(result[0], 0xb9);
        assert_eq!(result[1], 0x01);
        assert_eq!(result[2], 0x01);
    }

    // List encoding

    #[test]
    fn encode_empty_list() {
        let result = encode_list(&[]);
        assert_eq!(result, alloc::vec![0xc0]);
    }

    #[test]
    fn encode_list_cat_dog() {
        let cat = encode_str(b"cat");
        let dog = encode_str(b"dog");
        let result = encode_list(&[&cat, &dog]);
        assert_eq!(result, hex("c88363617483646f67"));
    }

    #[test]
    fn encode_short_list_boundary_55() {
        // 55-byte payload: short form
        let items: Vec<Vec<u8>> = (0..55).map(|_| encode_str(&[0x00])).collect();
        let refs: Vec<&[u8]> = items.iter().map(|v| v.as_slice()).collect();
        let result = encode_list(&refs);
        assert_eq!(result[0], 0xf7);
    }

    #[test]
    fn encode_long_list_56_payload() {
        // 56-byte payload: long form
        let items: Vec<Vec<u8>> = (0..56).map(|_| encode_str(&[0x00])).collect();
        let refs: Vec<&[u8]> = items.iter().map(|v| v.as_slice()).collect();
        let result = encode_list(&refs);
        assert_eq!(result[0], 0xf8);
        assert_eq!(result[1], 56);
    }

    #[test]
    fn encode_nested_list() {
        // [[], [[]], [[], []]]
        let empty = encode_list(&[]);
        let inner_single = encode_list(&[&empty]);
        let inner_double = encode_list(&[&empty, &empty]);
        let nested = encode_list(&[&empty, &inner_single, &inner_double]);
        // empty = [0xc0] (1 byte), inner_single = [0xc1, 0xc0] (2 bytes),
        // inner_double = [0xc2, 0xc0, 0xc0] (3 bytes)
        // total payload = 6 bytes -> prefix 0xc6
        assert_eq!(nested, hex("c6c0c1c0c2c0c0"));
    }

    // Decoding

    #[test]
    fn decode_single_byte() {
        for b in 0x00..=0x7fu8 {
            let input = [b];
            let item = decode(&input).unwrap();
            assert_eq!(item, RlpItem::Str(&input));
        }
    }

    #[test]
    fn decode_empty_string() {
        let item = decode(&[0x80]).unwrap();
        assert_eq!(item, RlpItem::Str(b""));
    }

    #[test]
    fn decode_dog() {
        let encoded = hex("83646f67");
        let item = decode(&encoded).unwrap();
        assert_eq!(item, RlpItem::Str(b"dog"));
    }

    #[test]
    fn decode_empty_list() {
        let item = decode(&[0xc0]).unwrap();
        assert_eq!(item, RlpItem::List(alloc::vec![]));
    }

    #[test]
    fn decode_list_cat_dog() {
        let encoded = hex("c88363617483646f67");
        let item = decode(&encoded).unwrap();
        assert_eq!(
            item,
            RlpItem::List(alloc::vec![RlpItem::Str(b"cat"), RlpItem::Str(b"dog")])
        );
    }

    #[test]
    fn decode_nested_list() {
        let encoded = hex("c6c0c1c0c2c0c0");
        let item = decode(&encoded).unwrap();
        let empty = RlpItem::List(alloc::vec![]);
        assert_eq!(
            item,
            RlpItem::List(alloc::vec![
                empty.clone(),
                RlpItem::List(alloc::vec![empty.clone()]),
                RlpItem::List(alloc::vec![empty.clone(), empty.clone()]),
            ])
        );
    }

    // Strict minimalism checks

    #[test]
    fn reject_leading_zero_in_string() {
        // 0x81 0x05 means "string of length 1, value is 0x05"
        // But 0x05 <= 0x7f, so it should be self-encoding -> reject
        let result = decode(&[0x81, 0x05]);
        assert_eq!(result, Err(RlpError::LeadingZeros));
    }

    #[test]
    fn reject_long_form_when_short_possible() {
        // 0xb8 + 55 means "long form string of length 55 with 1-byte length"
        // Length < 56 must use short form (0xb7 + 55 bytes)
        let mut encoded = alloc::vec![0xb8, 55];
        encoded.extend_from_slice(&[0x00u8; 55]);
        assert_eq!(decode(&encoded), Err(RlpError::InvalidLength));
    }

    #[test]
    fn decode_long_string() {
        let input = [0xBBu8; 56];
        let encoded = encode_str(&input);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded, RlpItem::Str(&input[..]));
    }

    // Round-trip tests

    #[test]
    fn roundtrip_string() {
        let test_cases: &[&[u8]] = &[
            b"",
            b"\x00",
            b"\x7f",
            b"\x80",
            b"\xff",
            b"hello",
            b"dog",
            &[0xABu8; 55],
            &[0xCDu8; 56],
            &[0xEFu8; 1000],
        ];
        for input in test_cases {
            let encoded = encode_str(input);
            let decoded = decode(&encoded).unwrap();
            assert_eq!(decoded, RlpItem::Str(input), "failed for {input:?}");
        }
    }

    #[test]
    fn roundtrip_list() {
        let test_cases: Vec<Vec<Vec<u8>>> = alloc::vec![
            alloc::vec![],
            alloc::vec![encode_str(b"a")],
            alloc::vec![encode_str(b"cat"), encode_str(b"dog")],
            alloc::vec![encode_list(&[]), encode_list(&[&encode_str(b"inner")])],
        ];
        for items in &test_cases {
            let refs: Vec<&[u8]> = items.iter().map(|v| v.as_slice()).collect();
            let encoded = encode_list(&refs);
            let decoded = decode(&encoded).unwrap();
            let expected_items: Vec<RlpItem> = items
                .iter()
                .map(|encoded| decode(encoded).unwrap())
                .collect();
            assert_eq!(decoded, RlpItem::List(expected_items));
        }
    }

    // Error cases

    #[test]
    fn decode_truncated_string() {
        // Claims 10 bytes but only 5 available
        let encoded = [0x8a, 0x01, 0x02, 0x03, 0x04, 0x05];
        let result = decode(&encoded);
        assert_eq!(result, Err(RlpError::Truncated));
    }

    #[test]
    fn decode_truncated_length() {
        // Long string prefix with missing length bytes
        let encoded = [0xb9]; // claims 2-byte length, but no length follows
        let result = decode(&encoded);
        assert_eq!(result, Err(RlpError::Truncated));
    }

    // Strict decoding with trailing data checks

    #[test]
    fn decode_strict_accepts_exact_input() {
        let encoded = hex("83646f67");
        let item = decode_strict(&encoded).unwrap();
        assert_eq!(item, RlpItem::Str(b"dog"));
    }

    #[test]
    fn decode_strict_rejects_trailing_data() {
        // Valid empty list followed by garbage byte
        let encoded = [0xc0, 0xde, 0xad];
        let result = decode_strict(&encoded);
        assert_eq!(result, Err(RlpError::TrailingData));
    }

    #[test]
    fn decode_ignores_trailing_data() {
        // decode (non-strict) should still work with trailing garbage
        let encoded = [0xc0, 0xde, 0xad];
        let item = decode(&encoded).unwrap();
        assert_eq!(item, RlpItem::List(alloc::vec![]));
    }

    #[test]
    fn decode_strict_empty_input() {
        let result = decode_strict(b"");
        assert_eq!(result, Err(RlpError::Truncated));
    }

    // Recursion depth limits

    #[test]
    fn decode_too_deep_nesting() {
        let mut encoded = alloc::vec![0xc0u8];
        for _ in 0..MAX_DECODE_DEPTH + 10 {
            let payload_len = encoded.len();
            let mut outer = Vec::new();
            if payload_len < 56 {
                outer.push(0xc0 + payload_len as u8);
            } else {
                let (len_bytes, len_len) = usize_to_be_bytes(payload_len);
                outer.push(0xf7 + len_len as u8);
                outer.extend_from_slice(&len_bytes[8 - len_len..]);
            }
            outer.extend_from_slice(&encoded);
            encoded = outer;
        }
        let result = decode_strict(&encoded);
        assert_eq!(result, Err(RlpError::TooDeep));
    }

    #[test]
    fn decode_acceptable_nesting() {
        let inner = encode_list(&[]);
        let mut encoded = encode_list(&[&inner]);
        for _ in 0..10 {
            encoded = encode_list(&[&encoded]);
        }
        let result = decode_strict(&encoded);
        assert!(result.is_ok());
    }

    // U256 encoding

    #[test]
    fn encode_u256_zero() {
        let val = U256::zero();
        let result = encode_u256(&val);
        assert_eq!(result, alloc::vec![0x80]);
    }

    #[test]
    fn encode_u256_one() {
        let val = U256::one();
        let result = encode_u256(&val);
        assert_eq!(result, alloc::vec![0x01]);
    }

    #[test]
    fn encode_u256_small() {
        // 0x0100 = 256 in big-endian is [0x01, 0x00] (2 bytes, no leading zeros)
        let val = U256::from_u64(256);
        let result = encode_u256(&val);
        assert_eq!(result, hex("820100"));
    }

    #[test]
    fn encode_u256_limb0_only() {
        // U256 with only the least-significant limb set
        let val = U256::from_limbs(0x0102030405060708, 0, 0, 0);
        let result = encode_u256(&val);
        // Big-endian: 01 02 03 04 05 06 07 08 (8 bytes)
        // 0x80 + 8 = 0x88 -> short string
        assert_eq!(result, hex("880102030405060708"));
    }

    #[test]
    fn encode_u256_large_mid_limb_preserves_trailing_zeros() {
        // U256 with only the middle-upper limb set = 0x01 * 2^128
        let val = U256::from_limbs(0, 0, 0x01, 0);
        let result = encode_u256(&val);
        // BE bytes: [0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,1, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0]
        // Strip leading zeros: [1, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0] (17 bytes)
        // 0x80 + 17 = 0x91 -> short string
        let mut expected = alloc::vec![0x91, 0x01];
        expected.extend_from_slice(&[0u8; 16]);
        assert_eq!(result, expected);
    }

    // Long-form list encoding (>255 byte payload)

    #[test]
    fn encode_long_list_over_255() {
        // 256-byte payload → 0xf9 prefix + 2-byte length (0x01, 0x00)
        let items: Vec<Vec<u8>> = (0..256).map(|_| encode_str(&[0x00])).collect();
        let refs: Vec<&[u8]> = items.iter().map(|v| v.as_slice()).collect();
        let result = encode_list(&refs);
        assert_eq!(result[0], 0xf9);
        assert_eq!(result[1], 0x01);
        assert_eq!(result[2], 0x00);
        assert_eq!(result.len(), 1 + 2 + 256);
    }

    #[test]
    fn encode_u256_max() {
        let val = U256([!0u64; 4]);
        let result = encode_u256(&val);
        // U256::MAX to_bytes_be = [0xFF; 32], RLP short string = 0xa0 prefix + 32 bytes
        let mut expected = alloc::vec![0xa0];
        expected.extend_from_slice(&[0xFFu8; 32]);
        assert_eq!(result, expected);
    }
}

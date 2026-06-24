/// RLP encoding and decoding with basic string/list roundtrips.
///
/// Run with: cargo run --example rlp_encode_decode
extern crate bare_metal_evm;

use bare_metal_evm::{
    decode, decode_strict, encode_list, encode_str, encode_u256, RlpError, RlpItem, U256,
};

fn main() -> Result<(), RlpError> {
    // --- String encoding ---
    let encoded = encode_str(b"dog");
    println!("1. 'dog' encoded: {encoded:02x?}");
    let decoded = decode(&encoded)?;
    assert_eq!(decoded, RlpItem::Str(b"dog"));
    if let RlpItem::Str(s) = &decoded {
        println!("   Decoded: {s:?}");
    }

    // --- U256 encoding ---
    let val = U256::from_u64(0x0100);
    let encoded = encode_u256(&val);
    println!("2. U256(0x0100) encoded: {encoded:02x?}");
    let decoded = decode(&encoded)?;
    // decode doesn't interpret as U256 — compare raw bytes
    if let RlpItem::Str(bytes) = &decoded {
        println!("   Decoded bytes: {bytes:02x?}");
    }

    // --- List encoding ---
    let cat = encode_str(b"cat");
    let dog = encode_str(b"dog");
    let items: [&[u8]; 2] = [&cat, &dog];
    let list = encode_list(&items);
    println!("3. ['cat', 'dog'] list: {list:02x?}");
    let decoded = decode(&list)?;
    if let RlpItem::List(inner) = &decoded {
        assert_eq!(inner.len(), 2);
        assert_eq!(inner[0], RlpItem::Str(b"cat"));
        assert_eq!(inner[1], RlpItem::Str(b"dog"));
        println!("   Decoded: cat + dog");
    }

    // --- Empty string ---
    let empty = encode_str(b"");
    println!("4. Empty string: {empty:02x?}");
    let decoded = decode(&empty)?;
    assert_eq!(decoded, RlpItem::Str(b""));
    println!("   Decoded empty string");

    // --- Nested lists ---
    let inner_encoded = encode_str(b"a");
    let inner_list = encode_list(&[&inner_encoded]);
    let outer_list = encode_list(&[&inner_list]);
    println!("5. Nested [['a']]: {outer_list:02x?}");
    let decoded = decode(&outer_list)?;
    if let RlpItem::List(outer) = &decoded {
        if let RlpItem::List(inner) = &outer[0] {
            assert_eq!(inner[0], RlpItem::Str(b"a"));
            println!("   Decoded inner: a");
        }
    }

    // --- Strict decode rejects trailing data ---
    let mut buf = encode_str(b"dog");
    buf.push(0xff);
    assert!(decode_strict(&buf).is_err());
    println!("6. Strict decode correctly rejected trailing data");

    println!("7. All RLP checks passed!");
    Ok(())
}

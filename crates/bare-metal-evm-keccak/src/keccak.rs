// Bare Metal EVM — Keccak-256 Sponge (Layer 1)
// A zero-dependency implementation of the Keccak-256 hash function
// used by Ethereum. Uses the 24-round Keccak-f[1600] permutation
// with the Ethereum-specific padding rule (0x01 suffix, not NIST SHA-3 0x06).
//
// Reference: Keccak Reference v3.0, https://keccak.team/files/Keccak-reference-3.0.pdf

// Constants

/// Rate for Keccak-256: 1088 bits = 136 bytes.
const RATE: usize = 136;

/// 24 round constants for Keccak-f[1600].
const RC: [u64; 24] = [
    0x0000000000000001,
    0x0000000000008082,
    0x800000000000808A,
    0x8000000080008000,
    0x000000000000808B,
    0x0000000080000001,
    0x8000000080008081,
    0x8000000000008009,
    0x000000000000008A,
    0x0000000000000088,
    0x0000000080008009,
    0x000000008000000A,
    0x000000008000808B,
    0x800000000000008B,
    0x8000000000008089,
    0x8000000000008003,
    0x8000000000008002,
    0x8000000000000080,
    0x000000000000800A,
    0x800000008000000A,
    0x8000000080008081,
    0x8000000000008080,
    0x0000000080000001,
    0x8000000080008008,
];

/// Rotation offsets for the ρ step, indexed as [x][y].
const RHO: [[u32; 5]; 5] = [
    [0, 36, 3, 41, 18],
    [1, 44, 10, 45, 2],
    [62, 6, 43, 15, 61],
    [28, 55, 25, 21, 56],
    [27, 20, 39, 8, 14],
];

// Lane permutation mapping for π step

/// Apply the π permutation: new position (y, (2x+3y)%5) ← old position (x, y).
/// Returns the destination column for a source at (x, y).
#[inline]
const fn pi_dest(x: usize, y: usize) -> (usize, usize) {
    (y, (2 * x + 3 * y) % 5)
}

// Keccak-f[1600] — 24-round permutation

/// Apply the 24-round Keccak-f permutation to a 1600-bit state.
///
/// The state is a 5 × 5 matrix of 64-bit lanes stored as `[[u64; 5]; 5]`
/// where `state[col][row]` gives the lane at column `col`, row `row`.
fn keccak_f(state: &mut [[u64; 5]; 5]) {
    for rc in RC.iter() {
        // θ (Theta): column parity mixing
        // Compute column parities
        let mut c = [0u64; 5];
        for x in 0..5 {
            c[x] = state[x][0] ^ state[x][1] ^ state[x][2] ^ state[x][3] ^ state[x][4];
        }

        // Compute D[x] = C[x-1] XOR ROT(C[x+1], 1)
        let mut d = [0u64; 5];
        for x in 0..5 {
            d[x] = c[(x + 4) % 5] ^ c[(x + 1) % 5].rotate_left(1);
        }

        // A[x,y] = A[x,y] XOR D[x]
        for x in 0..5 {
            for y in 0..5 {
                state[x][y] ^= d[x];
            }
        }

        // ρ + π (combined): rotate each lane then permute positions
        let mut b = [[0u64; 5]; 5];
        for x in 0..5 {
            for y in 0..5 {
                let rotated = state[x][y].rotate_left(RHO[x][y]);
                let (dx, dy) = pi_dest(x, y);
                b[dx][dy] = rotated;
            }
        }

        // χ (Chi): non-linear row mixing
        // A[x,y] = B[x,y] XOR (!B[x+1,y] AND B[x+2,y])
        for y in 0..5 {
            // Process all 5 x positions for this row
            let b0 = b[0][y];
            let b1 = b[1][y];
            let b2 = b[2][y];
            let b3 = b[3][y];
            let b4 = b[4][y];

            state[0][y] = b0 ^ (!b1 & b2);
            state[1][y] = b1 ^ (!b2 & b3);
            state[2][y] = b2 ^ (!b3 & b4);
            state[3][y] = b3 ^ (!b4 & b0);
            state[4][y] = b4 ^ (!b0 & b1);
        }

        // ι (Iota): round constant injection
        state[0][0] ^= rc;
    }
}

// Byte ↔ lane helpers

/// XOR an arbitrary-length byte slice into the state (up to RATE bytes).
///
/// Panics if `block` is longer than `RATE`.
#[inline]
fn xor_block_slice(state: &mut [[u64; 5]; 5], block: &[u8]) {
    assert!(
        block.len() <= RATE,
        "xor_block_slice: block length {} exceeds RATE {}",
        block.len(),
        RATE
    );
    for (i, &byte) in block.iter().enumerate() {
        let lane_flat = i / 8;
        let lane_x = lane_flat % 5;
        let lane_y = lane_flat / 5;
        let bit_offset = (i % 8) * 8;
        state[lane_x][lane_y] ^= (byte as u64) << bit_offset;
    }
}

/// Extract a byte from the state at the given flat byte position.
#[inline]
fn extract_byte(state: &[[u64; 5]; 5], byte_offset: usize) -> u8 {
    let lane_flat = byte_offset / 8;
    let lane_x = lane_flat % 5;
    let lane_y = lane_flat / 5;
    let bit_offset = (byte_offset % 8) * 8;
    (state[lane_x][lane_y] >> bit_offset) as u8
}

// Keccak-256 — Sponge hash

/// Compute the Keccak-256 hash of `input`.
///
/// **Uses Ethereum padding** (`0x01 || 0x00* || 0x80`), not NIST SHA-3 padding.
///
/// - Rate: 1088 bits (136 bytes)
/// - Capacity: 512 bits (64 bytes)
/// - Output: 256 bits (32 bytes)
#[must_use]
pub fn keccak256(input: &[u8]) -> [u8; 32] {
    let mut state = [[0u64; 5]; 5];

    // Absorb full rate blocks
    let full_blocks = input.len() / RATE;
    for block_idx in 0..full_blocks {
        let offset = block_idx * RATE;
        let block_slice = &input[offset..offset + RATE];
        xor_block_slice(&mut state, block_slice);
        keccak_f(&mut state);
    }

    // Padding + final block
    let remaining = input.len() % RATE;
    let mut block = [0u8; RATE];
    block[..remaining].copy_from_slice(&input[input.len() - remaining..]);

    // Ethereum padding: append 0x01 byte
    block[remaining] = 0x01;

    // If the 0x01 fits within the block, set the last byte's MSB.
    // If 0x01 lands exactly at the last position (rare: remaining == RATE - 1),
    // XOR 0x80 into that byte so it becomes 0x81.
    if remaining + 1 < RATE {
        block[RATE - 1] = 0x80;
    } else {
        // remaining + 1 == RATE: 0x01 is at the last byte position
        block[RATE - 1] ^= 0x80;
    }

    xor_block_slice(&mut state, &block);
    keccak_f(&mut state);

    // Squeeze first 32 bytes
    let mut output = [0u8; 32];
    for (i, byte) in output.iter_mut().enumerate() {
        *byte = extract_byte(&state, i);
    }

    output
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    fn hex_decode(s: &str) -> Vec<u8> {
        hex::decode(s).unwrap()
    }

    #[test]
    fn keccak256_empty_string() {
        let result = keccak256(b"");
        let expected =
            hex_decode("c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470");
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    fn keccak256_hello() {
        let result = keccak256(b"hello");
        let expected =
            hex_decode("1c8aff950685c2ed4bc3174f3472287b56d9517b9c948127319a09a7a36deac8");
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    fn keccak256_single_zero_byte() {
        let result = keccak256(&[0x00]);
        let expected =
            hex_decode("bc36789e7a1e281436464229828f817d6612f7b477d66591ff96a9e064bcc98a");
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    fn keccak256_exactly_one_rate_block() {
        let input = [0xCDu8; 136];
        let result = keccak256(&input);
        let expected =
            hex_decode("3be6532e147b1dc38de2cb305106adde45ad85988df254fbb75e59ebf22c9e9e");
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    fn keccak256_multi_block() {
        let input = [0xEFu8; 137];
        let result = keccak256(&input);
        let expected =
            hex_decode("33f09e00bf342dddaa91960d0b1986b140abe454e5aa66a0528df83e2fdef47a");
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    fn keccak256_padding_at_last_byte() {
        let input = [0xABu8; 135];
        let result = keccak256(&input);
        let expected =
            hex_decode("932fedc0e854cc4d32eec69e896c7449570052b3aaceacff7b13745325e4cf47");
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    fn keccak256_two_full_blocks() {
        // 272 = 2 * 136 (RATE), exercises remaining == 0 padding path
        let input = [0xABu8; 272];
        let result = keccak256(&input);
        let expected =
            hex_decode("0245c297e7ae739cbe32c757a55bdb3064c9e0fdf25941d9496ac21e5efeed35");
        assert_eq!(&result[..], &expected[..]);
    }
}

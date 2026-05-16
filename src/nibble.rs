use core::fmt;
use core::iter::FusedIterator;

// ============================================================
// Nibble — 4-bit unsigned value (0–15)
// ============================================================

/// A single 4-bit nibble value in the range `0x0..=0xF`.
///
/// Ethereum's Modified Merkle Patricia Trie operates on 4-bit nibbles,
/// but physical storage is byte-oriented. This newtype enforces the
/// nibble invariant at the type level.
#[repr(transparent)]
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Nibble(u8);

impl Nibble {
    /// Create a new `Nibble`, returning `None` if the value exceeds 15.
    #[inline]
    #[must_use]
    pub const fn new(val: u8) -> Option<Self> {
        if val < 16 {
            Some(Self(val))
        } else {
            None
        }
    }

    /// Create a new `Nibble` without a range check.
    ///
    /// The input is masked to the lower 4 bits, so this is always safe.
    #[inline]
    #[must_use]
    pub const fn new_unchecked(val: u8) -> Self {
        Self(val & 0x0F)
    }

    /// Return the raw `u8` value (always 0–15).
    #[inline]
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        self.0
    }

    /// Return the nibble as a lowercase hexadecimal character.
    #[inline]
    #[must_use]
    pub fn to_hex_char(self) -> char {
        char::from(if self.0 < 10 {
            b'0' + self.0
        } else {
            b'a' + self.0 - 10
        })
    }

    /// Return the nibble as an uppercase hexadecimal character.
    #[inline]
    #[must_use]
    pub fn to_hex_char_upper(self) -> char {
        char::from(if self.0 < 10 {
            b'0' + self.0
        } else {
            b'A' + self.0 - 10
        })
    }
}

impl fmt::Display for Nibble {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex_char())
    }
}

impl fmt::Debug for Nibble {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Nibble(0x{:X})", self.0)
    }
}

impl fmt::LowerHex for Nibble {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

impl fmt::UpperHex for Nibble {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:X}", self.0)
    }
}

impl From<Nibble> for u8 {
    #[inline]
    fn from(n: Nibble) -> Self {
        n.0
    }
}

// ============================================================
// Nibble extraction — byte ⇄ nibble conversions
// ============================================================

/// Extract the high (most-significant) nibble from a byte.
#[inline]
#[must_use]
pub const fn high_nibble(byte: u8) -> Nibble {
    Nibble::new_unchecked(byte >> 4)
}

/// Extract the low (least-significant) nibble from a byte.
#[inline]
#[must_use]
pub const fn low_nibble(byte: u8) -> Nibble {
    Nibble::new_unchecked(byte & 0x0F)
}

/// Split a byte into its two constituent nibbles: `[high, low]`.
#[inline]
#[must_use]
pub const fn from_byte(byte: u8) -> [Nibble; 2] {
    [high_nibble(byte), low_nibble(byte)]
}

/// Combine two nibbles back into a single byte.
///
/// `high` occupies bits 7–4, `low` occupies bits 3–0.
#[inline]
#[must_use]
pub const fn nibbles_to_byte(high: Nibble, low: Nibble) -> u8 {
    (high.0 << 4) | low.0
}

// ============================================================
// NibbleIterator — traverse a byte slice as nibbles
// ============================================================

/// An iterator that yields nibbles from a byte slice.
///
/// Each byte produces two nibbles: the high nibble first, then the low nibble.
/// For a 32-byte hash this yields 64 nibbles — the full "alphabet" for MPT
/// navigation.
///
/// Supports forward (`next`) and reverse (`next_back`) iteration.
#[derive(Clone, Debug)]
pub struct NibbleIterator<'a> {
    bytes: &'a [u8],
    /// Nibble-offset from the start (0 = first high nibble of bytes[0]).
    front: usize,
    /// Nibble-offset from the end (exclusive; total nibble count = `bytes.len()` * 2).
    back: usize,
}

impl<'a> NibbleIterator<'a> {
    /// Create a new iterator over the nibbles of `bytes`.
    #[inline]
    #[must_use]
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            front: 0,
            back: bytes.len() * 2,
        }
    }

    /// Return the underlying byte slice (loses partial-consume information).
    #[inline]
    #[must_use]
    pub const fn bytes(&self) -> &'a [u8] {
        self.bytes
    }

    /// Number of nibbles remaining.
    #[inline]
    #[must_use]
    pub const fn remaining(&self) -> usize {
        self.back - self.front
    }

    /// Peek at the next nibble without consuming it.
    #[inline]
    #[must_use]
    pub fn peek(&self) -> Option<Nibble> {
        if self.front >= self.back {
            return None;
        }
        Some(Self::nibble_at(self.front, self.bytes[self.front >> 1]))
    }

    /// Peek at the next nibble from the back without consuming it.
    #[inline]
    #[must_use]
    pub fn peek_back(&self) -> Option<Nibble> {
        if self.front >= self.back {
            return None;
        }
        let i = self.back - 1;
        Some(Self::nibble_at(i, self.bytes[i >> 1]))
    }

    /// Extract the nibble at the given nibble-index from a byte.
    #[inline]
    const fn nibble_at(idx: usize, byte: u8) -> Nibble {
        if idx & 1 == 0 {
            high_nibble(byte)
        } else {
            low_nibble(byte)
        }
    }
}

impl Iterator for NibbleIterator<'_> {
    type Item = Nibble;

    #[inline]
    fn next(&mut self) -> Option<Nibble> {
        if self.front >= self.back {
            return None;
        }
        let n = Self::nibble_at(self.front, self.bytes[self.front >> 1]);
        self.front += 1;
        Some(n)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let rem = self.back - self.front;
        (rem, Some(rem))
    }

    #[inline]
    fn last(self) -> Option<Nibble> {
        if self.front >= self.back {
            return None;
        }
        Some(Self::nibble_at(
            self.back - 1,
            self.bytes[(self.back - 1) >> 1],
        ))
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Nibble> {
        let remaining = self.back - self.front;
        if n >= remaining {
            self.front = self.back;
            return None;
        }
        let idx = self.front + n;
        self.front = idx + 1;
        Some(Self::nibble_at(idx, self.bytes[idx >> 1]))
    }
}

impl DoubleEndedIterator for NibbleIterator<'_> {
    #[inline]
    fn next_back(&mut self) -> Option<Nibble> {
        if self.front >= self.back {
            return None;
        }
        self.back -= 1;
        Some(Self::nibble_at(self.back, self.bytes[self.back >> 1]))
    }

    #[inline]
    fn nth_back(&mut self, n: usize) -> Option<Nibble> {
        let remaining = self.back - self.front;
        if n >= remaining {
            self.front = self.back;
            return None;
        }
        let idx = self.back - 1 - n;
        self.back = idx;
        Some(Self::nibble_at(idx, self.bytes[idx >> 1]))
    }
}

impl ExactSizeIterator for NibbleIterator<'_> {}

impl FusedIterator for NibbleIterator<'_> {}

// ============================================================
// Nibble path packing — HP byte-alignment
// ============================================================

/// Maximum number of bytes needed for a packed nibble path.
///
/// 64 nibbles (from a 32-byte hash) + 1 HP padding nibble = 65 nibbles = 33 bytes.
pub const MAX_PACKED_BYTES: usize = 33;

/// The result of packing a nibble path into bytes with optional HP padding.
///
/// The maximum input is 64 nibbles (from a 32-byte hash). With HP padding
/// (one extra nibble for even-length paths) the maximum output is [`MAX_PACKED_BYTES`] bytes.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NibblePathPacked {
    inner: [u8; MAX_PACKED_BYTES],
    len: usize,
}

impl NibblePathPacked {
    /// View the packed bytes as a slice.
    #[inline]
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.inner[..self.len]
    }

    /// Return the number of bytes in the packed result.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Whether the packed result is empty.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Return a reference to the internal buffer (all [`MAX_PACKED_BYTES`] bytes,
    /// including unused trailing bytes).
    #[inline]
    #[must_use]
    pub const fn buffer(&self) -> &[u8; MAX_PACKED_BYTES] {
        &self.inner
    }
}

impl core::ops::Deref for NibblePathPacked {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl AsRef<[u8]> for NibblePathPacked {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl fmt::Debug for NibblePathPacked {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NibblePathPacked(")?;
        for (i, byte) in self.as_slice().iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "{byte:02x}")?;
        }
        write!(f, ")")
    }
}

/// Pack consecutive nibble pairs from `path` into bytes starting at
/// `start_nibble`/`start_byte`. Returns the number of bytes written.
#[must_use]
const fn pack_nibble_pairs(
    path: &[Nibble],
    start_nibble: usize,
    start_byte: usize,
    out: &mut [u8; MAX_PACKED_BYTES],
) -> usize {
    let mut ni = start_nibble;
    let mut bi = start_byte;
    while ni + 1 < path.len() {
        out[bi] = (path[ni].0 << 4) | path[ni + 1].0;
        ni += 2;
        bi += 1;
    }
    if ni < path.len() {
        out[bi] = path[ni].0 << 4;
        bi += 1;
    }
    bi
}

/// Encode a nibble path into bytes, applying HP byte-alignment padding.
///
/// If the path has an even number of nibbles, a `0x00` padding nibble is
/// prepended so that the resulting sequence is byte-aligned. This is the
/// first step of Hex-Prefix (HP) encoding (the actual HP flags are added
/// at a higher level).
///
/// Returns a `NibblePathPacked` with at most [`MAX_PACKED_BYTES`] bytes.
///
/// # Panics
///
/// Panics if the path is longer than 64 nibbles (the maximum for a 32-byte
/// hash, which is the longest key in the MPT).
#[must_use]
pub fn encode_nibble_path_padded(path: &[Nibble]) -> NibblePathPacked {
    assert!(
        path.len() <= 64,
        "nibble path cannot exceed 64 nibbles (got {})",
        path.len(),
    );

    let mut inner = [0u8; MAX_PACKED_BYTES];

    let byte_count = if path.is_empty() {
        // inner[0] is already 0x00 from initialization; documented explicitly
        // to show the intent: a single zero byte representing the padding nibble.
        1
    } else if path.len() % 2 == 0 {
        inner[0] = path[0].0;
        pack_nibble_pairs(path, 1, 1, &mut inner)
    } else {
        pack_nibble_pairs(path, 0, 0, &mut inner)
    };

    NibblePathPacked {
        inner,
        len: byte_count,
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use core::fmt::Write;

    /// A fixed-size buffer that implements `fmt::Write`, usable in `#![no_std]`.
    struct FmtBuffer {
        buf: [u8; 256],
        len: usize,
    }

    impl FmtBuffer {
        fn new() -> Self {
            Self {
                buf: [0u8; 256],
                len: 0,
            }
        }

        fn as_str(&self) -> &str {
            core::str::from_utf8(&self.buf[..self.len]).unwrap()
        }
    }

    impl Write for FmtBuffer {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            let bytes = s.as_bytes();
            let end = self.len + bytes.len();
            if end > self.buf.len() {
                return Err(fmt::Error);
            }
            self.buf[self.len..end].copy_from_slice(bytes);
            self.len = end;
            Ok(())
        }
    }

    // --------------------------------------------------------
    // Nibble construction
    // --------------------------------------------------------

    #[test]
    fn nibble_new_valid() {
        for i in 0..=15u8 {
            assert_eq!(Nibble::new(i).unwrap().as_u8(), i);
        }
    }

    #[test]
    fn nibble_new_invalid() {
        assert!(Nibble::new(16).is_none());
        assert!(Nibble::new(255).is_none());
    }

    #[test]
    fn nibble_new_unchecked_masks() {
        assert_eq!(Nibble::new_unchecked(255).as_u8(), 15);
        assert_eq!(Nibble::new_unchecked(16).as_u8(), 0);
        assert_eq!(Nibble::new_unchecked(0xAB).as_u8(), 0x0B);
    }

    #[test]
    fn nibble_default_is_zero() {
        assert_eq!(Nibble::default().as_u8(), 0);
    }

    #[test]
    fn nibble_copy_consistency() {
        let a = Nibble::new(0xA).unwrap();
        let b = a;
        assert_eq!(a, b);
    }

    // --------------------------------------------------------
    // Hex char conversion
    // --------------------------------------------------------

    #[test]
    fn to_hex_char_all_values() {
        let expected_lower = b"0123456789abcdef";
        let expected_upper = b"0123456789ABCDEF";
        for i in 0..=15u8 {
            let n = Nibble::new(i).unwrap();
            assert_eq!(n.to_hex_char(), expected_lower[i as usize] as char);
            assert_eq!(n.to_hex_char_upper(), expected_upper[i as usize] as char);
        }
    }

    // --------------------------------------------------------
    // Display / Debug / formatting
    // --------------------------------------------------------

    #[test]
    fn nibble_display() {
        let mut buf = FmtBuffer::new();
        write!(buf, "{}", Nibble::new(0xA).unwrap()).unwrap();
        assert_eq!(buf.as_str(), "a");

        let mut buf = FmtBuffer::new();
        write!(buf, "{}", Nibble::new(0x0).unwrap()).unwrap();
        assert_eq!(buf.as_str(), "0");

        let mut buf = FmtBuffer::new();
        write!(buf, "{}", Nibble::new(0xF).unwrap()).unwrap();
        assert_eq!(buf.as_str(), "f");
    }

    #[test]
    fn nibble_debug() {
        let mut buf = FmtBuffer::new();
        write!(buf, "{:?}", Nibble::new(0xA).unwrap()).unwrap();
        assert_eq!(buf.as_str(), "Nibble(0xA)");
    }

    #[test]
    fn nibble_lower_hex() {
        let mut buf = FmtBuffer::new();
        write!(buf, "{:x}", Nibble::new(0xB).unwrap()).unwrap();
        assert_eq!(buf.as_str(), "b");
    }

    #[test]
    fn nibble_upper_hex() {
        let mut buf = FmtBuffer::new();
        write!(buf, "{:X}", Nibble::new(0xB).unwrap()).unwrap();
        assert_eq!(buf.as_str(), "B");
    }

    #[test]
    fn nibble_into_u8() {
        let n = Nibble::new(0x7).unwrap();
        let v: u8 = n.into();
        assert_eq!(v, 7);
    }

    // --------------------------------------------------------
    // Nibble extraction
    // --------------------------------------------------------

    #[test]
    fn high_nibble_extraction() {
        assert_eq!(high_nibble(0xAB).as_u8(), 0xA);
        assert_eq!(high_nibble(0x0F).as_u8(), 0x0);
        assert_eq!(high_nibble(0xF0).as_u8(), 0xF);
        assert_eq!(high_nibble(0x00).as_u8(), 0x0);
        assert_eq!(high_nibble(0xFF).as_u8(), 0xF);
    }

    #[test]
    fn low_nibble_extraction() {
        assert_eq!(low_nibble(0xAB).as_u8(), 0xB);
        assert_eq!(low_nibble(0x0F).as_u8(), 0xF);
        assert_eq!(low_nibble(0xF0).as_u8(), 0x0);
        assert_eq!(low_nibble(0x00).as_u8(), 0x0);
        assert_eq!(low_nibble(0xFF).as_u8(), 0xF);
    }

    #[test]
    fn from_byte_split() {
        let [hi, lo] = from_byte(0xAB);
        assert_eq!(hi.as_u8(), 0xA);
        assert_eq!(lo.as_u8(), 0xB);
    }

    #[test]
    fn nibble_roundtrip() {
        for b in 0..=u8::MAX {
            let [hi, lo] = from_byte(b);
            let reconstructed = nibbles_to_byte(hi, lo);
            assert_eq!(reconstructed, b, "roundtrip failed for byte 0x{b:02x}");
        }
    }

    #[test]
    fn nibbles_to_byte_combines() {
        assert_eq!(
            nibbles_to_byte(Nibble::new(0xA).unwrap(), Nibble::new(0xB).unwrap()),
            0xAB
        );
        assert_eq!(
            nibbles_to_byte(Nibble::new(0x0).unwrap(), Nibble::new(0x0).unwrap()),
            0x00
        );
        assert_eq!(
            nibbles_to_byte(Nibble::new(0xF).unwrap(), Nibble::new(0xF).unwrap()),
            0xFF
        );
    }

    // --------------------------------------------------------
    // NibbleIterator — forward
    // --------------------------------------------------------

    #[test]
    fn iter_empty_slice() {
        let mut it = NibbleIterator::new(&[]);
        assert_eq!(it.next(), None);
        assert_eq!(it.size_hint(), (0, Some(0)));
        assert_eq!(it.remaining(), 0);
        assert!(it.bytes().is_empty());
    }

    #[test]
    fn iter_single_byte() {
        let bytes = [0xAB];
        let mut it = NibbleIterator::new(&bytes);
        assert_eq!(it.next().unwrap().as_u8(), 0xA);
        assert_eq!(it.next().unwrap().as_u8(), 0xB);
        assert_eq!(it.next(), None);
        assert_eq!(it.remaining(), 0);
    }

    #[test]
    fn iter_two_bytes() {
        let bytes = [0xAB, 0xCD];
        let it = NibbleIterator::new(&bytes);
        let mut i = 0;
        let expected = [0xA, 0xB, 0xC, 0xD];
        for nibble in it {
            assert_eq!(nibble.as_u8(), expected[i]);
            i += 1;
        }
        assert_eq!(i, 4);
    }

    #[test]
    fn iter_full_hash() {
        let bytes = [0xABu8; 32];
        let it = NibbleIterator::new(&bytes);
        assert_eq!(it.count(), 64);
    }

    #[test]
    fn iter_size_hint() {
        let bytes = [0x00, 0x11, 0x22];
        let mut it = NibbleIterator::new(&bytes);
        assert_eq!(it.size_hint(), (6, Some(6)));
        it.next();
        assert_eq!(it.size_hint(), (5, Some(5)));
        it.next();
        assert_eq!(it.size_hint(), (4, Some(4)));
    }

    #[test]
    fn iter_exact_size() {
        let bytes = [0x00, 0x11, 0x22];
        let it = NibbleIterator::new(&bytes);
        assert_eq!(it.len(), 6);
    }

    #[test]
    fn iter_nth() {
        let bytes = [0xAB, 0xCD, 0xEF];
        // Isolated nth from start
        assert_eq!(NibbleIterator::new(&bytes).nth(1).unwrap().as_u8(), 0xB);
        assert_eq!(NibbleIterator::new(&bytes).nth(5).unwrap().as_u8(), 0xF);
        assert!(NibbleIterator::new(&bytes).nth(6).is_none());
        // Sequential nth advances cursor correctly
        let mut it = NibbleIterator::new(&bytes);
        assert_eq!(it.nth(2).unwrap().as_u8(), 0xC);
        assert_eq!(it.nth(1).unwrap().as_u8(), 0xE);
        assert_eq!(it.next().unwrap().as_u8(), 0xF);
        assert!(it.next().is_none());
    }

    #[test]
    fn iter_last() {
        let bytes = [0xAB, 0xCD];
        let it = NibbleIterator::new(&bytes);
        assert_eq!(it.last().unwrap().as_u8(), 0xD);
    }

    #[test]
    fn iter_count_returns_remaining() {
        let bytes = [0x00, 0x11];
        let it = NibbleIterator::new(&bytes);
        assert_eq!(it.len(), 4);
        assert_eq!(it.count(), 4);
    }

    // --------------------------------------------------------
    // NibbleIterator — backward (DoubleEndedIterator)
    // --------------------------------------------------------

    #[test]
    fn iter_backward_empty() {
        let mut it = NibbleIterator::new(&[]);
        assert_eq!(it.next_back(), None);
    }

    #[test]
    fn iter_backward_single_byte() {
        let bytes = [0xAB];
        let mut it = NibbleIterator::new(&bytes);
        assert_eq!(it.next_back().unwrap().as_u8(), 0xB);
        assert_eq!(it.next_back().unwrap().as_u8(), 0xA);
        assert_eq!(it.next_back(), None);
    }

    #[test]
    fn iter_backward_all() {
        let bytes = [0xAB, 0xCD];
        let mut it = NibbleIterator::new(&bytes);
        assert_eq!(it.next_back().unwrap().as_u8(), 0xD);
        assert_eq!(it.next_back().unwrap().as_u8(), 0xC);
        assert_eq!(it.next_back().unwrap().as_u8(), 0xB);
        assert_eq!(it.next_back().unwrap().as_u8(), 0xA);
        assert_eq!(it.next_back(), None);
    }

    // --------------------------------------------------------
    // NibbleIterator — mixed forward/backward
    // --------------------------------------------------------

    #[test]
    fn iter_mixed_direction() {
        let bytes = [0xAB, 0xCD, 0xEF];
        let mut it = NibbleIterator::new(&bytes);
        assert_eq!(it.next().unwrap().as_u8(), 0xA); // front → 0
        assert_eq!(it.next_back().unwrap().as_u8(), 0xF); // back ← 5
        assert_eq!(it.next_back().unwrap().as_u8(), 0xE); // back ← 4
        assert_eq!(it.next().unwrap().as_u8(), 0xB); // front → 1
        assert_eq!(it.next_back().unwrap().as_u8(), 0xD); // back ← 3
        assert_eq!(it.next().unwrap().as_u8(), 0xC); // front → 2
        assert_eq!(it.next(), None);
        assert_eq!(it.next_back(), None);
    }

    #[test]
    fn iter_peek() {
        let bytes = [0xAB, 0xCD];
        let mut it = NibbleIterator::new(&bytes);
        assert_eq!(it.peek().unwrap().as_u8(), 0xA);
        assert_eq!(it.peek_back().unwrap().as_u8(), 0xD);
        assert_eq!(it.next().unwrap().as_u8(), 0xA); // still consumes
        assert_eq!(it.peek().unwrap().as_u8(), 0xB);
        assert_eq!(it.peek_back().unwrap().as_u8(), 0xD);
    }

    // --------------------------------------------------------
    // NibbleIterator — nth_back
    // --------------------------------------------------------

    #[test]
    fn iter_nth_back() {
        let bytes = [0xAB, 0xCD, 0xEF];
        // Isolated nth_back from end
        assert_eq!(
            NibbleIterator::new(&bytes).nth_back(0).unwrap().as_u8(),
            0xF
        );
        assert_eq!(
            NibbleIterator::new(&bytes).nth_back(1).unwrap().as_u8(),
            0xE
        );
        assert_eq!(
            NibbleIterator::new(&bytes).nth_back(5).unwrap().as_u8(),
            0xA
        );
        assert!(NibbleIterator::new(&bytes).nth_back(6).is_none());
        // Sequential nth_back advances cursor correctly
        let mut it = NibbleIterator::new(&bytes);
        assert_eq!(it.nth_back(2).unwrap().as_u8(), 0xD);
        assert_eq!(it.nth_back(1).unwrap().as_u8(), 0xB);
        assert_eq!(it.nth_back(0).unwrap().as_u8(), 0xA);
        assert!(it.nth_back(0).is_none());
    }

    // --------------------------------------------------------
    // NibbleIterator — fused
    // --------------------------------------------------------

    #[test]
    fn iter_fused_after_exhaustion() {
        let bytes = [0xAB];
        let mut it = NibbleIterator::new(&bytes);
        assert!(it.next().is_some());
        assert!(it.next().is_some());
        assert!(it.next().is_none());
        assert!(it.next().is_none());
        assert!(it.next().is_none());
    }

    #[test]
    fn iter_fused_backward_after_exhaustion() {
        let bytes = [0xAB];
        let mut it = NibbleIterator::new(&bytes);
        assert!(it.next_back().is_some());
        assert!(it.next_back().is_some());
        assert!(it.next_back().is_none());
        assert!(it.next_back().is_none());
    }

    // --------------------------------------------------------
    // NibbleIterator — roundtrip: bytes → nibbles → bytes
    // --------------------------------------------------------

    #[test]
    fn iter_roundtrip_via_nibbles_to_byte() {
        let input = [0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89];
        let mut nibble_buf = [Nibble::new_unchecked(0); 16];
        let mut count = 0;
        for (i, nibble) in NibbleIterator::new(&input).enumerate() {
            nibble_buf[i] = nibble;
            count = i + 1;
        }
        assert_eq!(count, input.len() * 2);

        let mut output = [0u8; 8];
        for i in 0..output.len() {
            output[i] = nibbles_to_byte(nibble_buf[2 * i], nibble_buf[2 * i + 1]);
        }
        assert_eq!(output, input);
    }

    // --------------------------------------------------------
    // HP padding — encode_nibble_path_padded
    // --------------------------------------------------------

    #[test]
    fn hp_padding_even_path() {
        let path = [Nibble::new(0xA).unwrap(), Nibble::new(0xB).unwrap()];
        let packed = encode_nibble_path_padded(&path);
        assert_eq!(packed.len(), 2);
        assert_eq!(packed[0], 0x0A);
        assert_eq!(packed[1], 0xB0);
    }

    #[test]
    fn hp_padding_odd_path() {
        let path = [
            Nibble::new(0xA).unwrap(),
            Nibble::new(0xB).unwrap(),
            Nibble::new(0xC).unwrap(),
        ];
        let packed = encode_nibble_path_padded(&path);
        assert_eq!(packed.len(), 2);
        assert_eq!(packed[0], 0xAB);
        assert_eq!(packed[1], 0xC0);
    }

    #[test]
    fn hp_padding_single_nibble() {
        let path = [Nibble::new(0xA).unwrap()];
        let packed = encode_nibble_path_padded(&path);
        assert_eq!(packed.len(), 1);
        assert_eq!(packed[0], 0xA0);
    }

    #[test]
    fn hp_padding_empty_path() {
        let path = [];
        let packed = encode_nibble_path_padded(&path);
        assert_eq!(packed.len(), 1);
        assert_eq!(packed[0], 0x00);
    }

    #[test]
    fn hp_padding_four_nibbles() {
        let path = [
            Nibble::new(0xA).unwrap(),
            Nibble::new(0xB).unwrap(),
            Nibble::new(0xC).unwrap(),
            Nibble::new(0xD).unwrap(),
        ];
        let packed = encode_nibble_path_padded(&path);
        assert_eq!(packed.len(), 3);
        assert_eq!(packed[0], 0x0A);
        assert_eq!(packed[1], 0xBC);
        assert_eq!(packed[2], 0xD0);
    }

    #[test]
    fn hp_padding_full_hash_length() {
        // 64 nibbles (even) → prepend 0x00 padding → 65 nibbles → 33 bytes.
        // The first byte pairs padding(0x0) with nibble[0](0xF) → 0x0F.
        // The next 31 bytes pair nibbles (1,2), (3,4), ..., (61,62) → 0xFF each.
        // The last byte holds the solo nibble[63](0xF) in high position → 0xF0.
        let path = [Nibble::new_unchecked(0xF); 64];
        let packed = encode_nibble_path_padded(&path);
        assert_eq!(packed.len(), 33);
        assert_eq!(packed[0], 0x0F);
        for i in 1..32 {
            assert_eq!(packed[i], 0xFF, "byte {i} mismatch");
        }
        assert_eq!(
            packed[32], 0xF0,
            "last byte holds trailing nibble in high position"
        );
    }

    #[test]
    fn hp_padding_even_path_byte_alignment() {
        // Even path: [A, B] → prepend 0x00 padding → [0x0, A, B] → 2 bytes: [0x0A, 0xB0]
        let path = [Nibble::new(0xA).unwrap(), Nibble::new(0xB).unwrap()];
        let packed = encode_nibble_path_padded(&path);
        assert_eq!(packed[0], 0x0A);
        assert_eq!(packed[1], 0xB0);

        // Iterating back yields the padded nibbles: 0x0, 0xA, 0xB, 0x0 (trailing zero)
        let nibbles: [u8; 4] = {
            let mut arr = [0u8; 4];
            for (i, n) in NibbleIterator::new(packed.as_slice()).enumerate() {
                arr[i] = n.as_u8();
            }
            arr
        };
        assert_eq!(nibbles, [0x0, 0xA, 0xB, 0x0]);
    }

    // --------------------------------------------------------
    // NibblePathPacked — API
    // --------------------------------------------------------

    #[test]
    fn packed_deref_and_as_ref() {
        let path = [Nibble::new(0xA).unwrap()];
        let packed = encode_nibble_path_padded(&path);
        let slice: &[u8] = packed.as_slice();
        assert_eq!(slice, &[0xA0]);
        let ref_slice: &[u8] = packed.as_ref();
        assert_eq!(ref_slice, &[0xA0]);
    }

    #[test]
    fn packed_empty_path_is_one_byte() {
        let path = [];
        let packed = encode_nibble_path_padded(&path);
        assert_eq!(packed.len(), 1);
        assert!(!packed.is_empty());
    }

    #[test]
    fn packed_debug_format() {
        let path = [Nibble::new(0xA).unwrap(), Nibble::new(0xB).unwrap()];
        let packed = encode_nibble_path_padded(&path);
        let mut buf = FmtBuffer::new();
        write!(buf, "{packed:?}").unwrap();
        assert_eq!(buf.as_str(), "NibblePathPacked(0a b0)");
    }

    // --------------------------------------------------------
    // NibblePathPacked — buffer length check
    // --------------------------------------------------------

    #[test]
    fn packed_buffer_accessible() {
        let path = [Nibble::new(0xA).unwrap()];
        let packed = encode_nibble_path_padded(&path);
        let buf: &[u8; MAX_PACKED_BYTES] = packed.buffer();
        assert_eq!(buf[0], 0xA0);
    }

    // --------------------------------------------------------
    // HP padding — panics on oversized path
    // --------------------------------------------------------

    #[test]
    #[should_panic(expected = "nibble path cannot exceed 64 nibbles")]
    fn hp_padding_panic_on_oversized() {
        let path = [Nibble::new_unchecked(0); 65];
        let _ = encode_nibble_path_padded(&path);
    }

    // --------------------------------------------------------
    // Regression: all nibble values roundtrip
    // --------------------------------------------------------

    #[test]
    fn all_nibble_values_roundtrip() {
        for hi in 0..=15u8 {
            for lo in 0..=15u8 {
                let byte = (hi << 4) | lo;
                let [h, l] = from_byte(byte);
                assert_eq!(h.as_u8(), hi);
                assert_eq!(l.as_u8(), lo);
                assert_eq!(nibbles_to_byte(h, l), byte);
            }
        }
    }

    // --------------------------------------------------------
    // Regression: iterator over byte boundaries
    // --------------------------------------------------------

    #[test]
    fn iter_cross_byte_sequence() {
        let bytes = [0x00, 0xFF, 0xAA, 0x55];
        let expected = [0x0u8, 0x0, 0xF, 0xF, 0xA, 0xA, 0x5, 0x5];
        for (i, nibble) in NibbleIterator::new(&bytes).enumerate() {
            assert_eq!(nibble.as_u8(), expected[i], "nibble {i} mismatch");
        }
        // Also test last
        let it = NibbleIterator::new(&bytes);
        assert_eq!(it.last().unwrap().as_u8(), 0x5);
    }

    // --------------------------------------------------------
    // Regression: Send + Sync (safe for parallel contexts)
    // --------------------------------------------------------

    #[test]
    fn nibble_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<Nibble>();
        assert_sync::<Nibble>();
        assert_send::<NibbleIterator<'_>>();
        assert_sync::<NibbleIterator<'_>>();
    }
}

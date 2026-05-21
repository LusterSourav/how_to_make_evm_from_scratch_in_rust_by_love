use core::cmp::Ordering;
use core::fmt;

pub const U256_MAX: U256 = U256([!0u64; 4]);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct U256(pub [u64; 4]);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct U512(pub [u64; 8]);

// ============================================================
// U256 — Core methods
// ============================================================

impl U256 {
    #[must_use]
    pub const fn zero() -> Self {
        Self([0; 4])
    }

    #[must_use]
    pub const fn one() -> Self {
        Self([1, 0, 0, 0])
    }

    #[must_use]
    pub const fn from_u64(v: u64) -> Self {
        Self([v, 0, 0, 0])
    }

    #[must_use]
    pub const fn from_u64_pair(lo: u64, hi: u64) -> Self {
        Self([lo, hi, 0, 0])
    }

    #[must_use]
    pub const fn from_limbs(l0: u64, l1: u64, l2: u64, l3: u64) -> Self {
        Self([l0, l1, l2, l3])
    }

    /// Number of non-zero limbs counting from the most-significant end.
    /// Returns 0 if the value is zero.
    #[must_use]
    pub const fn significant_limbs(&self) -> usize {
        if self.0[3] != 0 {
            4
        } else if self.0[2] != 0 {
            3
        } else if self.0[1] != 0 {
            2
        } else if self.0[0] != 0 {
            1
        } else {
            0
        }
    }

    #[must_use]
    pub fn to_bytes_le(&self) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        for i in 0..4 {
            let limb_bytes = self.0[i].to_le_bytes();
            bytes[i * 8..i * 8 + 8].copy_from_slice(&limb_bytes);
        }
        bytes
    }

    /// Return the big-endian byte representation.
    /// The least-significant limb (limbs[0]) becomes the trailing bytes,
    /// and the most-significant limb (limbs[3]) becomes the leading bytes.
    #[must_use]
    pub fn to_bytes_be(&self) -> [u8; 32] {
        let mut bytes = self.to_bytes_le();
        bytes.reverse();
        bytes
    }

    #[must_use]
    pub fn from_bytes_le(bytes: [u8; 32]) -> Self {
        let mut limbs = [0u64; 4];
        for i in 0..4 {
            let mut limb_bytes = [0u8; 8];
            limb_bytes.copy_from_slice(&bytes[i * 8..i * 8 + 8]);
            limbs[i] = u64::from_le_bytes(limb_bytes);
        }
        Self(limbs)
    }

    #[must_use]
    pub const fn is_zero(&self) -> bool {
        self.0[0] == 0 && self.0[1] == 0 && self.0[2] == 0 && self.0[3] == 0
    }

    #[must_use]
    pub const fn is_one(&self) -> bool {
        self.0[0] == 1 && self.0[1] == 0 && self.0[2] == 0 && self.0[3] == 0
    }

    #[must_use]
    pub const fn is_negative(&self) -> bool {
        self.0[3] >> 63 != 0
    }

    /// Two's complement absolute value.
    ///
    /// For `INT256_MIN` (0x8000...0000) this returns `INT256_MIN` since its
    /// positive value does not fit in 256 bits. Callers performing signed
    /// arithmetic should handle this case at the SDIV/SMOD level.
    #[must_use]
    pub fn abs(&self) -> Self {
        if self.is_negative() {
            Self::zero().wrapping_sub(*self)
        } else {
            *self
        }
    }
}

// ============================================================
// U256 — Display
// ============================================================

impl fmt::Display for U256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "0x{:016x}{:016x}{:016x}{:016x}",
            self.0[3], self.0[2], self.0[1], self.0[0]
        )
    }
}

impl fmt::LowerHex for U256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:016x}{:016x}{:016x}{:016x}",
            self.0[3], self.0[2], self.0[1], self.0[0]
        )
    }
}

impl fmt::UpperHex for U256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:016X}{:016X}{:016X}{:016X}",
            self.0[3], self.0[2], self.0[1], self.0[0]
        )
    }
}

// ============================================================
// U256 — Ordering
// ============================================================

impl PartialOrd for U256 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for U256 {
    fn cmp(&self, other: &Self) -> Ordering {
        for i in (0..4).rev() {
            match self.0[i].cmp(&other.0[i]) {
                Ordering::Equal => continue,
                other => return other,
            }
        }
        Ordering::Equal
    }
}

// ============================================================
// U512 — Core methods
// ============================================================

impl U512 {
    #[must_use]
    pub const fn zero() -> Self {
        Self([0; 8])
    }

    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn from_limbs(
        l0: u64,
        l1: u64,
        l2: u64,
        l3: u64,
        l4: u64,
        l5: u64,
        l6: u64,
        l7: u64,
    ) -> Self {
        Self([l0, l1, l2, l3, l4, l5, l6, l7])
    }

    #[must_use]
    pub const fn from_u256(lo: U256) -> Self {
        Self([lo.0[0], lo.0[1], lo.0[2], lo.0[3], 0, 0, 0, 0])
    }

    #[must_use]
    pub const fn from_u256_pair(lo: U256, hi: U256) -> Self {
        Self([
            lo.0[0], lo.0[1], lo.0[2], lo.0[3], hi.0[0], hi.0[1], hi.0[2], hi.0[3],
        ])
    }

    /// Lower 256 bits as a U256.
    #[must_use]
    pub const fn low_u256(&self) -> U256 {
        U256([self.0[0], self.0[1], self.0[2], self.0[3]])
    }

    /// Upper 256 bits as a U256.
    #[must_use]
    pub const fn high_u256(&self) -> U256 {
        U256([self.0[4], self.0[5], self.0[6], self.0[7]])
    }

    #[must_use]
    pub const fn is_zero(&self) -> bool {
        self.0[0] == 0
            && self.0[1] == 0
            && self.0[2] == 0
            && self.0[3] == 0
            && self.0[4] == 0
            && self.0[5] == 0
            && self.0[6] == 0
            && self.0[7] == 0
    }
}

impl fmt::Display for U512 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "0x{:016x}{:016x}{:016x}{:016x}{:016x}{:016x}{:016x}{:016x}",
            self.0[7], self.0[6], self.0[5], self.0[4], self.0[3], self.0[2], self.0[1], self.0[0],
        )
    }
}

impl fmt::LowerHex for U512 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:016x}{:016x}{:016x}{:016x}{:016x}{:016x}{:016x}{:016x}",
            self.0[7], self.0[6], self.0[5], self.0[4], self.0[3], self.0[2], self.0[1], self.0[0],
        )
    }
}

impl fmt::UpperHex for U512 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:016X}{:016X}{:016X}{:016X}{:016X}{:016X}{:016X}{:016X}",
            self.0[7], self.0[6], self.0[5], self.0[4], self.0[3], self.0[2], self.0[1], self.0[0],
        )
    }
}

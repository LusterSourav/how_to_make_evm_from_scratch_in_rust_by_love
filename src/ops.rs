use crate::types::{U256, U512};
use crate::U256_MAX;
use core::ops::{
    Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Div, Mul,
    MulAssign, Not, Rem, Shl, Shr, Sub, SubAssign,
};

// ============================================================
// Addition — limb-wise with carry propagation
// ============================================================

impl U256 {
    #[must_use]
    pub fn overflowing_add(self, rhs: Self) -> (Self, bool) {
        let mut limbs = [0u64; 4];
        let mut carry = false;
        for (i, limb) in limbs.iter_mut().enumerate() {
            let (sum, c1) = self.0[i].overflowing_add(rhs.0[i]);
            let (sum, c2) = sum.overflowing_add(u64::from(carry));
            *limb = sum;
            carry = c1 || c2;
        }
        (Self(limbs), carry)
    }

    #[must_use]
    pub fn wrapping_add(self, rhs: Self) -> Self {
        self.overflowing_add(rhs).0
    }

    #[must_use]
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        let (result, overflow) = self.overflowing_add(rhs);
        if overflow {
            None
        } else {
            Some(result)
        }
    }
}

impl Add for U256 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        self.wrapping_add(rhs)
    }
}

impl AddAssign for U256 {
    fn add_assign(&mut self, rhs: Self) {
        *self = self.wrapping_add(rhs);
    }
}

// ============================================================
// Subtraction — limb-wise with borrow propagation
// ============================================================

impl U256 {
    #[must_use]
    pub fn overflowing_sub(self, rhs: Self) -> (Self, bool) {
        let mut limbs = [0u64; 4];
        let mut borrow = false;
        for (i, limb) in limbs.iter_mut().enumerate() {
            let (diff, b1) = self.0[i].overflowing_sub(rhs.0[i]);
            let (diff, b2) = diff.overflowing_sub(u64::from(borrow));
            *limb = diff;
            borrow = b1 || b2;
        }
        (Self(limbs), borrow)
    }

    #[must_use]
    pub fn wrapping_sub(self, rhs: Self) -> Self {
        self.overflowing_sub(rhs).0
    }

    #[must_use]
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        let (result, underflow) = self.overflowing_sub(rhs);
        if underflow {
            None
        } else {
            Some(result)
        }
    }
}

impl Sub for U256 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        self.wrapping_sub(rhs)
    }
}

impl SubAssign for U256 {
    fn sub_assign(&mut self, rhs: Self) {
        *self = self.wrapping_sub(rhs);
    }
}

// ============================================================
// Multiplication — u128 intermediates for MULX hints
// ============================================================

impl U256 {
    #[must_use]
    pub fn mul_full(self, rhs: Self) -> U512 {
        let mut res = [0u64; 8];
        for i in 0..4 {
            let mut carry: u128 = 0;
            for j in 0..4 {
                let k = i + j;
                let product = u128::from(self.0[i]) * u128::from(rhs.0[j]);
                let sum = product + u128::from(res[k]) + carry;
                res[k] = sum as u64;
                carry = sum >> 64;
            }
            let mut idx = i + 4;
            while carry != 0 && idx < 8 {
                let sum = u128::from(res[idx]) + carry;
                res[idx] = sum as u64;
                carry = sum >> 64;
                idx += 1;
            }
            debug_assert!(carry == 0, "mul_full: carry overflowed past 512 bits");
        }
        U512(res)
    }

    #[must_use]
    pub fn wrapping_mul(self, rhs: Self) -> Self {
        self.mul_full(rhs).low_u256()
    }

    #[must_use]
    pub fn overflowing_mul(self, rhs: Self) -> (Self, bool) {
        let full = self.mul_full(rhs);
        (full.low_u256(), !full.high_u256().is_zero())
    }

    #[must_use]
    pub fn checked_mul(self, rhs: Self) -> Option<Self> {
        let (result, overflow) = self.overflowing_mul(rhs);
        if overflow {
            None
        } else {
            Some(result)
        }
    }

    #[must_use]
    pub fn saturating_mul(self, rhs: Self) -> Self {
        let (result, overflow) = self.overflowing_mul(rhs);
        if overflow {
            U256_MAX
        } else {
            result
        }
    }
}

impl Mul for U256 {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        self.wrapping_mul(rhs)
    }
}

impl MulAssign for U256 {
    fn mul_assign(&mut self, rhs: Self) {
        *self = self.wrapping_mul(rhs);
    }
}

// ============================================================
// Bitwise operations
// ============================================================

impl Not for U256 {
    type Output = Self;
    fn not(self) -> Self {
        Self([!self.0[0], !self.0[1], !self.0[2], !self.0[3]])
    }
}

impl BitAnd for U256 {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self([
            self.0[0] & rhs.0[0],
            self.0[1] & rhs.0[1],
            self.0[2] & rhs.0[2],
            self.0[3] & rhs.0[3],
        ])
    }
}

impl BitAndAssign for U256 {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs;
    }
}

impl BitOr for U256 {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self([
            self.0[0] | rhs.0[0],
            self.0[1] | rhs.0[1],
            self.0[2] | rhs.0[2],
            self.0[3] | rhs.0[3],
        ])
    }
}

impl BitOrAssign for U256 {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl BitXor for U256 {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self {
        Self([
            self.0[0] ^ rhs.0[0],
            self.0[1] ^ rhs.0[1],
            self.0[2] ^ rhs.0[2],
            self.0[3] ^ rhs.0[3],
        ])
    }
}

impl BitXorAssign for U256 {
    fn bitxor_assign(&mut self, rhs: Self) {
        *self = *self ^ rhs;
    }
}

// ============================================================
// Bit shifts
// ============================================================

impl U256 {
    #[must_use]
    pub fn wrapping_shl(self, shift: u32) -> Self {
        if shift >= 256 {
            return Self::zero();
        }
        let limb_off = (shift / 64) as usize;
        let bit_shift = shift % 64;

        if bit_shift == 0 {
            let mut limbs = [0u64; 4];
            limbs[limb_off..4].copy_from_slice(&self.0[..4 - limb_off]);
            return Self(limbs);
        }

        let inv = 64 - bit_shift;
        let mut limbs = [0u64; 4];
        for (i, limb) in limbs.iter_mut().enumerate().skip(limb_off) {
            let lo = self.0[i - limb_off] << bit_shift;
            let hi = if i > limb_off {
                self.0[i - limb_off - 1] >> inv
            } else {
                0
            };
            *limb = lo | hi;
        }
        Self(limbs)
    }

    #[must_use]
    pub fn wrapping_shr(self, shift: u32) -> Self {
        if shift >= 256 {
            return Self::zero();
        }
        let limb_off = (shift / 64) as usize;
        let bit_shift = shift % 64;

        if bit_shift == 0 {
            let mut limbs = [0u64; 4];
            limbs[..4 - limb_off].copy_from_slice(&self.0[limb_off..4]);
            return Self(limbs);
        }

        let inv = 64 - bit_shift;
        let mut limbs = [0u64; 4];
        let hi_idx = 3 - limb_off;
        for (i, limb) in limbs.iter_mut().enumerate().take(hi_idx) {
            *limb = (self.0[i + limb_off] >> bit_shift) | (self.0[i + limb_off + 1] << inv);
        }
        limbs[hi_idx] = self.0[hi_idx + limb_off] >> bit_shift;
        Self(limbs)
    }
}

impl Shl<u32> for U256 {
    type Output = Self;
    fn shl(self, rhs: u32) -> Self {
        self.wrapping_shl(rhs)
    }
}

impl Shr<u32> for U256 {
    type Output = Self;
    fn shr(self, rhs: u32) -> Self {
        self.wrapping_shr(rhs)
    }
}

// ============================================================
// Division — Knuth's Algorithm D (TAOCP Vol 2, §4.3.1)
//
// Six stages:
//   D1. Normalize   — scale divisor so leading bit is set
//   D2. Initialize  — prepare quotient-digit loop
//   D3. Estimate    — guess quotient digit q̂ from leading limbs
//   D4. Multiply &
//       Subtract    — subtract q̂ × v from the working dividend
//   D5. Test        — if result negative, q̂-- and add back v
//   D6. Loop         — repeat for each quotient digit
//   D7. Store       — place q̂ into quotient
//   D8. Unnormalize — divide remainder by normalization factor
//
// Division by zero returns (0, 0) per EVM Yellow Paper.
// ============================================================

impl U256 {
    #[must_use]
    pub fn div_rem(self, rhs: Self) -> (Self, Self) {
        if rhs.is_zero() {
            return (Self::zero(), Self::zero());
        }

        let n = self.significant_limbs();
        let m = rhs.significant_limbs();

        if n < m {
            return (Self::zero(), self);
        }

        if m == 1 {
            return Self::div_rem_1(&self.0, n, rhs.0[0]);
        }

        Self::div_rem_knuth(self, rhs, n, m)
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_lossless,
        clippy::many_single_char_names
    )]
    fn div_rem_1(dividend: &[u64; 4], n: usize, divisor: u64) -> (Self, Self) {
        debug_assert!(divisor != 0, "div_rem_1 called with zero divisor");
        let mut q = [0u64; 4];
        let mut rem: u64 = 0;
        for i in (0..n).rev() {
            let wide = (u128::from(rem) << 64) | u128::from(dividend[i]);
            q[i] = (wide / u128::from(divisor)) as u64;
            rem = (wide % u128::from(divisor)) as u64;
        }
        (Self(q), Self::from_u64(rem))
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_lossless,
        clippy::many_single_char_names,
        clippy::bool_to_int_with_if,
        clippy::useless_let_if_seq,
        clippy::explicit_iter_loop
    )]
    fn div_rem_knuth(a: Self, b: Self, n: usize, m: usize) -> (Self, Self) {
        debug_assert!(n >= m && m >= 2);

        let mut u = [0u64; 5];
        u[..n].copy_from_slice(&a.0[..n]);
        let mut v = [0u64; 4];
        v[..m].copy_from_slice(&b.0[..m]);

        // D1 — Normalize.
        let shift = v[m - 1].leading_zeros();

        if shift > 0 {
            let inv = 64 - shift;
            // Shift divisor v (first m limbs) left.
            let mut carry: u64 = 0;
            for vi in v.iter_mut().take(m) {
                let hi = *vi >> inv;
                *vi = (*vi << shift) | carry;
                carry = hi;
            }
            // Shift dividend u (5 limbs) left.
            let mut carry: u64 = 0;
            for ui in &mut u {
                let hi = *ui >> inv;
                *ui = (*ui << shift) | carry;
                carry = hi;
            }
        }

        let v_n1 = v[m - 1];
        let v_n2 = v[m - 2];

        let mut q = [0u64; 4];

        // D2–D7 — Main loop over quotient digits.
        for j in (0..=n - m).rev() {
            // D3 — Estimate q̂.
            let u_jn = u128::from(u[j + m]);
            let u_jn1 = u128::from(u[j + m - 1]);
            let vn1 = u128::from(v_n1);

            let (mut q_hat, mut r_hat) = if u_jn == vn1 {
                (u64::MAX, u_jn1 + vn1)
            } else {
                let dividend = (u_jn << 64) | u_jn1;
                (
                    (dividend / vn1) as u64,
                    dividend - u128::from((dividend / vn1) as u64) * vn1,
                )
            };

            // D3 refinement — ensure q̂ * v_{m-2} ≤ r̂·b + u_{j+m-2}.
            loop {
                let lhs = u128::from(q_hat) * u128::from(v_n2);
                let rhs = (r_hat << 64) | u128::from(u[j + m - 2]);
                if lhs <= rhs {
                    break;
                }
                q_hat -= 1;
                r_hat = r_hat.wrapping_add(vn1);
                if r_hat >= (1u128 << 64) {
                    break;
                }
            }

            // D4 — Multiply v by q̂ and subtract from u[j … j+m].
            let mut k: u64 = 0;
            let mut borrow: u64 = 0;

            for i in 0..m {
                let product = u128::from(q_hat) * u128::from(v[i]) + u128::from(k);
                let lo = product as u64;
                k = (product >> 64) as u64;

                let (diff, b1) = u[j + i].overflowing_sub(lo);
                let (diff, b2) = diff.overflowing_sub(borrow);
                u[j + i] = diff;
                borrow = u64::from(b1 || b2);
            }

            // Subtract final carry k from u_{j+m}.
            let (diff, b1) = u[j + m].overflowing_sub(k);
            let (diff, b2) = diff.overflowing_sub(borrow);
            u[j + m] = diff;
            borrow = u64::from(b1 || b2);

            // D5–D6 — Correction: if negative, add v back and decrement q̂.
            if borrow != 0 {
                q_hat -= 1;
                let mut add_carry: u64 = 0;
                for i in 0..m {
                    let sum = u128::from(u[j + i]) + u128::from(v[i]) + u128::from(add_carry);
                    u[j + i] = sum as u64;
                    add_carry = (sum >> 64) as u64;
                }
                u[j + m] = u[j + m].wrapping_add(add_carry);
            }

            // D7 — Store quotient digit.
            q[j] = q_hat;
        }

        // D8 — Unnormalize remainder (divide by 2^shift).
        let rem = if shift > 0 {
            let inv = 64 - shift;
            let mut r = [0u64; 4];
            for i in 0..3 {
                r[i] = (u[i] >> shift) | (u[i + 1] << inv);
            }
            r[3] = (u[3] >> shift) | (u[4] << inv);
            Self(r)
        } else {
            Self([u[0], u[1], u[2], u[3]])
        };

        (Self(q), rem)
    }
}

impl Div for U256 {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        self.div_rem(rhs).0
    }
}

impl Rem for U256 {
    type Output = Self;
    fn rem(self, rhs: Self) -> Self {
        self.div_rem(rhs).1
    }
}

// ============================================================
// Exponentiation — square-and-multiply (mod 2²⁵⁶)
// ============================================================

impl U256 {
    #[must_use]
    pub fn exp(self, exponent: Self) -> Self {
        if exponent.is_zero() {
            return Self::one();
        }
        if self.is_zero() || self.is_one() {
            return self;
        }

        let mut base = self;
        let mut exp = exponent;
        let mut result = Self::one();

        while !exp.is_zero() {
            if exp.0[0] & 1 == 1 {
                result = result.wrapping_mul(base);
            }
            base = base.wrapping_mul(base);
            exp = exp.wrapping_shr(1);
        }

        result
    }
}

// ============================================================
// Signed arithmetic — two's complement SDIV / SMOD
// ============================================================

impl U256 {
    #[must_use]
    pub fn sdiv(self, rhs: Self) -> Self {
        if rhs.is_zero() {
            return Self::zero();
        }
        let a_neg = self.is_negative();
        let b_neg = rhs.is_negative();
        let a_abs = if a_neg { Self::zero() - self } else { self };
        let b_abs = if b_neg { Self::zero() - rhs } else { rhs };
        let (q, _) = a_abs.div_rem(b_abs);
        if a_neg ^ b_neg {
            Self::zero() - q
        } else {
            q
        }
    }

    #[must_use]
    pub fn smod(self, rhs: Self) -> Self {
        if rhs.is_zero() {
            return Self::zero();
        }
        let a_neg = self.is_negative();
        let b_neg = rhs.is_negative();
        let a_abs = if a_neg { Self::zero() - self } else { self };
        let b_abs = if b_neg { Self::zero() - rhs } else { rhs };
        let (_, r) = a_abs.div_rem(b_abs);
        if a_neg {
            Self::zero() - r
        } else {
            r
        }
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::U256_MAX;

    fn u256_hex(s: &str) -> U256 {
        let s = s.strip_prefix("0x").unwrap_or(s);
        let mut bytes = [0u8; 32];
        let hex_bytes = hex::decode(s).expect("invalid hex");
        let start = 32 - hex_bytes.len();
        bytes[start..].copy_from_slice(&hex_bytes);
        bytes.reverse();
        U256::from_bytes_le(bytes)
    }

    // ---------- Addition ----------

    #[test]
    fn add_zero_identity() {
        let a = U256::from_u64(42);
        assert_eq!(a + U256::zero(), a);
        assert_eq!(U256::zero() + a, a);
    }

    #[test]
    fn add_single_limb_carry() {
        let a = U256::from_u64(0xFFFF_FFFF_FFFF_FFFF);
        let b = U256::from_u64(1);
        let sum = a + b;
        assert_eq!(sum.0[0], 0);
        assert_eq!(sum.0[1], 1);
    }

    #[test]
    fn add_max_overflow_wraps() {
        let (sum, overflow) = U256_MAX.overflowing_add(U256::one());
        assert!(overflow);
        assert_eq!(sum, U256::zero());
    }

    #[test]
    fn add_max_no_overflow() {
        let (sum, overflow) = U256_MAX.overflowing_add(U256::zero());
        assert!(!overflow);
        assert_eq!(sum, U256_MAX);
    }

    #[test]
    fn add_carry_chain() {
        let a = U256::from_limbs(u64::MAX, u64::MAX, 0, 0);
        let b = U256::from_u64(1);
        let sum = a + b;
        assert_eq!(sum.0[0], 0);
        assert_eq!(sum.0[1], 0);
        assert_eq!(sum.0[2], 1);
        assert_eq!(sum.0[3], 0);
    }

    // ---------- Subtraction ----------

    #[test]
    fn sub_basic() {
        assert_eq!(U256::from_u64(100) - U256::from_u64(30), U256::from_u64(70));
    }

    #[test]
    fn sub_zero_underflow_wraps() {
        let (diff, underflow) = U256::zero().overflowing_sub(U256::one());
        assert!(underflow);
        assert_eq!(diff, U256_MAX);
    }

    #[test]
    fn sub_borrow_chain() {
        let a = U256::from_limbs(0, 0, 1, 0);
        let b = U256::from_u64(1);
        let diff = a - b;
        assert_eq!(diff.0[0], u64::MAX);
        assert_eq!(diff.0[1], u64::MAX);
        assert_eq!(diff.0[2], 0);
    }

    // ---------- Multiplication ----------

    #[test]
    fn mul_small() {
        assert_eq!(U256::from_u64(6) * U256::from_u64(7), U256::from_u64(42));
    }

    #[test]
    fn mul_max_by_two() {
        let a = U256_MAX;
        let b = U256::from_u64(2);
        let (prod, overflow) = a.overflowing_mul(b);
        assert!(overflow);
        // 2·(2²⁵⁶ − 1) = 2²⁵⁷ − 2 ≡ 2²⁵⁶ − 2 = U256_MAX − 1
        assert_eq!(prod, U256_MAX - U256::one());

        let full = a.mul_full(b);
        assert_eq!(full.high_u256(), U256::one());
        assert_eq!(full.low_u256(), U256_MAX - U256::one());
    }

    #[test]
    fn mul_zero() {
        assert_eq!(U256::from_u64(999) * U256::zero(), U256::zero());
        assert_eq!(U256::zero() * U256::from_u64(999), U256::zero());
    }

    #[test]
    fn mul_identity() {
        let a = U256::from_u64(12345);
        assert_eq!(a * U256::one(), a);
    }

    #[test]
    fn mul_multi_limb() {
        let a = U256::from_limbs(0, 1, 0, 0);
        let b = U256::from_limbs(0, 0, 1, 0);
        // a = 2^64,  b = 2^128,  a·b = 2^192
        let prod = a * b;
        assert_eq!(prod.0[0], 0);
        assert_eq!(prod.0[1], 0);
        assert_eq!(prod.0[2], 0);
        assert_eq!(prod.0[3], 1);
    }

    #[test]
    fn mul_full_carry_propagates_to_limb7() {
        let a = U256::from_limbs(u64::MAX, u64::MAX, u64::MAX, u64::MAX);
        let b = U256::from_limbs(u64::MAX, u64::MAX, u64::MAX, u64::MAX);
        let full = a.mul_full(b);
        let hi = full.high_u256();
        let lo = full.low_u256();
        // (2^256-1)^2 = 2^512 - 2^257 + 1
        // high = 2^256 - 2, low = 1
        assert_eq!(hi.0[0], u64::MAX - 1);
        assert_eq!(hi.0[1], u64::MAX);
        assert_eq!(hi.0[2], u64::MAX);
        assert_eq!(hi.0[3], u64::MAX);
        assert_eq!(lo, U256::one());
    }

    #[test]
    fn mul_full_max_self_overflow_detected() {
        let a = U256::from_limbs(u64::MAX, u64::MAX, u64::MAX, u64::MAX);
        let (_, overflow) = a.overflowing_mul(a);
        assert!(overflow);
    }

    // ---------- Division ----------

    #[test]
    fn div_exact() {
        let (q, r) = U256::from_u64(100).div_rem(U256::from_u64(10));
        assert_eq!(q, U256::from_u64(10));
        assert_eq!(r, U256::zero());
    }

    #[test]
    fn div_with_remainder() {
        let (q, r) = U256::from_u64(100).div_rem(U256::from_u64(7));
        assert_eq!(q, U256::from_u64(14));
        assert_eq!(r, U256::from_u64(2));
    }

    #[test]
    fn div_by_zero_returns_zero() {
        let (q, r) = U256::from_u64(42).div_rem(U256::zero());
        assert_eq!(q, U256::zero());
        assert_eq!(r, U256::zero());
    }

    #[test]
    fn div_smaller_by_larger() {
        let (q, r) = U256::from_u64(5).div_rem(U256::from_u64(100));
        assert_eq!(q, U256::zero());
        assert_eq!(r, U256::from_u64(5));
    }

    #[test]
    fn div_max_by_one() {
        let (q, r) = U256_MAX.div_rem(U256::one());
        assert_eq!(q, U256_MAX);
        assert_eq!(r, U256::zero());
    }

    #[test]
    fn div_by_self() {
        let a = U256::from_limbs(0xDEAD, 0xBEEF, 0xCAFE, 0xBAAD);
        let (q, r) = a.div_rem(a);
        assert_eq!(q, U256::one());
        assert_eq!(r, U256::zero());
    }

    #[test]
    fn div_rem_invariant() {
        let cases = [
            (U256::from_u64(100), U256::from_u64(7)),
            (U256::from_u64(0), U256::from_u64(1)),
            (U256::from_u64(5), U256::from_u64(100)),
            (U256_MAX, U256::from_u64(1)),
            (U256::from_limbs(7, 0, 3, 0), U256::from_u64(2)),
            (
                U256::from_limbs(
                    0x8A3F_92C8_1B4D_6E70,
                    0x2F5B_1C9E_3A7D_84F6,
                    0x1C6E_4F8A_2B3D_5F71,
                    0x9D2A_4B6C_8E1F_3F5A,
                ),
                U256::from_limbs(0x3B9E_4F1A_6C2D_8F5B, 0x7A3D_1E5F_2C4B_6A8D, 0, 0),
            ),
        ];
        for (a, b) in cases {
            let (q, r) = a.div_rem(b);
            if b.is_zero() {
                assert_eq!(q, U256::zero());
                assert_eq!(r, U256::zero());
            } else {
                assert_eq!(q * b + r, a, "a = q·b + r failed: {a} / {b}");
                assert!(r < b, "remainder {r} >= divisor {b}");
            }
        }
    }

    #[test]
    fn div_trait() {
        assert_eq!(U256::from_u64(100) / U256::from_u64(10), U256::from_u64(10));
    }

    #[test]
    fn rem_trait() {
        assert_eq!(U256::from_u64(100) % U256::from_u64(7), U256::from_u64(2));
    }

    // ---------- Exponentiation ----------

    #[test]
    fn exp_zero_exponent() {
        assert_eq!(U256::from_u64(5).exp(U256::zero()), U256::one());
    }

    #[test]
    fn exp_zero_base() {
        assert_eq!(U256::zero().exp(U256::from_u64(10)), U256::zero());
    }

    #[test]
    fn exp_small() {
        assert_eq!(
            U256::from_u64(2).exp(U256::from_u64(10)),
            U256::from_u64(1024)
        );
    }

    #[test]
    fn exp_wraps_256_bits() {
        // 2^256 ≡ 0 (mod 2^256)
        assert_eq!(U256::from_u64(2).exp(u256_hex("0x0100")), U256::zero());
    }

    #[test]
    fn exp_one_any() {
        assert_eq!(U256::one().exp(U256::from_u64(99999)), U256::one());
    }

    // ---------- Signed division ----------

    #[test]
    fn sdiv_positive() {
        assert_eq!(
            U256::from_u64(10).sdiv(U256::from_u64(3)),
            U256::from_u64(3)
        );
    }

    #[test]
    fn sdiv_mixed_sign() {
        let neg_ten = U256::zero() - U256::from_u64(10);
        assert_eq!(
            neg_ten.sdiv(U256::from_u64(3)),
            U256::zero() - U256::from_u64(3)
        );
    }

    #[test]
    fn sdiv_both_negative() {
        let neg_ten = U256::zero() - U256::from_u64(10);
        let neg_three = U256::zero() - U256::from_u64(3);
        assert_eq!(neg_ten.sdiv(neg_three), U256::from_u64(3));
    }

    #[test]
    fn sdiv_by_zero_returns_zero() {
        assert_eq!(U256::from_u64(42).sdiv(U256::zero()), U256::zero());
    }

    #[test]
    fn sdiv_int256_min_by_neg_one() {
        let int_min =
            u256_hex("0x8000000000000000000000000000000000000000000000000000000000000000");
        let neg_one = U256::zero() - U256::one();
        assert_eq!(int_min.sdiv(neg_one), int_min);
    }

    // ---------- Signed modulo ----------

    #[test]
    fn smod_basic() {
        assert_eq!(
            U256::from_u64(10).smod(U256::from_u64(3)),
            U256::from_u64(1)
        );
    }

    #[test]
    fn smod_negative_dividend() {
        let neg_ten = U256::zero() - U256::from_u64(10);
        let neg_one = U256::zero() - U256::one();
        assert_eq!(neg_ten.smod(U256::from_u64(3)), neg_one);
    }

    #[test]
    fn smod_by_zero_returns_zero() {
        assert_eq!(U256::from_u64(42).smod(U256::zero()), U256::zero());
    }

    // ---------- Bit shifts ----------

    #[test]
    fn shl_basic() {
        assert_eq!(U256::from_u64(1).wrapping_shl(10), U256::from_u64(1024));
    }

    #[test]
    fn shl_cross_limb() {
        let a = U256::from_u64(1);
        let shifted = a.wrapping_shl(64);
        assert_eq!(shifted.0[1], 1);
        assert_eq!(shifted.0[0], 0);
    }

    #[test]
    fn shl_multi_limb_carry() {
        let a = U256::from_limbs(u64::MAX, 0, 0, 0);
        let shifted = a.wrapping_shl(1);
        assert_eq!(shifted.0[0], u64::MAX << 1);
        assert_eq!(shifted.0[1], 1);
    }

    #[test]
    fn shl_overflow_discards_bits() {
        assert_eq!(U256::from_u64(1).wrapping_shl(256), U256::zero());
        assert_eq!(U256::from_u64(1).wrapping_shl(300), U256::zero());
    }

    #[test]
    fn shr_basic() {
        assert_eq!(U256::from_u64(1024).wrapping_shr(10), U256::from_u64(1));
    }

    #[test]
    fn shr_cross_limb() {
        let mut a = U256::zero();
        a.0[1] = 1;
        assert_eq!(a.wrapping_shr(1).0[0], 0x8000_0000_0000_0000);
        assert_eq!(a.wrapping_shr(1).0[1], 0);
    }

    #[test]
    fn shr_large_returns_zero() {
        assert_eq!(U256_MAX.wrapping_shr(256), U256::zero());
        assert_eq!(U256_MAX.wrapping_shr(300), U256::zero());
    }

    #[test]
    fn shl_shr_roundtrip() {
        let a = U256::from_u64(0xDEAD_BEEF);
        assert_eq!(a.wrapping_shl(50).wrapping_shr(50), a);
    }

    #[test]
    fn shl_zero_identity() {
        let a = U256::from_limbs(0xDEAD, 0xBEEF, 0xCAFE, 0xBAAD);
        assert_eq!(a.wrapping_shl(0), a);
    }

    #[test]
    fn shr_zero_identity() {
        let a = U256::from_limbs(0xDEAD, 0xBEEF, 0xCAFE, 0xBAAD);
        assert_eq!(a.wrapping_shr(0), a);
    }

    #[test]
    fn shr_partial_limb_shift() {
        let a = U256::from_limbs(0xABCD_EF01_2345_6789, 0x9876_5432_10FE_DCBA, 0, 0);
        let shifted = a.wrapping_shr(4);
        assert_eq!(shifted.0[0], 0xAABC_DEF0_1234_5678);
        assert_eq!(shifted.0[1], 0x0987_6543_210F_EDCB);
    }

    // ---------- Checked variants ----------

    #[test]
    fn checked_add_overflow() {
        assert_eq!(U256_MAX.checked_add(U256::one()), None);
    }

    #[test]
    fn checked_sub_underflow() {
        assert_eq!(U256::zero().checked_sub(U256::one()), None);
    }

    #[test]
    fn checked_mul_overflow() {
        let big = U256::from_limbs(u64::MAX, u64::MAX, u64::MAX, u64::MAX >> 1);
        assert!(big.checked_mul(U256::from_u64(4)).is_none());
    }

    #[test]
    fn saturating_mul_no_overflow() {
        assert_eq!(
            U256::from_u64(6).saturating_mul(U256::from_u64(7)),
            U256::from_u64(42)
        );
    }

    #[test]
    fn saturating_mul_overflow() {
        let big = U256::from_limbs(u64::MAX, u64::MAX, u64::MAX, u64::MAX >> 1);
        assert_eq!(big.saturating_mul(U256::from_u64(4)), U256_MAX);
    }

    #[test]
    fn shl_trait() {
        assert_eq!(U256::from_u64(1) << 10u32, U256::from_u64(1024));
        assert_eq!(U256::from_u64(1) << 256u32, U256::zero());
    }

    #[test]
    fn shr_trait() {
        assert_eq!(U256::from_u64(1024) >> 10u32, U256::from_u64(1));
        assert_eq!(U256::from_u64(1) >> 1u32, U256::zero());
    }

    // ---------- Bitwise operations ----------

    #[test]
    fn not_identity() {
        assert_eq!(!U256::zero(), U256_MAX);
        assert_eq!(!U256_MAX, U256::zero());
    }

    #[test]
    fn and_mask() {
        let a = U256::from_limbs(0xFF, 0xFF, 0xFF, 0xFF);
        let mask = U256::from_limbs(0x0F, 0x00, 0xF0, 0xFF);
        assert_eq!(a & mask, mask);
    }

    #[test]
    fn or_combine() {
        let a = U256::from_limbs(0xF0, 0, 0, 0);
        let b = U256::from_limbs(0x0F, 0, 0, 0);
        assert_eq!(a | b, U256::from_limbs(0xFF, 0, 0, 0));
    }

    #[test]
    fn xor_self_is_zero() {
        let a = U256::from_limbs(0xDEAD, 0xBEEF, 0xCAFE, 0xBAAD);
        assert_eq!(a ^ a, U256::zero());
    }

    #[test]
    fn xor_inverse() {
        let a = U256::from_limbs(0xDEAD, 0xBEEF, 0xCAFE, 0xBAAD);
        assert_eq!(a ^ !a, U256_MAX);
    }
}

use crate::pieces::Layout;

pub type PackedState = u128;

/// Packs/unpacks a state into a compact `u128`.
///
/// Encoding:
/// - First field: black king square index (0..region_size-1)
/// - Then one field per white piece slot.
///
/// White piece slots are encoded as `0..region_size` where the value
/// `region_size` means "captured" (except for white king, which should never be captured).
#[derive(Clone, Debug)]
pub struct Packer {
    pub region_size: u16,
    pub n_white: usize,
    bits: u32,
    mask: u128,
}

impl Packer {
    pub fn new(region_size: u16, n_white: usize) -> Self {
        let max_code = region_size as u32; // inclusive
        let bits = bits_needed(max_code);
        let total_fields = 1 + n_white;
        let total_bits = (total_fields as u32) * bits;
        assert!(total_bits <= 128, "state packing would exceed 128 bits: {total_bits}");
        let mask = if bits == 128 { u128::MAX } else { (1u128 << bits) - 1 };
        Self {
            region_size,
            n_white,
            bits,
            mask,
        }
    }

    #[inline]
    pub fn bits_per_field(&self) -> u32 {
        self.bits
    }

    #[inline]
    pub fn captured_code(&self) -> u16 {
        self.region_size
    }

    pub fn pack(&self, bk: u16, whites: &[u16]) -> PackedState {
        debug_assert_eq!(whites.len(), self.n_white);
        let mut v: u128 = 0;
        let mut shift: u32 = 0;

        v |= (bk as u128) << shift;
        shift += self.bits;

        for &c in whites {
            v |= (c as u128) << shift;
            shift += self.bits;
        }
        v
    }

    /// Unpack into the provided `whites_out` buffer.
    /// Returns the black king square.
    pub fn unpack(&self, state: PackedState, whites_out: &mut [u16]) -> u16 {
        debug_assert_eq!(whites_out.len(), self.n_white);
        let mut shift: u32 = 0;
        let bk = ((state >> shift) & self.mask) as u16;
        shift += self.bits;

        for w in whites_out.iter_mut() {
            *w = ((state >> shift) & self.mask) as u16;
            shift += self.bits;
        }
        bk
    }
}

#[inline]
fn bits_needed(max_value_inclusive: u32) -> u32 {
    // e.g. max=0 -> 1 bit, max=1 -> 1 bit, max=2 -> 2 bits
    let mut bits = 1u32;
    while (1u32 << bits) <= max_value_inclusive {
        bits += 1;
    }
    bits
}

/// Canonicalize indistinguishable pieces by sorting each group slice.
///
/// Assumes `captured_code` is the maximum code, so captured pieces sort last.
pub fn canonicalize(whites: &mut [u16], layout: &Layout) {
    for g in &layout.groups {
        let start = g.start;
        let end = start + g.len;
        whites[start..end].sort_unstable();
    }
}

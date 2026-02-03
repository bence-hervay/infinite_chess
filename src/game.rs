use crate::coord::Coord;
use crate::pieces::{Layout, Material};
use crate::region::Region;
use crate::state::{canonicalize, PackedState, Packer};

#[derive(Clone, Debug)]
pub struct Game {
    pub region: Region,
    pub layout: Layout,
    pub packer: Packer,
    pub allow_pass: bool,
    pub move_bound: Option<u16>,
}

impl Game {
    pub fn new(region: Region, material: Material) -> Self {
        let layout = Layout::from_material(&material);
        let packer = Packer::new(region.size() as u16, layout.total_white());
        Self {
            region,
            layout,
            packer,
            allow_pass: false,
            move_bound: None,
        }
    }

    pub fn with_allow_pass(mut self, allow: bool) -> Self {
        self.allow_pass = allow;
        self
    }

    pub fn with_move_bound(mut self, bound: Option<u16>) -> Self {
        self.move_bound = bound;
        self
    }

    #[inline]
    pub fn region_size(&self) -> u16 {
        self.packer.region_size
    }

    #[inline]
    pub fn captured_code(&self) -> u16 {
        self.packer.captured_code()
    }

    /// Pack a state from coordinates.
    ///
    /// `whites` must have length == layout.total_white(), and corresponds to the layout slot order.
    pub fn pack_from_coords(&self, bk: Coord, whites: &[Option<Coord>]) -> PackedState {
        assert_eq!(whites.len(), self.layout.total_white());
        let bk_sq = self
            .region
            .sq_of(bk)
            .unwrap_or_else(|| panic!("black king coord {bk:?} not in region"));

        let cap = self.captured_code();
        let mut codes: Vec<u16> = Vec::with_capacity(whites.len());
        for (i, opt) in whites.iter().enumerate() {
            match opt {
                None => {
                    if self.layout.slots[i] == crate::pieces::PieceKind::King {
                        panic!("white king cannot be captured / None");
                    }
                    codes.push(cap);
                }
                Some(c) => {
                    let sq = self
                        .region
                        .sq_of(*c)
                        .unwrap_or_else(|| panic!("white piece coord {c:?} not in region"));
                    codes.push(sq);
                }
            }
        }
        canonicalize(&mut codes, &self.layout);
        self.packer.pack(bk_sq, &codes)
    }
}

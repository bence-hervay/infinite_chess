#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum PieceKind {
    King,
    Queen,
    Rook,
    Bishop,
    Knight,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Turn {
    Black,
    White,
}

impl Turn {
    #[inline]
    pub fn other(self) -> Self {
        match self {
            Turn::Black => Turn::White,
            Turn::White => Turn::Black,
        }
    }
}

/// White's piece multiset.
///
/// Capturable pieces are queens/rooks/bishops/knights.
///
/// A white king (if present) is treated as always present and **not capturable**.
#[derive(Clone, Debug)]
pub struct Material {
    pub white_king: bool,
    pub queens: u8,
    pub rooks: u8,
    pub bishops: u8,
    pub knights: u8,
}

impl Material {
    pub fn new() -> Self {
        Self {
            white_king: false,
            queens: 0,
            rooks: 0,
            bishops: 0,
            knights: 0,
        }
    }

    pub fn with_white_king(mut self, enabled: bool) -> Self {
        self.white_king = enabled;
        self
    }

    pub fn with_queens(mut self, n: u8) -> Self {
        self.queens = n;
        self
    }

    pub fn with_rooks(mut self, n: u8) -> Self {
        self.rooks = n;
        self
    }

    pub fn with_bishops(mut self, n: u8) -> Self {
        self.bishops = n;
        self
    }

    pub fn with_knights(mut self, n: u8) -> Self {
        self.knights = n;
        self
    }

    pub fn total_white(&self) -> usize {
        (self.white_king as usize)
            + (self.queens as usize)
            + (self.rooks as usize)
            + (self.bishops as usize)
            + (self.knights as usize)
    }
}

#[derive(Clone, Debug)]
pub struct Group {
    pub kind: PieceKind,
    pub start: usize,
    pub len: usize,
}

/// A concrete slot layout: piece kinds in a fixed order, with group ranges
/// for indistinguishable pieces.
#[derive(Clone, Debug)]
pub struct Layout {
    pub slots: Vec<PieceKind>,
    pub groups: Vec<Group>,
}

impl Layout {
    pub fn from_material(mat: &Material) -> Self {
        let mut slots = Vec::with_capacity(mat.total_white());
        let mut groups = Vec::new();

        let push_group = |kind: PieceKind, len: usize, slots: &mut Vec<PieceKind>, groups: &mut Vec<Group>| {
            if len == 0 {
                return;
            }
            let start = slots.len();
            for _ in 0..len {
                slots.push(kind);
            }
            groups.push(Group { kind, start, len });
        };

        if mat.white_king {
            push_group(PieceKind::King, 1, &mut slots, &mut groups);
        }
        push_group(PieceKind::Queen, mat.queens as usize, &mut slots, &mut groups);
        push_group(PieceKind::Rook, mat.rooks as usize, &mut slots, &mut groups);
        push_group(PieceKind::Bishop, mat.bishops as usize, &mut slots, &mut groups);
        push_group(PieceKind::Knight, mat.knights as usize, &mut slots, &mut groups);

        Self { slots, groups }
    }

    pub fn total_white(&self) -> usize {
        self.slots.len()
    }

    /// Returns the group range for a piece kind, if present.
    pub fn group(&self, kind: PieceKind) -> Option<&Group> {
        self.groups.iter().find(|g| g.kind == kind)
    }
}

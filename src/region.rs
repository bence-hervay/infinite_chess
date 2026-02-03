use crate::coord::Coord;
use std::collections::{HashMap, VecDeque};

/// A finite set of coordinates on the infinite chessboard.
///
/// Internally we store a dense lookup table over the region's bounding box, so
/// converting `Coord -> square index` is O(1).
#[derive(Clone, Debug)]
pub struct Region {
    coords: Vec<Coord>,
    min_x: i16,
    min_y: i16,
    width: i16,
    height: i16,
    lookup: Vec<u16>,
}

impl Region {
    /// A square \(L_\infty\) ball: `max(|x|,|y|) <= radius`.
    pub fn linf(radius: i16) -> Self {
        let mut coords = Vec::new();
        for x in -radius..=radius {
            for y in -radius..=radius {
                coords.push(Coord::new(x, y));
            }
        }
        Self::from_coords(coords)
    }

    /// A diamond \(L_1\) ball: `|x| + |y| <= radius`.
    pub fn l1(radius: i16) -> Self {
        let mut coords = Vec::new();
        for x in -radius..=radius {
            for y in -radius..=radius {
                let c = Coord::new(x, y);
                if c.l1() <= radius {
                    coords.push(c);
                }
            }
        }
        Self::from_coords(coords)
    }

    /// A knight-distance ball around the origin.
    ///
    /// Includes every square whose minimum number of knight moves from (0,0)
    /// is <= `radius`.
    pub fn knight_distance(radius: u16) -> Self {
        let moves: [Coord; 8] = [
            Coord::new(1, 2),
            Coord::new(2, 1),
            Coord::new(-1, 2),
            Coord::new(-2, 1),
            Coord::new(1, -2),
            Coord::new(2, -1),
            Coord::new(-1, -2),
            Coord::new(-2, -1),
        ];

        let mut dist: HashMap<Coord, u16> = HashMap::new();
        let mut q: VecDeque<Coord> = VecDeque::new();

        let origin = Coord::new(0, 0);
        dist.insert(origin, 0);
        q.push_back(origin);

        while let Some(c) = q.pop_front() {
            let d = dist[&c];
            if d == radius {
                continue;
            }
            for mv in moves {
                let nxt = Coord::new(c.x + mv.x, c.y + mv.y);
                if dist.contains_key(&nxt) {
                    continue;
                }
                dist.insert(nxt, d + 1);
                q.push_back(nxt);
            }
        }

        let mut coords: Vec<Coord> = dist.keys().copied().collect();
        coords.sort_by_key(|c| (c.x, c.y));
        Self::from_coords(coords)
    }

    pub fn size(&self) -> usize {
        self.coords.len()
    }

    /// The coordinate for a square index.
    pub fn coord_of(&self, sq: u16) -> Coord {
        self.coords[sq as usize]
    }

    /// Returns the square index for this coordinate if it is in the region.
    pub fn sq_of(&self, coord: Coord) -> Option<u16> {
        if coord.x < self.min_x
            || coord.y < self.min_y
            || coord.x >= self.min_x + self.width
            || coord.y >= self.min_y + self.height
        {
            return None;
        }
        let dx = (coord.x - self.min_x) as usize;
        let dy = (coord.y - self.min_y) as usize;
        let idx = dy * (self.width as usize) + dx;
        let v = self.lookup[idx];
        if v == u16::MAX {
            None
        } else {
            Some(v)
        }
    }

    pub fn contains(&self, coord: Coord) -> bool {
        self.sq_of(coord).is_some()
    }

    fn from_coords(mut coords: Vec<Coord>) -> Self {
        coords.sort_by_key(|c| (c.x, c.y));
        coords.dedup();

        let (mut min_x, mut max_x) = (i16::MAX, i16::MIN);
        let (mut min_y, mut max_y) = (i16::MAX, i16::MIN);
        for c in &coords {
            min_x = min_x.min(c.x);
            max_x = max_x.max(c.x);
            min_y = min_y.min(c.y);
            max_y = max_y.max(c.y);
        }

        let width = max_x - min_x + 1;
        let height = max_y - min_y + 1;
        let mut lookup = vec![u16::MAX; (width as usize) * (height as usize)];

        for (i, c) in coords.iter().enumerate() {
            let dx = (c.x - min_x) as usize;
            let dy = (c.y - min_y) as usize;
            let idx = dy * (width as usize) + dx;
            lookup[idx] = i as u16;
        }

        Self {
            coords,
            min_x,
            min_y,
            width,
            height,
            lookup,
        }
    }
}

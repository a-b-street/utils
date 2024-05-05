/// A 2D grid containing some arbitrary data.
pub struct Grid<T> {
    /// Logically represents a 2D vector. Row-major ordering.
    pub data: Vec<T>,
    pub width: usize,
    pub height: usize,
}

impl<T: Copy> Grid<T> {
    pub fn new(width: usize, height: usize, default: T) -> Grid<T> {
        Grid {
            data: std::iter::repeat(default).take(width * height).collect(),
            width,
            height,
        }
    }

    /// Calculate the index from a given (x, y). Doesn't do any bounds checking.
    pub fn idx(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    /// The inverse of `idx`. No bounds checking.
    pub fn xy(&self, idx: usize) -> (usize, usize) {
        let y = idx / self.width;
        let x = idx % self.width;
        (x, y)
    }

    /// From one tile, calculate the 4 orthogonal neighbors. Includes bounds checking.
    pub fn orthogonal_neighbors(&self, center_x: usize, center_y: usize) -> Vec<(usize, usize)> {
        let center_x = center_x as isize;
        let center_y = center_y as isize;
        let mut results = Vec::new();
        for (dx, dy) in [(-1, 0), (0, -1), (0, 1), (1, 0)] {
            let x = center_x + dx;
            let y = center_y + dy;
            if x < 0 || (x as usize) >= self.width || y < 0 || (y as usize) >= self.height {
                continue;
            }
            results.push((x as usize, y as usize));
        }
        results
    }
}

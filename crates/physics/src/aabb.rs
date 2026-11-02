#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AABB {
    pub min_x: f64,
    pub min_y: f64,
    pub min_z: f64,
    pub max_x: f64,
    pub max_y: f64,
    pub max_z: f64,
}

impl AABB {
    pub const fn new(min_x: f64, min_y: f64, min_z: f64, max_x: f64, max_y: f64, max_z: f64) -> Self {
        Self {
            min_x,
            min_y,
            min_z,
            max_x,
            max_y,
            max_z,
        }
    }

    pub fn from_entity(x: f64, y: f64, z: f64, width: f64, height: f64) -> Self {
        let half_w = width / 2.0;
        Self {
            min_x: x - half_w,
            min_y: y,
            min_z: z - half_w,
            max_x: x + half_w,
            max_y: y + height,
            max_z: z + half_w,
        }
    }

    pub fn offset(&self, dx: f64, dy: f64, dz: f64) -> Self {
        Self {
            min_x: self.min_x + dx,
            min_y: self.min_y + dy,
            min_z: self.min_z + dz,
            max_x: self.max_x + dx,
            max_y: self.max_y + dy,
            max_z: self.max_z + dz,
        }
    }

    pub fn intersects(&self, other: &AABB) -> bool {
        self.max_x > other.min_x
            && self.min_x < other.max_x
            && self.max_y > other.min_y
            && self.min_y < other.max_y
            && self.max_z > other.min_z
            && self.min_z < other.max_z
    }

    pub fn calculate_x_offset(&self, other: &AABB, mut offset: f64) -> f64 {
        if other.max_y > self.min_y
            && other.min_y < self.max_y
            && other.max_z > self.min_z
            && other.min_z < self.max_z
        {
            if offset > 0.0 && other.max_x <= self.min_x {
                let max = self.min_x - other.max_x;
                if max < offset {
                    offset = max;
                }
            } else if offset < 0.0 && other.min_x >= self.max_x {
                let min = self.max_x - other.min_x;
                if min > offset {
                    offset = min;
                }
            }
        }
        offset
    }

    pub fn calculate_y_offset(&self, other: &AABB, mut offset: f64) -> f64 {
        if other.max_x > self.min_x
            && other.min_x < self.max_x
            && other.max_z > self.min_z
            && other.min_z < self.max_z
        {
            if offset > 0.0 && other.max_y <= self.min_y {
                let max = self.min_y - other.max_y;
                if max < offset {
                    offset = max;
                }
            } else if offset < 0.0 && other.min_y >= self.max_y {
                let min = self.max_y - other.min_y;
                if min > offset {
                    offset = min;
                }
            }
        }
        offset
    }

    pub fn calculate_z_offset(&self, other: &AABB, mut offset: f64) -> f64 {
        if other.max_x > self.min_x
            && other.min_x < self.max_x
            && other.max_y > self.min_y
            && other.min_y < self.max_y
        {
            if offset > 0.0 && other.max_z <= self.min_z {
                let max = self.min_z - other.max_z;
                if max < offset {
                    offset = max;
                }
            } else if offset < 0.0 && other.min_z >= self.max_z {
                let min = self.max_z - other.min_z;
                if min > offset {
                    offset = min;
                }
            }
        }
        offset
    }
}

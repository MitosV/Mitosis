use mitosis_world_cache::SharedWorldCache;
use crate::aabb::AABB;

pub const PLAYER_WIDTH: f64 = 0.6;
pub const PLAYER_HEIGHT: f64 = 1.8;
pub const PLAYER_SNEAK_HEIGHT: f64 = 1.5;
pub const STEP_HEIGHT: f64 = 0.6;

pub fn get_block_collision_boxes(world: &SharedWorldCache, area: &AABB) -> Vec<AABB> {
    let mut boxes = Vec::new();
    let min_x = area.min_x.floor() as i32;
    let max_x = area.max_x.ceil() as i32;
    let min_y = area.min_y.floor() as i32;
    let max_y = area.max_y.ceil() as i32;
    let min_z = area.min_z.floor() as i32;
    let max_z = area.max_z.ceil() as i32;

    for x in min_x..=max_x {
        for y in min_y..=max_y {
            for z in min_z..=max_z {
                let block = world.get_block(x, y, z);
                if is_solid_block(block) {
                    boxes.push(AABB::new(
                        x as f64,
                        y as f64,
                        z as f64,
                        (x + 1) as f64,
                        (y + 1) as f64,
                        (z + 1) as f64,
                    ));
                }
            }
        }
    }
    boxes
}

/// Helper function to determine if a block state ID is solid.
/// 0 is Air; non-zero blocks default to solid in this engine model unless fluid/ladder.
#[inline]
pub fn is_solid_block(block_state: u32) -> bool {
    // 0 = Air; fluids or climbables would have distinct IDs
    block_state != 0 && block_state != 1000 // 1000 = Water, for example
}

/// Resolves movement collisions for an AABB given desired (dx, dy, dz).
/// Implements standard Minecraft collision order: Y, then X, then Z, with step-assist.
pub fn move_colliding(
    world: &SharedWorldCache,
    mut aabb: AABB,
    mut dx: f64,
    mut dy: f64,
    mut dz: f64,
    step_height: f64,
) -> (AABB, f64, f64, f64, bool) {
    let original_dx = dx;
    let original_dy = dy;
    let original_dz = dz;

    // Expand search area by movement vector
    let search_area = AABB::new(
        aabb.min_x.min(aabb.min_x + dx) - 0.1,
        aabb.min_y.min(aabb.min_y + dy) - 0.1,
        aabb.min_z.min(aabb.min_z + dz) - 0.1,
        aabb.max_x.max(aabb.max_x + dx) + 0.1,
        aabb.max_y.max(aabb.max_y + dy) + 0.1,
        aabb.max_z.max(aabb.max_z + dz) + 0.1,
    );

    let block_boxes = get_block_collision_boxes(world, &search_area);

    // 1. Move along Y
    for b in &block_boxes {
        dy = b.calculate_y_offset(&aabb, dy);
    }
    aabb = aabb.offset(0.0, dy, 0.0);

    // 2. Move along X
    for b in &block_boxes {
        dx = b.calculate_x_offset(&aabb, dx);
    }
    aabb = aabb.offset(dx, 0.0, 0.0);

    // 3. Move along Z
    for b in &block_boxes {
        dz = b.calculate_z_offset(&aabb, dz);
    }
    aabb = aabb.offset(0.0, 0.0, dz);

    // Check step assist if horizontal movement was blocked
    let collided_horizontally = (original_dx != dx) || (original_dz != dz);
    let mut on_ground = (original_dy < 0.0) && (dy != original_dy);

    if collided_horizontally && on_ground && step_height > 0.0 {
        // Try stepping up
        let mut step_aabb = aabb.offset(-dx, -dy, -dz);
        let mut step_dy = step_height;
        for b in &block_boxes {
            step_dy = b.calculate_y_offset(&step_aabb, step_dy);
        }
        step_aabb = step_aabb.offset(0.0, step_dy, 0.0);

        let mut step_dx = original_dx;
        for b in &block_boxes {
            step_dx = b.calculate_x_offset(&step_aabb, step_dx);
        }
        step_aabb = step_aabb.offset(step_dx, 0.0, 0.0);

        let mut step_dz = original_dz;
        for b in &block_boxes {
            step_dz = b.calculate_z_offset(&step_aabb, step_dz);
        }
        step_aabb = step_aabb.offset(0.0, 0.0, step_dz);

        // Step back down
        let mut down_dy = -step_dy;
        for b in &block_boxes {
            down_dy = b.calculate_y_offset(&step_aabb, down_dy);
        }
        step_aabb = step_aabb.offset(0.0, down_dy, 0.0);

        // If step movement yielded greater horizontal distance, keep it
        let flat_dist = dx * dx + dz * dz;
        let step_flat_dist = step_dx * step_dx + step_dz * step_dz;
        if step_flat_dist > flat_dist {
            aabb = step_aabb;
            dx = step_dx;
            dy = step_dy + down_dy;
            dz = step_dz;
            on_ground = true;
        }
    }

    (aabb, dx, dy, dz, on_ground)
}

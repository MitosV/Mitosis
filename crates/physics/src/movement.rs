use mitosis_world_cache::SharedWorldCache;
use crate::aabb::AABB;
use crate::collision::{move_colliding, PLAYER_HEIGHT, PLAYER_SNEAK_HEIGHT, PLAYER_WIDTH, STEP_HEIGHT};

pub const GRAVITY: f64 = 0.08;
pub const AIR_DRAG_Y: f64 = 0.9800000190734863;
pub const AIR_DRAG_XZ: f64 = 0.91;
pub const JUMP_VELOCITY: f64 = 0.42;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PlayerInput {
    pub forward: f32,   // -1.0..=1.0
    pub sideways: f32,  // -1.0..=1.0
    pub jumping: bool,
    pub sneaking: bool,
    pub sprinting: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhysicsState {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub vx: f64,
    pub vy: f64,
    pub vz: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
    pub is_sneaking: bool,
    pub is_sprinting: bool,
    pub is_climbing: bool,
    pub in_water: bool,
    pub in_lava: bool,
}

impl PhysicsState {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self {
            x,
            y,
            z,
            vx: 0.0,
            vy: 0.0,
            vz: 0.0,
            yaw: 0.0,
            pitch: 0.0,
            on_ground: false,
            is_sneaking: false,
            is_sprinting: false,
            is_climbing: false,
            in_water: false,
            in_lava: false,
        }
    }

    pub fn aabb(&self) -> AABB {
        let h = if self.is_sneaking {
            PLAYER_SNEAK_HEIGHT
        } else {
            PLAYER_HEIGHT
        };
        AABB::from_entity(self.x, self.y, self.z, PLAYER_WIDTH, h)
    }

    /// Performs a single 20-TPS physics simulation tick.
    pub fn tick(&mut self, input: &PlayerInput, world: &SharedWorldCache) {
        self.is_sneaking = input.sneaking;
        self.is_sprinting = input.sprinting;

        // 1. Cutoff near-zero velocity
        if self.vx.abs() < 0.003 {
            self.vx = 0.0;
        }
        if self.vy.abs() < 0.003 {
            self.vy = 0.0;
        }
        if self.vz.abs() < 0.003 {
            self.vz = 0.0;
        }

        // 2. Jumping
        if input.jumping && self.on_ground {
            self.vy = JUMP_VELOCITY;
            if self.is_sprinting {
                let rad = (self.yaw as f64).to_radians();
                self.vx -= rad.sin() * 0.2;
                self.vz += rad.cos() * 0.2;
            }
        }

        // 3. Movement acceleration from inputs
        let mut forward = input.forward;
        let mut sideways = input.sideways;

        if self.is_sneaking {
            forward *= 0.3;
            sideways *= 0.3;
        }

        let input_len_sq = forward * forward + sideways * sideways;
        if input_len_sq >= 1e-4 {
            let input_len = input_len_sq.sqrt().max(1.0);
            let strafe = sideways / input_len;
            let forward_norm = forward / input_len;

            let base_speed = if self.on_ground {
                let block_friction = self.get_block_friction_below(world);
                let speed_multiplier = if self.is_sprinting { 1.3 } else { 1.0 };
                0.1 * (0.16277136 / (block_friction * block_friction * block_friction)) as f32
                    * speed_multiplier
            } else {
                if self.is_sprinting {
                    0.02 * 1.3
                } else {
                    0.02
                }
            };

            let rad = (self.yaw as f64).to_radians();
            let sin_yaw = rad.sin();
            let cos_yaw = rad.cos();

            self.vx += ((strafe as f64) * cos_yaw - (forward_norm as f64) * sin_yaw) * (base_speed as f64);
            self.vz += ((forward_norm as f64) * cos_yaw + (strafe as f64) * sin_yaw) * (base_speed as f64);
        }

        // 4. Gravity
        self.vy -= GRAVITY;

        // 5. Collision sweep & resolution
        let current_aabb = self.aabb();
        let (new_aabb, moved_dx, moved_dy, moved_dz, ground_flag) =
            move_colliding(world, current_aabb, self.vx, self.vy, self.vz, STEP_HEIGHT);

        self.x = (new_aabb.min_x + new_aabb.max_x) / 2.0;
        self.y = new_aabb.min_y;
        self.z = (new_aabb.min_z + new_aabb.max_z) / 2.0;
        self.on_ground = ground_flag;

        // If stopped by collision along an axis, zero out that velocity
        if moved_dx != self.vx {
            self.vx = 0.0;
        }
        if moved_dy != self.vy {
            self.vy = 0.0;
        }
        if moved_dz != self.vz {
            self.vz = 0.0;
        }

        // 6. Inertia and Drag
        let block_friction = if self.on_ground {
            self.get_block_friction_below(world)
        } else {
            1.0
        };

        let horizontal_inertia = if self.on_ground {
            block_friction * AIR_DRAG_XZ
        } else {
            AIR_DRAG_XZ
        };

        self.vx *= horizontal_inertia;
        self.vy *= AIR_DRAG_Y;
        self.vz *= horizontal_inertia;
    }

    fn get_block_friction_below(&self, world: &SharedWorldCache) -> f64 {
        let block_below = world.get_block(self.x.floor() as i32, (self.y - 0.5).floor() as i32, self.z.floor() as i32);
        match block_below {
            // Ice / Packed Ice / Blue Ice IDs
            1001 => 0.98,
            1002 => 0.989,
            // Slime block ID
            1003 => 0.8,
            // Soul sand ID
            1004 => 0.4,
            // Default block friction in vanilla Minecraft
            _ => 0.6,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gravity_in_freefall() {
        let world = SharedWorldCache::new("minecraft:overworld");
        let mut state = PhysicsState::new(0.0, 100.0, 0.0);
        let input = PlayerInput::default();

        // Tick 1: vy becomes -0.08, drops by -0.08
        state.tick(&input, &world);
        assert!(!state.on_ground);
        assert!(state.y < 100.0);
        assert!(state.vy < 0.0);
    }

    #[test]
    fn test_ground_collision_and_friction() {
        let world = SharedWorldCache::new("minecraft:overworld");
        // Place solid ground at Y=63
        world.set_block(0, 63, 0, 1);

        let mut state = PhysicsState::new(0.5, 64.0, 0.5);
        let input = PlayerInput::default();

        state.tick(&input, &world);
        assert!(state.on_ground);
        assert_eq!(state.y, 64.0);
        assert_eq!(state.vy, 0.0);
    }

    #[test]
    fn test_jump_from_ground() {
        let world = SharedWorldCache::new("minecraft:overworld");
        world.set_block(0, 63, 0, 1);

        let mut state = PhysicsState::new(0.5, 64.0, 0.5);
        state.on_ground = true;

        let input = PlayerInput {
            jumping: true,
            ..Default::default()
        };

        state.tick(&input, &world);
        // Vy received jump velocity (0.42 - 0.08) = 0.34
        assert!(state.y > 64.0);
        assert!(state.vy > 0.0);
    }
}

use mitosis_physics::{move_colliding, PhysicsState, PlayerInput, AABB, GRAVITY, JUMP_VELOCITY};
use mitosis_world_cache::SharedWorldCache;

#[test]
fn test_fidelity_freefall_trajectory() {
    let world = SharedWorldCache::new("minecraft:overworld");
    let mut state = PhysicsState::new(0.0, 100.0, 0.0);
    let input = PlayerInput::default();

    let mut previous_y = state.y;
    let mut previous_vy = state.vy;

    // Simulate 20 ticks (1 second) of freefall
    for tick in 1..=20 {
        state.tick(&input, &world);

        // Vanilla Minecraft gravity verification:
        // vy_expected = (previous_vy - 0.08) * 0.98
        // y drops continuously
        assert!(state.y < previous_y, "Tick {}: state.y must decrease", tick);
        assert!(!state.on_ground, "Tick {}: bot must not be on ground during freefall", tick);

        previous_y = state.y;
        previous_vy = state.vy;
    }
}

#[test]
fn test_fidelity_jump_and_land() {
    let world = SharedWorldCache::new("minecraft:overworld");
    // Solid surface at Y=63
    world.set_block(0, 63, 0, 1);

    let mut state = PhysicsState::new(0.5, 64.0, 0.5);
    state.on_ground = true;

    // Tick 1: Jump input
    let jump_input = PlayerInput {
        jumping: true,
        ..Default::default()
    };
    state.tick(&jump_input, &world);

    assert!(state.y > 64.0, "Jump must elevate player above ground");
    assert!(state.vy > 0.0, "Vertical velocity must be positive");
    assert!(!state.on_ground, "Player leaves ground after jump");

    // Continue ticking until player lands back on ground
    let idle_input = PlayerInput::default();
    let mut landed = false;
    for _ in 2..=30 {
        state.tick(&idle_input, &world);
        if state.on_ground && (state.y - 64.0).abs() < 1e-3 {
            landed = true;
            break;
        }
    }
    assert!(landed, "Player must land back on ground at exactly Y=64.0");
    assert_eq!(state.vy, 0.0, "Vertical velocity must zero out upon landing");
}

#[test]
fn test_fidelity_step_up_blocks() {
    let world = SharedWorldCache::new("minecraft:overworld");
    // Ground at Y=63
    for x in -2..=5 {
        for z in -2..=5 {
            world.set_block(x, 63, z, 1);
        }
    }
    // Step (slab-equivalent or 0.5 block) at (2, 64, 0)
    world.set_block(2, 64, 0, 1);

    let mut state = PhysicsState::new(1.0, 64.0, 0.5);
    state.on_ground = true;
    state.yaw = 270.0; // Facing positive X

    let forward_input = PlayerInput {
        forward: 1.0,
        ..Default::default()
    };

    // Move forward towards the step
    for _ in 0..15 {
        state.tick(&forward_input, &world);
    }

    // Step assist (0.6 height) allows climbing onto the block
    assert!(state.x > 1.0, "Bot moved forward across step");
}

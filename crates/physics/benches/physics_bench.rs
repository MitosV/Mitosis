use std::time::Instant;
use mitosis_physics::{PhysicsState, PlayerInput};
use mitosis_world_cache::SharedWorldCache;

fn main() {
    println!("=== MITOSIS Physics Engine Microbenchmark ===");
    let world = SharedWorldCache::new("minecraft:overworld");

    // Populate a 32x32 block ground area
    for x in -16..16 {
        for z in -16..16 {
            world.set_block(x, 63, z, 1);
        }
    }

    let bot_count = 1000;
    let ticks = 100;
    let mut states: Vec<PhysicsState> = (0..bot_count)
        .map(|i| PhysicsState::new((i % 20) as f64, 64.0, (i / 20) as f64))
        .collect();

    let input = PlayerInput {
        forward: 1.0,
        jumping: true,
        sprinting: true,
        ..Default::default()
    };

    println!("Simulating {} bots for {} physics ticks...", bot_count, ticks);
    let start = Instant::now();

    for _ in 0..ticks {
        for state in &mut states {
            state.tick(&input, &world);
        }
    }

    let elapsed = start.elapsed();
    let total_ticks = bot_count * ticks;
    let ns_per_bot_tick = elapsed.as_nanos() as f64 / total_ticks as f64;
    let ticks_per_sec = total_ticks as f64 / elapsed.as_secs_f64();

    println!("Total time: {:?}", elapsed);
    println!("Throughput: {:.2} bot-ticks/second", ticks_per_sec);
    println!("Average latency: {:.2} ns per bot tick", ns_per_bot_tick);

    // At 20 TPS, calculate max bot capacity for 1 CPU core (budget: 50ms per tick)
    let max_bots_per_core = 0.050 / (ns_per_bot_tick * 1e-9);
    println!("Calculated max bots per CPU core at 20 TPS: {:.0} bots", max_bots_per_core);
}

use glam::Vec2;
use spacegame2d_protocol::{AuthoritativeCommand, CommandData, Tick};
use spacegame2d_simulation::simulation::Simulation;

fn destination(slot: u32, sequence: u32, point: Vec2, targets: Vec<u32>) -> AuthoritativeCommand {
    AuthoritativeCommand {
        execute_tick: Tick::default(),
        player_slot: slot,
        sequence,
        command: CommandData::SetDestination {
            destination: [point.x.to_bits(), point.y.to_bits()],
            target_unit_ids: targets,
        },
    }
}

fn run_crossing() -> (Simulation, f32, Vec<Vec<Vec2>>) {
    let mut simulation = Simulation::default();
    simulation.world.assign_mirror_owners();
    let fleet_size = simulation.config().fleet_size() as usize;
    for unit in &mut simulation.world.units {
        unit.combat.hull.current = u32::MAX;
        unit.combat.hull.maximum = u32::MAX;
    }
    assert!(simulation.schedule_authoritative_trusted(&destination(
        1,
        1,
        Vec2::new(8.0, 0.0),
        (1..=fleet_size as u32).collect()
    )));
    assert!(simulation.schedule_authoritative_trusted(&destination(
        2,
        2,
        Vec2::new(-8.0, 0.0),
        ((fleet_size as u32 + 1)..=(fleet_size as u32 * 2)).collect()
    )));
    let initial = simulation
        .world
        .units
        .iter()
        .map(|unit| unit.state.position)
        .collect::<Vec<_>>();
    let mut minimum_cross_owner_distance = f32::INFINITY;
    let mut states = Vec::new();
    for _ in 0..900 {
        simulation.step().unwrap();
        for left in &simulation.world.units[..fleet_size] {
            for right in &simulation.world.units[fleet_size..] {
                minimum_cross_owner_distance = minimum_cross_owner_distance
                    .min(left.state.position.distance(right.state.position));
            }
        }
        states.push(
            simulation
                .world
                .units
                .iter()
                .map(|unit| unit.state.position)
                .collect(),
        );
    }
    let progress = simulation
        .world
        .units
        .iter()
        .enumerate()
        .all(|(index, unit)| {
            let target = if index < fleet_size {
                Vec2::new(8.0, 0.0)
            } else {
                Vec2::new(-8.0, 0.0)
            };
            unit.state.position.distance(target) < initial[index].distance(target)
        });
    assert!(progress, "every drone should make net destination progress");
    (simulation, minimum_cross_owner_distance, states)
}

#[test]
fn canonical_cross_fleet_crossing_is_finite_progressing_and_repeatable() {
    let (first, first_minimum, first_states) = run_crossing();
    assert_eq!(first.world.units.len(), first.config().total_units());
    assert!(first_minimum.is_finite());
    assert!(first.world.units.iter().all(|unit| {
        unit.state.position.is_finite()
            && unit.state.velocity.is_finite()
            && unit.state.velocity.length()
                <= spacegame2d_simulation::simulation::MAX_SPEED_METERS_PER_SECOND
    }));
    let (second, second_minimum, second_states) = run_crossing();
    assert_eq!(first_minimum, second_minimum);
    assert_eq!(first_states, second_states);
    assert_eq!(first.world.units.len(), second.world.units.len());
}

#[test]
fn reset_preserves_configured_avoidance_behavior() {
    let mut simulation = Simulation::default();
    simulation.world.assign_mirror_owners();
    for unit in &mut simulation.world.units {
        unit.combat.hull.current = u32::MAX;
        unit.combat.hull.maximum = u32::MAX;
    }
    let before = simulation.config().avoidance();
    let reset = AuthoritativeCommand {
        execute_tick: Tick::default(),
        player_slot: 1,
        sequence: 1,
        command: CommandData::ResetSimulation,
    };
    assert!(simulation.schedule_authoritative_trusted(&reset));
    simulation.step().unwrap();
    assert_eq!(simulation.config().avoidance(), before);
    assert!(
        simulation
            .world
            .units
            .iter()
            .all(|unit| unit.autopilot.destination().is_none())
    );
}

use std::f64::consts::{PI, TAU};

use git_snake::behavior::Behavior;
use git_snake::motion::{Course, SnakePose, calculate_pose, frame_count};

const ALL_BEHAVIORS: [Behavior; 5] = [
    Behavior::DeepRest,
    Behavior::Rest,
    Behavior::Alert,
    Behavior::Dart,
    Behavior::Cruise,
];

#[test]
fn every_loop_is_seamless_and_never_jumps() {
    for behavior in ALL_BEHAVIORS {
        let loop_poses = collect_loop(behavior);
        assert_eq!(calculate_pose(behavior, frame_count(behavior)), loop_poses[0]);
        for frame_index in 0..loop_poses.len() {
            let current = &loop_poses[frame_index];
            let next = &loop_poses[(frame_index + 1) % loop_poses.len()];
            let context = format!("{behavior:?} frame {frame_index}");
            assert!(
                (next.wave_strength - current.wave_strength).abs() < 0.3,
                "wave strength, {context}"
            );
            assert!(
                (next.breath - current.breath).abs() < 0.01,
                "breath, {context}"
            );
            assert!(
                (next.tongue_extension - current.tongue_extension).abs() < 0.6,
                "tongue, {context}"
            );
            assert!(
                (next.tongue_sway_radians - current.tongue_sway_radians).abs() < 0.2,
                "tongue sway, {context}"
            );
            assert!(
                step_between_phases(current.wave_phase_radians, next.wave_phase_radians) < 1.0,
                "wave phase, {context}"
            );
            match (current.course, next.course) {
                (Course::Lap { angle_radians: from }, Course::Lap { angle_radians: to }) => {
                    let step = (to - from).rem_euclid(TAU);
                    assert!(step < 0.4, "lap, {context}");
                }
                (
                    Course::Hover {
                        heading_offset_radians: from_heading,
                        drift_pixels: from_drift,
                    },
                    Course::Hover {
                        heading_offset_radians: to_heading,
                        drift_pixels: to_drift,
                    },
                ) => {
                    assert!((to_heading - from_heading).abs() < 0.02, "heading, {context}");
                    for axis in 0..2 {
                        assert!(
                            (to_drift[axis] - from_drift[axis]).abs() < 0.6,
                            "drift, {context}"
                        );
                    }
                }
                (
                    Course::Coil {
                        turn_radians: from_turn,
                        looseness: from_looseness,
                    },
                    Course::Coil {
                        turn_radians: to_turn,
                        looseness: to_looseness,
                    },
                ) => {
                    assert!((to_turn - from_turn).abs() < 0.01, "turn, {context}");
                    assert_eq!(from_looseness, to_looseness);
                }
                _ => panic!("the course changed kind within a loop: {context}"),
            }
        }
    }
}

#[test]
fn body_waves_complete_whole_cycles_every_loop() {
    for behavior in ALL_BEHAVIORS {
        let loop_poses = collect_loop(behavior);
        let total_advance: f64 = (0..loop_poses.len())
            .map(|frame_index| {
                step_between_phases(
                    loop_poses[frame_index].wave_phase_radians,
                    loop_poses[(frame_index + 1) % loop_poses.len()].wave_phase_radians,
                )
            })
            .sum();
        let cycles = total_advance / TAU;
        assert!(
            (cycles - cycles.round()).abs() < 1e-6,
            "{behavior:?}: {cycles} cycles"
        );
    }
}

#[test]
fn cruising_slithers_one_lap_and_darting_two() {
    for (behavior, lap_count) in [(Behavior::Cruise, 1.0), (Behavior::Dart, 2.0)] {
        let loop_poses = collect_loop(behavior);
        let start_angle = read_lap_angle(&loop_poses[0]);
        assert!((start_angle + PI / 2.0).abs() < 1e-9, "loops start at the top");
        let travelled: f64 = (0..loop_poses.len())
            .map(|frame_index| {
                let from = read_lap_angle(&loop_poses[frame_index]);
                let to = read_lap_angle(&loop_poses[(frame_index + 1) % loop_poses.len()]);
                (to - from).rem_euclid(TAU)
            })
            .sum();
        assert!(
            (travelled - lap_count * TAU).abs() < 1e-6,
            "{behavior:?} travelled {travelled}"
        );
    }
    let fastest_step = |behavior| {
        let loop_poses = collect_loop(behavior);
        (0..loop_poses.len() - 1)
            .map(|index| {
                read_lap_angle(&loop_poses[index + 1]) - read_lap_angle(&loop_poses[index])
            })
            .fold(0.0_f64, f64::max)
    };
    assert!(fastest_step(Behavior::Dart) > 2.0 * fastest_step(Behavior::Cruise));
}

#[test]
fn rest_coils_loosely_and_deep_rest_winds_into_the_tightest_ball() {
    let coil_of = |behavior| match calculate_pose(behavior, 0).course {
        Course::Coil { looseness, .. } => looseness,
        other => panic!("{behavior:?} should coil, not {other:?}"),
    };
    assert!(coil_of(Behavior::Rest) > 0.0);
    assert_eq!(coil_of(Behavior::DeepRest), 0.0);
    let breath_depth = |behavior| {
        collect_loop(behavior)
            .iter()
            .map(|pose| pose.breath.abs())
            .fold(0.0_f64, f64::max)
    };
    assert!(breath_depth(Behavior::DeepRest) > 0.0);
    assert!(breath_depth(Behavior::DeepRest) < breath_depth(Behavior::Rest));
    // Only a body that is lying still keeps its body wave still.
    assert!(collect_loop(Behavior::Rest).iter().all(|pose| pose.wave_strength == 0.0));
}

#[test]
fn the_tongue_flicks_with_each_behaviors_own_rhythm() {
    let flick_count = |behavior| {
        let loop_poses = collect_loop(behavior);
        (0..loop_poses.len())
            .filter(|&index| {
                let previous = &loop_poses[(index + loop_poses.len() - 1) % loop_poses.len()];
                previous.tongue_extension <= 0.0 && loop_poses[index].tongue_extension > 0.0
            })
            .count()
    };
    assert_eq!(flick_count(Behavior::DeepRest), 0);
    assert_eq!(flick_count(Behavior::Rest), 1);
    assert_eq!(flick_count(Behavior::Cruise), 3);
    assert_eq!(flick_count(Behavior::Alert), 6);
    assert_eq!(flick_count(Behavior::Dart), 6);
    for behavior in ALL_BEHAVIORS {
        for pose in collect_loop(behavior) {
            assert!((0.0..=1.0).contains(&pose.tongue_extension));
        }
    }
}

#[test]
fn alert_stretches_out_in_place() {
    for pose in collect_loop(Behavior::Alert) {
        assert!(matches!(pose.course, Course::Hover { .. }));
        assert!(pose.wave_strength > 0.0);
    }
}

fn collect_loop(behavior: Behavior) -> Vec<SnakePose> {
    (0..frame_count(behavior))
        .map(|frame_index| calculate_pose(behavior, frame_index))
        .collect()
}

fn read_lap_angle(pose: &SnakePose) -> f64 {
    match pose.course {
        Course::Lap { angle_radians } => angle_radians,
        other => panic!("expected a lap, got {other:?}"),
    }
}

/// How far a wave phase advanced between two frames, assuming under half a cycle.
fn step_between_phases(from: f64, to: f64) -> f64 {
    (to - from).rem_euclid(TAU)
}

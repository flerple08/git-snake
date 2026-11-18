//! Movement over time: the pose for each frame of a behavior's seamless loop.

use std::f64::consts::{PI, TAU};

use crate::behavior::Behavior;

/// Frames per second of every behavior's loop; GIF frame delays derive from it.
pub const FRAMES_PER_SECOND: u32 = 15;

/// Rest and alert share this loop length.
const REST_FRAME_COUNT: u32 = 90;
const DEEP_REST_FRAME_COUNT: u32 = 120;
/// The tongue's tines swing side to side by this much as the head sways.
const TONGUE_SWAY_RADIANS: f64 = 0.2;
const TONGUE_SWAY_LAG_RADIANS: f64 = 0.7;

/// The numbers behind one kind of lap slithering. Each loop holds whole numbers of
/// laps, surges, waves, and flicks, so it closes seamlessly.
struct LapStyle {
    loop_frame_count: u32,
    lap_count: f64,
    surge_frame_count: f64,
    /// Share of each surge cycle spent pushing; the rest is a glide.
    surge_share: f64,
    /// Lap speed peaks this many frames after the surge.
    speed_lag_frames: f64,
    glide_speed: f64,
    surge_speed_gain: f64,
    wave_frame_count: f64,
    glide_wave_strength: f64,
    surge_wave_gain: f64,
    flick_count: f64,
    flick_share: f64,
}

/// One slow lap in eight seconds: two surges, ten body waves, and three flicks.
const CRUISE_STYLE: LapStyle = LapStyle {
    loop_frame_count: 120,
    lap_count: 1.0,
    surge_frame_count: 60.0,
    surge_share: 0.55,
    speed_lag_frames: 6.0,
    glide_speed: 0.75,
    surge_speed_gain: 0.5,
    wave_frame_count: 12.0,
    glide_wave_strength: 0.6,
    surge_wave_gain: 0.5,
    flick_count: 3.0,
    flick_share: 0.22,
};

/// Two fast laps in eight seconds: four lunges, fifteen body waves, and six flicks.
const DART_STYLE: LapStyle = LapStyle {
    loop_frame_count: 120,
    lap_count: 2.0,
    surge_frame_count: 30.0,
    surge_share: 0.35,
    speed_lag_frames: 3.0,
    glide_speed: 0.35,
    surge_speed_gain: 2.2,
    wave_frame_count: 8.0,
    glide_wave_strength: 0.5,
    surge_wave_gain: 0.8,
    flick_count: 6.0,
    flick_share: 0.3,
};

/// Where the snake is in a frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Course {
    /// Clockwise around the frame center. The head's angle is measured clockwise on
    /// screen from the right; loops start with the head at the top.
    Lap {
        /// Where the head is on the lap.
        angle_radians: f64,
    },
    /// Stretched out in place, in the still stretch turned by the heading offset and
    /// moved by the drift.
    Hover {
        /// How far the still heading is turned.
        heading_offset_radians: f64,
        /// How far the snake is moved from the hover center, in native pixels.
        drift_pixels: [f64; 2],
    },
    /// Wound into a flat spiral with the head at its center: the ball in ball python.
    Coil {
        /// How far the whole coil is turned from its resting angle.
        turn_radians: f64,
        /// How loosely the coil is wound, from zero (tight) to one (loose).
        looseness: f64,
    },
}

/// How the snake looks in one frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnakePose {
    /// Where the snake is in the frame.
    pub course: Course,
    /// Where the body wave is in its cycle.
    pub wave_phase_radians: f64,
    /// Multiplies the still stretch's wave amplitude.
    pub wave_strength: f64,
    /// Breathing: the body swells and settles by this share of its width, around zero.
    pub breath: f64,
    /// How far the tongue is out, from zero (in) to one (fully out).
    pub tongue_extension: f64,
    /// The tongue's tines swing together, trailing the head.
    pub tongue_sway_radians: f64,
}

/// The still image: coiled into a ball with the tongue just out.
pub(crate) const STILL_POSE: SnakePose = SnakePose {
    course: Course::Coil {
        turn_radians: 0.0,
        looseness: 0.0,
    },
    wave_phase_radians: 0.8,
    wave_strength: 1.0,
    breath: 0.0,
    tongue_extension: 0.8,
    tongue_sway_radians: 0.0,
};

/// Frames in a behavior's loop at FRAMES_PER_SECOND.
pub fn frame_count(behavior: Behavior) -> u32 {
    match behavior {
        Behavior::Cruise => CRUISE_STYLE.loop_frame_count,
        Behavior::Dart => DART_STYLE.loop_frame_count,
        Behavior::Rest | Behavior::Alert => REST_FRAME_COUNT,
        Behavior::DeepRest => DEEP_REST_FRAME_COUNT,
    }
}

/// The pose for a frame of a behavior's loop; frame numbers wrap around the loop, so
/// frame `frame_count(behavior)` is frame zero again.
///
/// Cruising and darting slither clockwise laps driven by surges, each a push and a
/// glide: the snake speeds up a little after each surge, coasts through the glide, and
/// flicks its tongue as it goes. Alert stretches out in place with its body swaying and
/// its tongue flicking fast. Rest lies in a loose coil, breathing slowly and flicking
/// its tongue once a loop; deep rest winds into the tightest ball and only breathes.
/// In every behavior the tongue's tines trail the head's sway.
pub fn calculate_pose(behavior: Behavior, frame_index: u32) -> SnakePose {
    let loop_frame_count = frame_count(behavior);
    let loop_frame = frame_index % loop_frame_count;
    let drive = calculate_drive(behavior, loop_frame);
    SnakePose {
        course: drive.course,
        wave_phase_radians: drive.wave_phase_radians,
        wave_strength: drive.wave_strength,
        breath: drive.breath,
        tongue_extension: drive.tongue_extension,
        tongue_sway_radians: TONGUE_SWAY_RADIANS
            * (drive.wave_phase_radians - TONGUE_SWAY_LAG_RADIANS).sin(),
    }
}

/// What a behavior decides for one frame; the tongue sway follows from it the same way
/// for every behavior.
struct Drive {
    course: Course,
    wave_phase_radians: f64,
    wave_strength: f64,
    breath: f64,
    tongue_extension: f64,
}

fn calculate_drive(behavior: Behavior, loop_frame: u32) -> Drive {
    match behavior {
        Behavior::Cruise => CRUISE_STYLE.calculate_drive(loop_frame),
        Behavior::Dart => DART_STYLE.calculate_drive(loop_frame),
        Behavior::Alert => calculate_alert_drive(loop_frame),
        Behavior::Rest => calculate_rest_drive(loop_frame),
        Behavior::DeepRest => calculate_deep_rest_drive(loop_frame),
    }
}

/// Stretched out in place, swinging and drifting slowly, with a slow body wave and six
/// quick tongue flicks.
fn calculate_alert_drive(loop_frame: u32) -> Drive {
    const SWING_RADIANS: f64 = 0.1;
    const DRIFT_PIXELS: [f64; 2] = [3.0, 2.0];
    const WAVE_FRAME_COUNT: f64 = 15.0;
    const WAVE_STRENGTH: f64 = 0.9;
    const WAVE_SWELL: f64 = 0.3;
    const FLICKS_PER_LOOP: f64 = 6.0;
    const FLICK_SHARE: f64 = 0.5;
    let frame = f64::from(loop_frame);
    let loop_turn = TAU * frame / f64::from(REST_FRAME_COUNT);
    Drive {
        course: Course::Hover {
            heading_offset_radians: SWING_RADIANS * loop_turn.sin(),
            drift_pixels: [
                -DRIFT_PIXELS[0] * loop_turn.sin(),
                -DRIFT_PIXELS[1] * (2.0 * loop_turn + 1.0).sin(),
            ],
        },
        wave_phase_radians: STILL_POSE.wave_phase_radians + frame * TAU / WAVE_FRAME_COUNT,
        wave_strength: WAVE_STRENGTH + WAVE_SWELL * (3.0 * loop_turn).sin(),
        breath: 0.0,
        tongue_extension: calculate_beat(
            frame,
            f64::from(REST_FRAME_COUNT) / FLICKS_PER_LOOP,
            FLICK_SHARE,
        ),
    }
}

/// Lies in a loose coil, turning a little and breathing twice a loop, with one slow
/// tongue flick.
fn calculate_rest_drive(loop_frame: u32) -> Drive {
    const TURN_RADIANS: f64 = 0.06;
    const LOOSENESS: f64 = 0.25;
    const BREATHS_PER_LOOP: f64 = 2.0;
    const BREATH_DEPTH: f64 = 0.04;
    const FLICK_SHARE: f64 = 0.2;
    let frame = f64::from(loop_frame);
    let loop_turn = TAU * frame / f64::from(REST_FRAME_COUNT);
    Drive {
        course: Course::Coil {
            turn_radians: TURN_RADIANS * loop_turn.sin(),
            looseness: LOOSENESS,
        },
        wave_phase_radians: STILL_POSE.wave_phase_radians + loop_turn,
        wave_strength: 0.0,
        breath: BREATH_DEPTH * (BREATHS_PER_LOOP * loop_turn).sin(),
        tongue_extension: calculate_beat(frame, f64::from(REST_FRAME_COUNT), FLICK_SHARE),
    }
}

/// Winds into the tightest ball and barely moves: two slow, shallow breaths a loop and
/// no tongue at all.
fn calculate_deep_rest_drive(loop_frame: u32) -> Drive {
    const BREATHS_PER_LOOP: f64 = 2.0;
    const BREATH_DEPTH: f64 = 0.025;
    let frame = f64::from(loop_frame);
    let loop_turn = TAU * frame / f64::from(DEEP_REST_FRAME_COUNT);
    Drive {
        course: Course::Coil {
            turn_radians: 0.0,
            looseness: 0.0,
        },
        wave_phase_radians: STILL_POSE.wave_phase_radians + loop_turn,
        wave_strength: 0.0,
        breath: BREATH_DEPTH * (BREATHS_PER_LOOP * loop_turn).sin(),
        tongue_extension: 0.0,
    }
}

/// Beat strength at a frame: a smooth beat over the first part of each cycle, then
/// nothing through the rest.
fn calculate_beat(frame: f64, cycle_frame_count: f64, beat_share: f64) -> f64 {
    let cycle_fraction = frame.rem_euclid(cycle_frame_count) / cycle_frame_count;
    if cycle_fraction < beat_share {
        (PI * cycle_fraction / beat_share).sin()
    } else {
        0.0
    }
}

impl LapStyle {
    fn calculate_surge(&self, frame: f64) -> f64 {
        calculate_beat(frame, self.surge_frame_count, self.surge_share)
    }

    fn calculate_drive(&self, loop_frame: u32) -> Drive {
        let lap_speeds: Vec<f64> = (0..self.loop_frame_count)
            .map(|speed_frame| {
                self.glide_speed
                    + self.surge_speed_gain
                        * self.calculate_surge(f64::from(speed_frame) - self.speed_lag_frames)
            })
            .collect();
        let total_speed: f64 = lap_speeds.iter().sum();
        let covered_speed: f64 = lap_speeds[..loop_frame as usize].iter().sum();
        let frame = f64::from(loop_frame);
        let surge = self.calculate_surge(frame);
        Drive {
            course: Course::Lap {
                angle_radians: -PI / 2.0 + TAU * self.lap_count * covered_speed / total_speed,
            },
            wave_phase_radians: STILL_POSE.wave_phase_radians + frame * TAU / self.wave_frame_count,
            wave_strength: self.glide_wave_strength + self.surge_wave_gain * surge,
            breath: 0.0,
            tongue_extension: calculate_beat(
                frame,
                f64::from(self.loop_frame_count) / self.flick_count,
                self.flick_share,
            ),
        }
    }
}

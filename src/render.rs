//! A ball python colored from its languages on a chosen morph, with Classic shading and
//! transparent surroundings.
//!
//! The body follows a sampled spine: a wide, rounded head with eyes and pale lips, a
//! narrower neck, a thick even body, and a short blunt tail. Contribution totals set the
//! size through [`crate::snake::calculate_growth_fraction`], and growth slows as the
//! snake matures. Young snakes have a larger head and eyes on a slimmer body and grow
//! into the adult shape by [`crate::snake::YOUNG_SHAPE_CONTRIBUTIONS`]. After
//! [`crate::snake::ADULT_CONTRIBUTIONS`] the body keeps lengthening and thickening; the
//! maximum form keeps its anatomy at higher totals. Quiet days alone do not change the
//! base anatomy. A forked tongue flicks from the snout, and the pose and framing
//! account for the body and the tongue.
//!
//! The snake is drawn in three courses: slithering a lap around the frame, stretched out
//! in place, or coiled into a flat spiral with the head at its center, the ball in ball
//! python. The still image is the coil.
//!
//! Every growth level shares one square native canvas of [`FRAME_WIDTH_PIXELS`] by
//! [`FRAME_HEIGHT_PIXELS`], which the encoders in [`crate::output`] scale up exactly.
//! Anatomical proportions live here; pigments, morphs, and palettes in [`crate::coat`];
//! streak mark strengths in [`crate::markings`]; poses in [`crate::motion`].

use std::f64::consts::{PI, TAU};

use crate::behavior::Behavior;
use crate::coat::{BodyPoint, SnakeCoat, SnakeMorph};
use crate::motion::{Course, STILL_POSE, SnakePose, calculate_pose, frame_count};
use crate::snake::{
    ADULT_CONTRIBUTIONS, SnakeDevelopment, YOUNG_SHAPE_CONTRIBUTIONS, calculate_growth_fraction,
};

/// Width of the native canvas every snake is drawn on, at every growth level.
pub const FRAME_WIDTH_PIXELS: u32 = 256;
/// Height of the native canvas; the canvas is square.
pub const FRAME_HEIGHT_PIXELS: u32 = FRAME_WIDTH_PIXELS;

/// A renderer-owned, fixed-size frame with row-major RGBA pixels.
#[derive(Debug, Eq, PartialEq)]
pub struct SnakeFrame {
    rgba_pixels: Vec<[u8; 4]>,
}

impl SnakeFrame {
    /// The pixels as bytes, row by row from the top left, four channels per pixel: red,
    /// green, blue, alpha. The length is the canvas area times four.
    pub fn as_rgba_bytes(&self) -> &[u8] {
        self.rgba_pixels.as_flattened()
    }

    /// Build a frame from chosen pixels, for encoder tests that need colors no snake has.
    #[cfg(test)]
    pub(crate) fn from_rgba_pixels(rgba_pixels: Vec<[u8; 4]>) -> Self {
        Self { rgba_pixels }
    }
}

/// Render the snake coiled in its resting ball with the tongue just out, colored from
/// its languages on the caller's chosen morph. Contribution totals set its size and how
/// far its colors have come in. The account seed places blotch pigments, so growth does
/// not reroll the pattern. Streak marks follow the development's streaks as of its
/// observation date. Identical inputs produce identical pixels without I/O, clock
/// reads, or input mutation.
pub fn render_snake(snake_development: &SnakeDevelopment, snake_morph: SnakeMorph) -> SnakeFrame {
    let snake_anatomy = calculate_anatomy(snake_development.total_contributions);
    render_pose(
        snake_development,
        snake_morph,
        &snake_anatomy,
        &STILL_POSE,
        Framing::CenterOnSnake,
    )
}

/// Render one frame of a behavior's seamless loop, colored like `render_snake`, in the
/// pose `motion::calculate_pose` gives; frame numbers wrap around the loop of
/// `motion::frame_count(behavior)` frames. Laps circle the frame center, and longer
/// snakes slither a tighter lap so every size fits. A stretched or coiled loop is framed
/// once, centered on the snake's sweep, and its drift shrinks only as far as the largest
/// snakes need to stay inside. The view never follows the tongue, so it never shakes.
/// Identical inputs produce identical pixels without I/O, clock reads, or input mutation.
pub fn render_slither_frame(
    snake_development: &SnakeDevelopment,
    snake_morph: SnakeMorph,
    behavior: Behavior,
    frame_index: u32,
) -> SnakeFrame {
    let snake_anatomy = calculate_anatomy(snake_development.total_contributions);
    let pose = calculate_pose(behavior, frame_index);
    let pose_center = match pose.course {
        Course::Lap { .. } => [0.0, 0.0],
        Course::Hover { drift_pixels, .. } => {
            let hover_fit = fit_hover_loop(behavior, &snake_anatomy);
            [0, 1].map(|axis_index| {
                hover_fit.sweep_center[axis_index]
                    - hover_fit.drift_scales[axis_index] * drift_pixels[axis_index]
            })
        }
        Course::Coil { looseness, .. } => calculate_coil_center(&snake_anatomy, looseness),
    };
    render_pose(
        snake_development,
        snake_morph,
        &snake_anatomy,
        &pose,
        Framing::At(pose_center),
    )
}

/// Where the view is centered: on the snake itself for the still image, or at a point
/// fixed for the whole loop.
enum Framing {
    CenterOnSnake,
    At([f64; 2]),
}

/// One framing for a whole stretched loop.
struct HoverFit {
    sweep_center: [f64; 2],
    /// Shares of the pose's drift that keep every frame inside, from zero through one.
    drift_scales: [f64; 2],
}

/// The center of a coil at rest: its bounds without turn or breath, so the view stays
/// still while the coil breathes.
fn calculate_coil_center(snake_anatomy: &SnakeAnatomy, looseness: f64) -> [f64; 2] {
    let resting_pose = SnakePose {
        course: Course::Coil {
            turn_radians: 0.0,
            looseness,
        },
        breath: 0.0,
        ..STILL_POSE
    };
    let body_spine = build_spine(snake_anatomy, &resting_pose);
    let [min_corner, max_corner] = measure_curve_bounds(&body_spine);
    [0, 1].map(|axis_index| (min_corner[axis_index] + max_corner[axis_index]) / 2.0)
}

/// Center a stretched loop on everything the snake covers across its frames, then
/// shrink the drift on each axis only as far as needed to keep every frame inside the
/// frame.
fn fit_hover_loop(behavior: Behavior, snake_anatomy: &SnakeAnatomy) -> HoverFit {
    // Room for the outline, a clear pixel at the frame edge, and rounding.
    const FIT_MARGIN_PIXELS: f64 = 2.5;
    let frame_bounds: Vec<([[f64; 2]; 2], [f64; 2])> = (0..frame_count(behavior))
        .map(|frame_index| {
            let pose = calculate_pose(behavior, frame_index);
            let body_spine = build_spine(snake_anatomy, &pose);
            let [mut min_corner, mut max_corner] = measure_curve_bounds(&body_spine);
            // A fully extended tongue covers the pixel to its right and below.
            let full_tongue_pose = SnakePose {
                tongue_extension: 1.0,
                ..pose
            };
            for tongue_point in
                calculate_tongue_points(&body_spine, snake_anatomy, &full_tongue_pose)
            {
                for axis_index in 0..2 {
                    min_corner[axis_index] = min_corner[axis_index].min(tongue_point[axis_index]);
                    max_corner[axis_index] =
                        max_corner[axis_index].max(tongue_point[axis_index] + 1.0);
                }
            }
            let drift_pixels = match pose.course {
                Course::Hover { drift_pixels, .. } => drift_pixels,
                Course::Lap { .. } | Course::Coil { .. } => [0.0; 2],
            };
            ([min_corner, max_corner], drift_pixels)
        })
        .collect();
    let frame_sizes = [
        f64::from(FRAME_WIDTH_PIXELS),
        f64::from(FRAME_HEIGHT_PIXELS),
    ];
    let mut hover_fit = HoverFit {
        sweep_center: [0.0; 2],
        drift_scales: [1.0; 2],
    };
    for axis_index in 0..2 {
        let sweep_min = frame_bounds
            .iter()
            .map(|(bounds, _)| bounds[0][axis_index])
            .fold(f64::INFINITY, f64::min);
        let sweep_max = frame_bounds
            .iter()
            .map(|(bounds, _)| bounds[1][axis_index])
            .fold(f64::NEG_INFINITY, f64::max);
        let sweep_center = (sweep_min + sweep_max) / 2.0;
        let frame_half = frame_sizes[axis_index] / 2.0;
        let mut drift_scale: f64 = 1.0;
        for (bounds, drift_pixels) in &frame_bounds {
            let drift = drift_pixels[axis_index];
            let room = if drift > 0.0 {
                frame_half - FIT_MARGIN_PIXELS - (bounds[1][axis_index] - sweep_center)
            } else {
                frame_half - FIT_MARGIN_PIXELS + (bounds[0][axis_index] - sweep_center)
            };
            if drift != 0.0 {
                drift_scale = drift_scale.min(room / drift.abs());
            }
        }
        hover_fit.sweep_center[axis_index] = sweep_center;
        hover_fit.drift_scales[axis_index] = drift_scale.clamp(0.0, 1.0);
    }
    hover_fit
}

fn render_pose(
    snake_development: &SnakeDevelopment,
    snake_morph: SnakeMorph,
    snake_anatomy: &SnakeAnatomy,
    pose: &SnakePose,
    framing: Framing,
) -> SnakeFrame {
    let snake_coat = SnakeCoat::new(snake_development, snake_morph);
    let body_spine = build_spine(snake_anatomy, pose);
    let [min_corner, max_corner] = measure_curve_bounds(&body_spine);
    let pose_center = match framing {
        Framing::CenterOnSnake => [
            (min_corner[0] + max_corner[0]) / 2.0,
            (min_corner[1] + max_corner[1]) / 2.0,
        ],
        Framing::At(pose_center) => pose_center,
    };
    let snake_appearance = SnakeAppearance {
        snake_anatomy,
        snake_coat: &snake_coat,
        snake_morph,
    };
    let mut rgba_pixels = Vec::with_capacity((FRAME_WIDTH_PIXELS * FRAME_HEIGHT_PIXELS) as usize);
    for pixel_y in 0..FRAME_HEIGHT_PIXELS {
        for pixel_x in 0..FRAME_WIDTH_PIXELS {
            let sample_position = [
                f64::from(pixel_x) + 0.5 - f64::from(FRAME_WIDTH_PIXELS) / 2.0 + pose_center[0],
                f64::from(pixel_y) + 0.5 - f64::from(FRAME_HEIGHT_PIXELS) / 2.0 + pose_center[1],
            ];
            // Nothing outside the curve's bounds can be covered, so skip sampling it.
            let is_inside_bounds = (0..2).all(|axis_index| {
                (min_corner[axis_index]..=max_corner[axis_index])
                    .contains(&sample_position[axis_index])
            });
            rgba_pixels.push(if is_inside_bounds {
                sample_snake(
                    &body_spine,
                    sample_position,
                    [pixel_x, pixel_y],
                    &snake_appearance,
                )
            } else {
                [0; 4]
            });
        }
    }
    let tongue_points = calculate_tongue_points(&body_spine, snake_anatomy, pose);
    paint_tongue(
        &mut rgba_pixels,
        &tongue_points,
        pose_center,
        snake_morph.select_tongue_color(),
    );
    paint_outline(&mut rgba_pixels);
    SnakeFrame { rgba_pixels }
}

const OUTLINE_COLOR: [u8; 4] = [24, 20, 37, 255];
const DITHER_RANKS: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

struct SnakeAnatomy {
    body_length_pixels: f64,
    /// The full width of the thickest body.
    body_width_pixels: f64,
    head_length_pixels: f64,
    head_half_width_pixels: f64,
    /// Young snakes have proportionally larger eyes; adults use 1.
    eye_scale: f64,
}

struct SnakeAppearance<'a> {
    snake_anatomy: &'a SnakeAnatomy,
    snake_coat: &'a SnakeCoat,
    snake_morph: SnakeMorph,
}

// Positions and half widths are pixels; consecutive points form tapered strokes.
#[derive(Clone, Copy)]
struct CurvePoint {
    position: [f64; 2],
    half_width_pixels: f64,
}

struct CurveSample {
    along_fraction: f64,
    lateral_pixels: f64,
    distance_pixels: f64,
    half_width_pixels: f64,
    normal_direction: [f64; 2],
}

fn calculate_anatomy(total_contributions: u64) -> SnakeAnatomy {
    const MIN_BODY_LENGTH_PIXELS: f64 = 44.0;
    const ADULT_BODY_LENGTH_PIXELS: f64 = 190.0;
    const MAX_BODY_LENGTH_PIXELS: f64 = 260.0;
    const MIN_BODY_WIDTH_PIXELS: f64 = 6.0;
    const MAX_BODY_WIDTH_PIXELS: f64 = 24.0;
    const HEAD_LENGTH_PER_BODY_WIDTH: f64 = 1.6;
    const YOUNG_HEAD_HALF_WIDTH_PER_BODY_WIDTH: f64 = 0.72;
    const ADULT_HEAD_HALF_WIDTH_PER_BODY_WIDTH: f64 = 0.66;
    const YOUNG_EYE_SCALE: f64 = 1.7;
    let growth_fraction = calculate_growth_fraction(total_contributions);
    let adult_growth_fraction = calculate_growth_fraction(ADULT_CONTRIBUTIONS);
    let young_growth_fraction = (growth_fraction / adult_growth_fraction).min(1.0);
    let mature_growth_fraction =
        ((growth_fraction - adult_growth_fraction) / (1.0 - adult_growth_fraction)).max(0.0);
    let adult_shape_fraction =
        (growth_fraction / calculate_growth_fraction(YOUNG_SHAPE_CONTRIBUTIONS)).min(1.0);
    let body_length_pixels = MIN_BODY_LENGTH_PIXELS
        + (ADULT_BODY_LENGTH_PIXELS - MIN_BODY_LENGTH_PIXELS) * young_growth_fraction
        + (MAX_BODY_LENGTH_PIXELS - ADULT_BODY_LENGTH_PIXELS) * mature_growth_fraction;
    let length_fraction = (body_length_pixels - MIN_BODY_LENGTH_PIXELS)
        / (MAX_BODY_LENGTH_PIXELS - MIN_BODY_LENGTH_PIXELS);
    let body_width_pixels =
        interpolate(MIN_BODY_WIDTH_PIXELS, MAX_BODY_WIDTH_PIXELS, length_fraction);
    SnakeAnatomy {
        body_length_pixels,
        body_width_pixels,
        head_length_pixels: body_width_pixels * HEAD_LENGTH_PER_BODY_WIDTH,
        head_half_width_pixels: body_width_pixels
            * interpolate(
                YOUNG_HEAD_HALF_WIDTH_PER_BODY_WIDTH,
                ADULT_HEAD_HALF_WIDTH_PER_BODY_WIDTH,
                adult_shape_fraction,
            ),
        eye_scale: interpolate(YOUNG_EYE_SCALE, 1.0, adult_shape_fraction),
    }
}

fn interpolate(from: f64, to: f64, fraction: f64) -> f64 {
    from + (to - from) * fraction
}

/// Ease in and out across zero through one; values outside are clamped.
fn smooth_step(fraction: f64) -> f64 {
    let fraction = fraction.clamp(0.0, 1.0);
    fraction * fraction * (3.0 - 2.0 * fraction)
}

/// The body's half width a given distance from the snout: a rounded snout that widens
/// to the back of the head, a narrow neck, a thick even body, and a short blunt tail.
fn calculate_body_half_width(along_pixels: f64, snake_anatomy: &SnakeAnatomy) -> f64 {
    const WIDEST_HEAD_FRACTION: f64 = 0.72;
    const NOSE_WIDTH_FRACTION: f64 = 0.42;
    const NECK_END_HEAD_LENGTHS: f64 = 1.45;
    const NECK_HALF_WIDTH_FRACTION: f64 = 0.4;
    const SHOULDER_LENGTH_WIDTHS: f64 = 2.2;
    const TAIL_START_FRACTION: f64 = 0.86;
    const TAIL_TAPER_EXPONENT: f64 = 0.9;
    let head_length = snake_anatomy.head_length_pixels;
    let head_half_width = snake_anatomy.head_half_width_pixels;
    let body_width = snake_anatomy.body_width_pixels;
    let body_half_width = body_width / 2.0;
    let neck_half_width = body_width * NECK_HALF_WIDTH_FRACTION;
    let widest_head_pixels = head_length * WIDEST_HEAD_FRACTION;
    let neck_end_pixels = head_length * NECK_END_HEAD_LENGTHS;
    let shoulder_end_pixels = neck_end_pixels + SHOULDER_LENGTH_WIDTHS * body_width;
    let tail_start_pixels = snake_anatomy.body_length_pixels * TAIL_START_FRACTION;
    if along_pixels < widest_head_pixels {
        let nose_progress = along_pixels / widest_head_pixels;
        head_half_width
            * (NOSE_WIDTH_FRACTION
                + (1.0 - NOSE_WIDTH_FRACTION) * (1.0 - (1.0 - nose_progress).powi(2)).sqrt())
    } else if along_pixels < neck_end_pixels {
        interpolate(
            head_half_width,
            neck_half_width,
            smooth_step((along_pixels - widest_head_pixels) / (neck_end_pixels - widest_head_pixels)),
        )
    } else if along_pixels < shoulder_end_pixels {
        interpolate(
            neck_half_width,
            body_half_width,
            smooth_step((along_pixels - neck_end_pixels) / (shoulder_end_pixels - neck_end_pixels)),
        )
    } else if along_pixels < tail_start_pixels {
        body_half_width
    } else {
        let tail_progress = ((along_pixels - tail_start_pixels)
            / (snake_anatomy.body_length_pixels - tail_start_pixels))
            .clamp(0.0, 1.0);
        body_half_width * (1.0 - tail_progress.powf(TAIL_TAPER_EXPONENT))
    }
}

/// The spine follows the pose's course: the stretched snake's gentle bend, a clockwise
/// circle around the frame center, or a flat spiral. Body waves ripple through the
/// first two.
fn build_spine(snake_anatomy: &SnakeAnatomy, pose: &SnakePose) -> Vec<CurvePoint> {
    const MIN_HALF_WIDTH_PIXELS: f64 = 1.3;
    const HEADING_RADIANS: f64 = -0.08;
    const BEND_RADIANS: f64 = 2.4;
    const WAVE_AMPLITUDE_FRACTION: f64 = 0.07;
    const LAP_WAVE_CYCLES: f64 = 2.0;
    const HOVER_WAVE_CYCLES: f64 = 1.5;
    const HEAD_WAVE_FRACTION: f64 = 0.2;
    let body_length_pixels = snake_anatomy.body_length_pixels;
    let point_count = body_length_pixels.round() as usize;
    let calculate_point_half_width = |point_index: usize| {
        (calculate_body_half_width(point_index as f64, snake_anatomy) * (1.0 + pose.breath))
            .max(MIN_HALF_WIDTH_PIXELS)
    };
    let (center_positions, wave_cycles): (Vec<[f64; 2]>, f64) = match pose.course {
        Course::Hover {
            heading_offset_radians,
            ..
        } => {
            let mut center_positions = vec![[0.0, 0.0]];
            for point_index in 1..point_count {
                let backward_angle_radians = HEADING_RADIANS
                    + heading_offset_radians
                    + PI
                    + BEND_RADIANS * point_index as f64 / point_count as f64;
                let [previous_x, previous_y] = center_positions[point_index - 1];
                center_positions.push([
                    previous_x + backward_angle_radians.cos(),
                    previous_y + backward_angle_radians.sin(),
                ]);
            }
            (center_positions, HOVER_WAVE_CYCLES)
        }
        // Each body point sits one pixel of arc behind the one in front.
        Course::Lap { angle_radians } => {
            let radius_pixels = calculate_lap_radius(body_length_pixels);
            let center_positions = (0..point_count)
                .map(|point_index| {
                    let point_angle_radians = angle_radians - point_index as f64 / radius_pixels;
                    [
                        radius_pixels * point_angle_radians.cos(),
                        radius_pixels * point_angle_radians.sin(),
                    ]
                })
                .collect();
            (center_positions, LAP_WAVE_CYCLES)
        }
        Course::Coil {
            turn_radians,
            looseness,
        } => {
            return build_coil_spine(
                snake_anatomy,
                point_count,
                turn_radians,
                looseness,
                calculate_point_half_width,
            );
        }
    };
    let mut body_spine = Vec::with_capacity(point_count);
    for (point_index, center_position) in center_positions.iter().enumerate() {
        let along_fraction = point_index as f64 / point_count as f64;
        let forward_direction = calculate_forward_direction(&center_positions, point_index);
        let wave_envelope = HEAD_WAVE_FRACTION + (1.0 - HEAD_WAVE_FRACTION) * along_fraction;
        let wave_offset = body_length_pixels
            * WAVE_AMPLITUDE_FRACTION
            * pose.wave_strength
            * wave_envelope
            * (along_fraction * TAU * wave_cycles - pose.wave_phase_radians).sin();
        body_spine.push(CurvePoint {
            position: [
                center_position[0] - forward_direction[1] * wave_offset,
                center_position[1] + forward_direction[0] * wave_offset,
            ],
            half_width_pixels: calculate_point_half_width(point_index),
        });
    }
    body_spine
}

/// A flat spiral with the head at its center and the tail on the outside. Each turn lies
/// one body width (a little more when loose) outside the last, so the coils touch.
fn build_coil_spine(
    snake_anatomy: &SnakeAnatomy,
    point_count: usize,
    turn_radians: f64,
    looseness: f64,
    calculate_point_half_width: impl Fn(usize) -> f64,
) -> Vec<CurvePoint> {
    const START_RADIUS_PER_BODY_WIDTH: f64 = 0.3;
    const LOOSE_PITCH_GAIN: f64 = 0.6;
    const START_ANGLE_RADIANS: f64 = -2.2;
    let body_width = snake_anatomy.body_width_pixels;
    let radius_gain_per_radian =
        body_width * (0.92 + LOOSE_PITCH_GAIN * looseness) / TAU;
    let start_radius_pixels = body_width * START_RADIUS_PER_BODY_WIDTH;
    let mut spiral_angle_radians = 0.0_f64;
    let mut body_spine = Vec::with_capacity(point_count);
    for point_index in 0..point_count {
        let radius_pixels = start_radius_pixels + radius_gain_per_radian * spiral_angle_radians;
        let placed_angle_radians = START_ANGLE_RADIANS + turn_radians + spiral_angle_radians;
        body_spine.push(CurvePoint {
            position: [
                radius_pixels * placed_angle_radians.cos(),
                radius_pixels * placed_angle_radians.sin(),
            ],
            half_width_pixels: calculate_point_half_width(point_index),
        });
        // Advance one pixel of arc along the spiral.
        spiral_angle_radians += 1.0 / radius_pixels.hypot(radius_gain_per_radian);
    }
    body_spine
}

/// Where the tongue is: a stem straight out from the snout and two tines splayed from
/// its end, one point per pixel, swung together by the sway. Empty when it is in.
fn calculate_tongue_points(
    body_spine: &[CurvePoint],
    snake_anatomy: &SnakeAnatomy,
    pose: &SnakePose,
) -> Vec<[f64; 2]> {
    const MIN_EXTENSION: f64 = 0.12;
    const LENGTH_PER_HEAD_LENGTH: f64 = 1.1;
    const MIN_LENGTH_PIXELS: f64 = 4.0;
    const STEM_SHARE: f64 = 0.62;
    const TINE_SPLAY_RADIANS: f64 = 0.55;
    if pose.tongue_extension < MIN_EXTENSION {
        return Vec::new();
    }
    let tongue_length_pixels = pose.tongue_extension
        * (snake_anatomy.head_length_pixels * LENGTH_PER_HEAD_LENGTH).max(MIN_LENGTH_PIXELS);
    let snout = body_spine[0];
    let snout_positions: Vec<[f64; 2]> = body_spine
        .iter()
        .take(3)
        .map(|curve_point| curve_point.position)
        .collect();
    let forward_direction = calculate_forward_direction(&snout_positions, 0);
    let heading_radians =
        forward_direction[1].atan2(forward_direction[0]) + pose.tongue_sway_radians;
    let root_position = [
        snout.position[0] + forward_direction[0] * snout.half_width_pixels * 0.6,
        snout.position[1] + forward_direction[1] * snout.half_width_pixels * 0.6,
    ];
    let stem_steps = (tongue_length_pixels * STEM_SHARE).round().max(1.0) as u32;
    let tine_steps = (tongue_length_pixels * (1.0 - STEM_SHARE)).round().max(1.0) as u32;
    let mut tongue_points = Vec::new();
    for step_pixels in 1..=stem_steps {
        tongue_points.push([
            root_position[0] + heading_radians.cos() * f64::from(step_pixels),
            root_position[1] + heading_radians.sin() * f64::from(step_pixels),
        ]);
    }
    let fork_position = [
        root_position[0] + heading_radians.cos() * f64::from(stem_steps),
        root_position[1] + heading_radians.sin() * f64::from(stem_steps),
    ];
    for tine_side in [-1.0, 1.0] {
        let tine_angle = heading_radians + tine_side * TINE_SPLAY_RADIANS;
        for step_pixels in 1..=tine_steps {
            tongue_points.push([
                fork_position[0] + tine_angle.cos() * f64::from(step_pixels),
                fork_position[1] + tine_angle.sin() * f64::from(step_pixels),
            ]);
        }
    }
    tongue_points
}

/// Paint the tongue points over empty pixels, before the outline, so the tongue is
/// outlined like the snake.
fn paint_tongue(
    rgba_pixels: &mut [[u8; 4]],
    tongue_points: &[[f64; 2]],
    pose_center: [f64; 2],
    tongue_color: [u8; 4],
) {
    let frame_width = FRAME_WIDTH_PIXELS as usize;
    for tongue_point in tongue_points {
        let pixel_x =
            (tongue_point[0] - pose_center[0] + f64::from(FRAME_WIDTH_PIXELS) / 2.0).floor();
        let pixel_y =
            (tongue_point[1] - pose_center[1] + f64::from(FRAME_HEIGHT_PIXELS) / 2.0).floor();
        if !(0.0..f64::from(FRAME_WIDTH_PIXELS)).contains(&pixel_x)
            || !(0.0..f64::from(FRAME_HEIGHT_PIXELS)).contains(&pixel_y)
        {
            continue;
        }
        let pixel_index = pixel_y as usize * frame_width + pixel_x as usize;
        if rgba_pixels[pixel_index][3] == 0 {
            rgba_pixels[pixel_index] = tongue_color;
        }
    }
}

/// The lap circle's radius: as wide as the frame allows, tighter for longer snakes so
/// the whole snake stays inside, and never so tight that the body covers more than its
/// share of the circle.
fn calculate_lap_radius(body_length_pixels: f64) -> f64 {
    const FRAME_REACH_PIXELS: f64 = 116.0;
    const OUTER_REACH_FRACTION: f64 = 0.33;
    const MAX_LAP_RADIUS_PIXELS: f64 = 88.0;
    const MAX_BODY_SHARE_OF_LAP: f64 = 0.6;
    (FRAME_REACH_PIXELS - OUTER_REACH_FRACTION * body_length_pixels)
        .min(MAX_LAP_RADIUS_PIXELS)
        .max(body_length_pixels / (TAU * MAX_BODY_SHARE_OF_LAP))
}

/// The corners of the box that holds the curve, including its half widths.
fn measure_curve_bounds(body_spine: &[CurvePoint]) -> [[f64; 2]; 2] {
    let mut min_corner = [f64::INFINITY; 2];
    let mut max_corner = [f64::NEG_INFINITY; 2];
    for curve_point in body_spine {
        for axis_index in 0..2 {
            min_corner[axis_index] = min_corner[axis_index]
                .min(curve_point.position[axis_index] - curve_point.half_width_pixels);
            max_corner[axis_index] = max_corner[axis_index]
                .max(curve_point.position[axis_index] + curve_point.half_width_pixels);
        }
    }
    [min_corner, max_corner]
}

fn sample_snake(
    body_spine: &[CurvePoint],
    sample_position: [f64; 2],
    pixel_position: [u32; 2],
    snake_appearance: &SnakeAppearance,
) -> [u8; 4] {
    match sample_curve(body_spine, sample_position) {
        Some(body_sample) => shade_body(&body_sample, pixel_position, snake_appearance),
        None => [0; 4],
    }
}

// Constructed curves contain at least two points and no zero-length segments.
fn sample_curve(curve_points: &[CurvePoint], sample_position: [f64; 2]) -> Option<CurveSample> {
    let mut closest_sample = None;
    let mut closest_distance_pixels = f64::INFINITY;
    for (segment_index, end_points) in curve_points.windows(2).enumerate() {
        let [start_point, end_point] = [end_points[0], end_points[1]];
        let segment_x = end_point.position[0] - start_point.position[0];
        let segment_y = end_point.position[1] - start_point.position[1];
        let offset_x = sample_position[0] - start_point.position[0];
        let offset_y = sample_position[1] - start_point.position[1];
        let segment_length_pixels = segment_x.hypot(segment_y);
        let segment_progress = ((offset_x * segment_x + offset_y * segment_y)
            / segment_length_pixels.powi(2))
        .clamp(0.0, 1.0);
        let distance_pixels = (offset_x - segment_progress * segment_x)
            .hypot(offset_y - segment_progress * segment_y);
        if distance_pixels >= closest_distance_pixels {
            continue;
        }
        closest_distance_pixels = distance_pixels;
        closest_sample = Some(CurveSample {
            along_fraction: (segment_index as f64 + segment_progress)
                / (curve_points.len() - 1) as f64,
            lateral_pixels: (offset_x * segment_y - offset_y * segment_x) / segment_length_pixels,
            distance_pixels,
            half_width_pixels: start_point.half_width_pixels
                + segment_progress * (end_point.half_width_pixels - start_point.half_width_pixels),
            normal_direction: [
                segment_y / segment_length_pixels,
                -segment_x / segment_length_pixels,
            ],
        });
    }
    closest_sample
        .filter(|curve_sample| curve_sample.distance_pixels <= curve_sample.half_width_pixels)
}

fn shade_body(
    body_sample: &CurveSample,
    pixel_position: [u32; 2],
    snake_appearance: &SnakeAppearance,
) -> [u8; 4] {
    const INNER_RIM_PIXELS: f64 = 0.8;
    let snake_anatomy = snake_appearance.snake_anatomy;
    let along_pixels = body_sample.along_fraction * (snake_anatomy.body_length_pixels - 1.0);
    let lateral_fraction = body_sample.lateral_pixels / body_sample.half_width_pixels;
    let body_point = BodyPoint {
        along_pixels,
        lateral_fraction,
        half_width_pixels: body_sample.half_width_pixels,
        body_width_pixels: snake_anatomy.body_width_pixels,
        body_length_pixels: snake_anatomy.body_length_pixels,
        head_length_pixels: snake_anatomy.head_length_pixels,
    };
    let body_paint = snake_appearance
        .snake_coat
        .select_body_paint(&body_point, calculate_dither_threshold(pixel_position));
    if body_sample.distance_pixels > body_sample.half_width_pixels - INNER_RIM_PIXELS {
        return body_paint.shade_ramp[0];
    }
    if let Some(eye_color) = shade_eye(body_sample, along_pixels, snake_appearance) {
        return eye_color;
    }
    if let Some(glint_color) = body_paint.glint_color {
        return glint_color;
    }
    let surface_brightness = calculate_body_brightness(body_sample, lateral_fraction)
        + body_paint.brightness_shift;
    adjust_saturation(
        choose_shade(body_paint.shade_ramp, surface_brightness, pixel_position),
        body_paint.saturation,
    )
}

/// The eyes: a dark pupil with a ring of iris, on each side of the head.
fn shade_eye(
    body_sample: &CurveSample,
    along_pixels: f64,
    snake_appearance: &SnakeAppearance,
) -> Option<[u8; 4]> {
    const EYE_ALONG_HEAD_FRACTION: f64 = 0.46;
    const EYE_LATERAL_FRACTION: f64 = 0.58;
    const PUPIL_RADIUS_PIXELS: f64 = 0.9;
    const IRIS_RING_PIXELS: f64 = 0.9;
    let snake_anatomy = snake_appearance.snake_anatomy;
    let eye_along_pixels = snake_anatomy.head_length_pixels * EYE_ALONG_HEAD_FRACTION;
    if along_pixels > snake_anatomy.head_length_pixels {
        return None;
    }
    let eye_half_width = calculate_body_half_width(eye_along_pixels, snake_anatomy);
    let eye_lateral_pixels = eye_half_width * EYE_LATERAL_FRACTION;
    let offset_along = along_pixels - eye_along_pixels;
    let offset_across = body_sample.lateral_pixels.abs() - eye_lateral_pixels;
    let eye_distance = offset_along.hypot(offset_across);
    let pupil_radius = PUPIL_RADIUS_PIXELS * snake_anatomy.eye_scale;
    if eye_distance < pupil_radius {
        Some(OUTLINE_COLOR)
    } else if eye_distance < pupil_radius + IRIS_RING_PIXELS {
        Some(snake_appearance.snake_morph.select_iris_color())
    } else {
        None
    }
}

fn calculate_body_brightness(body_sample: &CurveSample, lateral_fraction: f64) -> f64 {
    const AMBIENT_BRIGHTNESS: f64 = 0.24;
    const DIFFUSE_STRENGTH: f64 = 0.7;
    const LIGHT_DIRECTION: [f64; 3] = [-0.32, -0.5, 0.8];
    let surface_normal_z = (1.0 - lateral_fraction.powi(2)).max(0.0).sqrt();
    let incident_light = lateral_fraction
        * (body_sample.normal_direction[0] * LIGHT_DIRECTION[0]
            + body_sample.normal_direction[1] * LIGHT_DIRECTION[1])
        + surface_normal_z * LIGHT_DIRECTION[2];
    AMBIENT_BRIGHTNESS + DIFFUSE_STRENGTH * incident_light.max(0.0)
}

fn calculate_forward_direction(curve_points: &[[f64; 2]], point_index: usize) -> [f64; 2] {
    const DIRECTION_SPAN_SAMPLES: usize = 2;
    let earlier_point = curve_points[point_index.saturating_sub(DIRECTION_SPAN_SAMPLES)];
    let later_point = curve_points[(point_index + DIRECTION_SPAN_SAMPLES).min(curve_points.len() - 1)];
    let tangent_x = earlier_point[0] - later_point[0];
    let tangent_y = earlier_point[1] - later_point[1];
    let tangent_length_pixels = tangent_x.hypot(tangent_y);
    [
        tangent_x / tangent_length_pixels,
        tangent_y / tangent_length_pixels,
    ]
}

fn paint_outline(rgba_pixels: &mut [[u8; 4]]) {
    let original_pixels = rgba_pixels.to_vec();
    let frame_width = FRAME_WIDTH_PIXELS as usize;
    for (pixel_index, pixel_channels) in rgba_pixels.iter_mut().enumerate() {
        if original_pixels[pixel_index][3] != 0 {
            continue;
        }
        let pixel_x = pixel_index % frame_width;
        let pixel_y = pixel_index / frame_width;
        let touches_shape = (pixel_x > 0 && original_pixels[pixel_index - 1][3] != 0)
            || (pixel_x + 1 < frame_width && original_pixels[pixel_index + 1][3] != 0)
            || (pixel_y > 0 && original_pixels[pixel_index - frame_width][3] != 0)
            || (pixel_y + 1 < FRAME_HEIGHT_PIXELS as usize
                && original_pixels[pixel_index + frame_width][3] != 0);
        if touches_shape {
            *pixel_channels = OUTLINE_COLOR;
        }
    }
}

fn choose_shade(
    shade_ramp: &[[u8; 4]],
    surface_brightness: f64,
    pixel_position: [u32; 2],
) -> [u8; 4] {
    let shade_position = surface_brightness.clamp(0.0, 1.0) * (shade_ramp.len() - 1) as f64;
    let base_index = shade_position.floor() as usize;
    let shade_index = base_index
        + usize::from(shade_position.fract() > calculate_dither_threshold(pixel_position));
    shade_ramp[shade_index.min(shade_ramp.len() - 1)]
}

/// Scale a color's distance from its own gray; one leaves it unchanged.
fn adjust_saturation(pixel_color: [u8; 4], saturation: f64) -> [u8; 4] {
    let [red, green, blue, alpha] = pixel_color.map(f64::from);
    let gray = 0.3 * red + 0.59 * green + 0.11 * blue;
    let saturate = |channel: f64| {
        (gray + (channel - gray) * saturation)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    [saturate(red), saturate(green), saturate(blue), alpha as u8]
}

/// The ordered-dither threshold at a pixel, from zero through one.
fn calculate_dither_threshold(pixel_position: [u32; 2]) -> f64 {
    let dither_x = pixel_position[0] as usize % DITHER_RANKS[0].len();
    let dither_y = pixel_position[1] as usize % DITHER_RANKS.len();
    (f64::from(DITHER_RANKS[dither_y][dither_x]) + 0.5)
        / (DITHER_RANKS.len() * DITHER_RANKS[0].len()) as f64
}

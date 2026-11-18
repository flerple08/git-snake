//! Ball python coats colored by language pigments on a chosen morph, sampled in body
//! coordinates.
//!
//! A ball python's pattern is a dark ground with a row of rounded blotches down each
//! flank, each with a dark spot in its middle (the "alien head" pattern), pale lips, and
//! a pale stripe above each eye. The caller chooses one of six [`SnakeMorph`] values,
//! which sets the ground and the blotches' native color. Languages then repaint the
//! blotches.
//!
//! Each language's GitHub color becomes the pigment nearest in hue: red (ember), orange
//! (amber), yellow (citrine), or blue (azure). Grays and languages without a color become
//! shadow. Languages on the same pigment add their shares, and equal shares rank ember,
//! shadow, citrine, amber, then azure. Each blotch along the body takes one pigment, and
//! the blotches are dealt out so each pigment covers its share of them. The second pigment
//! also colors the stripes above the eyes. A pigment that would blend into the morph's
//! ground swaps for a brighter or deeper shade of itself so it stays visible. Young
//! snakes keep their native blotch color until [`crate::snake::FULL_COLOR_CONTRIBUTIONS`]:
//! the colored pigments come in first, and shadow last. A snake without counted languages
//! shows its morph's native blotches.
//!
//! The streak marks from [`crate::markings`] show here. The current streak fills a
//! dithered share of the dorsal stripe with the morph's streak color. The longest streak
//! changes only the language colors, from dull through normal to extra bright and vivid,
//! with shadow never lightening, and then iridescent glints build on the blotches.

use std::collections::BTreeMap;
use std::ops::Range;

use crate::history::LanguageColor;
use crate::markings::{StreakMarks, calculate_streak_marks};
use crate::snake::{FULL_COLOR_CONTRIBUTIONS, SnakeDevelopment, calculate_growth_fraction};

const NORMAL_GROUND_RAMP: [[u8; 4]; 4] = [
    [34, 24, 20, 255],
    [58, 40, 28, 255],
    [86, 58, 36, 255],
    [118, 82, 48, 255],
];
const NORMAL_BLOTCH_RAMP: [[u8; 4]; 4] = [
    [120, 80, 40, 255],
    [168, 118, 58, 255],
    [204, 158, 84, 255],
    [232, 196, 120, 255],
];
const PASTEL_GROUND_RAMP: [[u8; 4]; 4] = [
    [70, 48, 28, 255],
    [110, 78, 40, 255],
    [150, 112, 60, 255],
    [190, 148, 84, 255],
];
const PASTEL_BLOTCH_RAMP: [[u8; 4]; 4] = [
    [176, 130, 52, 255],
    [214, 170, 70, 255],
    [240, 205, 105, 255],
    [255, 232, 150, 255],
];
const BANANA_GROUND_RAMP: [[u8; 4]; 4] = [
    [150, 112, 36, 255],
    [200, 160, 48, 255],
    [236, 202, 80, 255],
    [250, 232, 134, 255],
];
const BANANA_BLOTCH_RAMP: [[u8; 4]; 4] = [
    [120, 70, 30, 255],
    [165, 100, 42, 255],
    [205, 135, 60, 255],
    [232, 170, 90, 255],
];
const ALBINO_GROUND_RAMP: [[u8; 4]; 4] = [
    [186, 164, 130, 255],
    [220, 200, 160, 255],
    [242, 228, 190, 255],
    [255, 248, 224, 255],
];
const ALBINO_BLOTCH_RAMP: [[u8; 4]; 4] = [
    [200, 130, 30, 255],
    [236, 172, 50, 255],
    [252, 206, 88, 255],
    [255, 230, 130, 255],
];
const AXANTHIC_GROUND_RAMP: [[u8; 4]; 4] = [
    [26, 28, 34, 255],
    [48, 52, 62, 255],
    [76, 82, 94, 255],
    [108, 116, 130, 255],
];
const AXANTHIC_BLOTCH_RAMP: [[u8; 4]; 4] = [
    [96, 102, 112, 255],
    [134, 142, 154, 255],
    [172, 180, 192, 255],
    [208, 215, 225, 255],
];
const CINNAMON_GROUND_RAMP: [[u8; 4]; 4] = [
    [58, 28, 20, 255],
    [96, 46, 30, 255],
    [134, 70, 42, 255],
    [170, 100, 60, 255],
];
const CINNAMON_BLOTCH_RAMP: [[u8; 4]; 4] = [
    [150, 90, 40, 255],
    [190, 128, 56, 255],
    [220, 164, 84, 255],
    [242, 200, 122, 255],
];

const STREAK_GOLD_RAMP: [[u8; 4]; 3] = [
    [247, 118, 34, 255],
    [254, 174, 52, 255],
    [254, 231, 97, 255],
];
const STREAK_CYAN_RAMP: [[u8; 4]; 3] = [
    [18, 140, 190, 255],
    [44, 200, 235, 255],
    [160, 240, 250, 255],
];
const STREAK_RED_RAMP: [[u8; 4]; 3] = [
    [162, 38, 51, 255],
    [228, 59, 68, 255],
    [246, 117, 122, 255],
];

const DEEP_EMBER_RAMP: [[u8; 4]; 3] = [[90, 18, 30, 255], [140, 28, 42, 255], [190, 45, 55, 255]];
const BRIGHT_EMBER_RAMP: [[u8; 4]; 4] = [
    [228, 59, 68, 255],
    [240, 96, 79, 255],
    [246, 117, 122, 255],
    [255, 170, 170, 255],
];
const DEEP_AMBER_RAMP: [[u8; 4]; 3] = [[115, 62, 57, 255], [190, 74, 47, 255], [215, 118, 67, 255]];
const BRIGHT_AMBER_RAMP: [[u8; 4]; 4] = [
    [247, 118, 34, 255],
    [254, 174, 52, 255],
    [255, 200, 100, 255],
    [255, 225, 150, 255],
];
const DEEP_CITRINE_RAMP: [[u8; 4]; 3] = [
    [110, 76, 10, 255],
    [150, 106, 16, 255],
    [190, 140, 24, 255],
];
const BRIGHT_CITRINE_RAMP: [[u8; 4]; 4] = [
    [215, 118, 67, 255],
    [254, 174, 52, 255],
    [254, 231, 97, 255],
    [255, 243, 176, 255],
];
const DEEP_AZURE_RAMP: [[u8; 4]; 3] = [[18, 40, 90, 255], [28, 60, 130, 255], [40, 90, 170, 255]];
const BRIGHT_AZURE_RAMP: [[u8; 4]; 3] = [
    [18, 78, 137, 255],
    [40, 170, 235, 255],
    [90, 215, 255, 255],
];
const DEEP_SHADOW_RAMP: [[u8; 4]; 3] = [[24, 20, 37, 255], [38, 43, 68, 255], [58, 68, 102, 255]];
const BRIGHT_SHADOW_RAMP: [[u8; 4]; 4] = [
    [150, 152, 164, 255],
    [196, 198, 208, 255],
    [232, 234, 240, 255],
    [255, 255, 255, 255],
];

const GLINT_COLOR: [u8; 4] = [255, 255, 255, 255];
const TONGUE_COLOR: [u8; 4] = [228, 59, 68, 255];
const DARK_IRIS_COLOR: [u8; 4] = [226, 168, 52, 255];
const ALBINO_IRIS_COLOR: [u8; 4] = [236, 96, 112, 255];

/// How far from the back's center line the dorsal stripe reaches, as a share of the
/// half width.
const DORSAL_STRIPE_LATERAL_LIMIT: f64 = 0.13;
/// Blotches repeat this many body widths apart.
const BLOTCH_PERIOD_WIDTHS: f64 = 1.45;
const DULL_SATURATION: f64 = 0.3;
const VIVID_SATURATION_GAIN: f64 = 0.3;
const DULL_DARKENING: f64 = 0.15;
const VIVID_BRIGHTENING: f64 = 0.12;

/// The snake's morph, chosen by the profile owner: the ground color and native blotch
/// color the language pigments are painted over.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SnakeMorph {
    /// The wild type: dark brown with tan blotches.
    #[default]
    Normal,
    /// Pastel: a lighter golden ground with butter blotches.
    Pastel,
    /// Banana: a yellow ground with amber blotches.
    Banana,
    /// Albino: cream with yellow blotches and rosy eyes.
    Albino,
    /// Axanthic: black and gray.
    Axanthic,
    /// Cinnamon: a reddish brown ground.
    Cinnamon,
}

impl SnakeMorph {
    fn select_ground_ramp(self) -> &'static [[u8; 4]] {
        match self {
            Self::Normal => &NORMAL_GROUND_RAMP,
            Self::Pastel => &PASTEL_GROUND_RAMP,
            Self::Banana => &BANANA_GROUND_RAMP,
            Self::Albino => &ALBINO_GROUND_RAMP,
            Self::Axanthic => &AXANTHIC_GROUND_RAMP,
            Self::Cinnamon => &CINNAMON_GROUND_RAMP,
        }
    }

    /// The blotches' native color, shown by languages without color and by young snakes.
    fn select_blotch_ramp(self) -> &'static [[u8; 4]] {
        match self {
            Self::Normal => &NORMAL_BLOTCH_RAMP,
            Self::Pastel => &PASTEL_BLOTCH_RAMP,
            Self::Banana => &BANANA_BLOTCH_RAMP,
            Self::Albino => &ALBINO_BLOTCH_RAMP,
            Self::Axanthic => &AXANTHIC_BLOTCH_RAMP,
            Self::Cinnamon => &CINNAMON_BLOTCH_RAMP,
        }
    }

    /// The forked tongue's color.
    pub(crate) fn select_tongue_color(self) -> [u8; 4] {
        TONGUE_COLOR
    }

    /// The color around the eye's dark pupil.
    pub(crate) fn select_iris_color(self) -> [u8; 4] {
        match self {
            Self::Albino => ALBINO_IRIS_COLOR,
            _ => DARK_IRIS_COLOR,
        }
    }

    /// The current-streak stripe color, chosen to stand out against this morph's back.
    fn select_streak_ramp(self) -> &'static [[u8; 4]] {
        match self {
            Self::Normal | Self::Cinnamon | Self::Pastel => &STREAK_CYAN_RAMP,
            Self::Banana | Self::Albino => &STREAK_RED_RAMP,
            Self::Axanthic => &STREAK_GOLD_RAMP,
        }
    }

    /// The morph's version of a pigment: of the pigment's bright and deep shades, the
    /// one that contrasts more with this morph's ground, so no pigment disappears into
    /// it.
    fn select_pigment_ramp(self, pigment: Pigment) -> &'static [[u8; 4]] {
        let (deep_ramp, bright_ramp) = pigment.select_shades();
        let ground_brightness = measure_brightness(middle_color(self.select_ground_ramp()));
        let contrast_of =
            |ramp: &[[u8; 4]]| (measure_brightness(middle_color(ramp)) - ground_brightness).abs();
        if contrast_of(deep_ramp) > contrast_of(bright_ramp) {
            deep_ramp
        } else {
            bright_ramp
        }
    }
}

/// Snake pigments; each language color becomes the one nearest in hue.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Pigment {
    /// Red.
    Ember,
    /// Gray and black.
    Shadow,
    /// Yellow and green.
    Citrine,
    /// Orange.
    Amber,
    /// Blue and violet.
    Azure,
}

impl Pigment {
    /// The deep and bright shades the morph chooses between.
    fn select_shades(self) -> (&'static [[u8; 4]], &'static [[u8; 4]]) {
        match self {
            Self::Ember => (&DEEP_EMBER_RAMP, &BRIGHT_EMBER_RAMP),
            Self::Shadow => (&DEEP_SHADOW_RAMP, &BRIGHT_SHADOW_RAMP),
            Self::Citrine => (&DEEP_CITRINE_RAMP, &BRIGHT_CITRINE_RAMP),
            Self::Amber => (&DEEP_AMBER_RAMP, &BRIGHT_AMBER_RAMP),
            Self::Azure => (&DEEP_AZURE_RAMP, &BRIGHT_AZURE_RAMP),
        }
    }
}

fn middle_color(ramp: &[[u8; 4]]) -> [u8; 4] {
    ramp[ramp.len() / 2]
}

fn measure_brightness(color: [u8; 4]) -> f64 {
    0.3 * f64::from(color[0]) + 0.59 * f64::from(color[1]) + 0.11 * f64::from(color[2])
}

/// A snake's language pigments laid out on one morph.
///
/// Languages on the same pigment add their shares. Each blotch takes one pigment, dealt
/// so every pigment covers its share of the blotches; the second pigment also colors the
/// eye stripes. Young snakes show native blotches until FULL_COLOR_CONTRIBUTIONS. Streak
/// marks color the dorsal stripe and change only the language colors.
pub(crate) struct SnakeCoat {
    identity_seed: u64,
    snake_morph: SnakeMorph,
    /// Largest share first; equal shares keep pigment order.
    pigment_shares: Vec<(Pigment, f64)>,
    color_fraction: f64,
    streak_marks: StreakMarks,
}

/// How to color one pixel of the coat.
pub(crate) struct CoatPaint {
    pub(crate) shade_ramp: &'static [[u8; 4]],
    /// Added to the lighting before a shade is chosen.
    pub(crate) brightness_shift: f64,
    /// Scales a shaded color's distance from its own gray; one leaves it unchanged.
    pub(crate) saturation: f64,
    /// Replaces the shaded color, as on a glinting scale.
    pub(crate) glint_color: Option<[u8; 4]>,
}

impl CoatPaint {
    fn plain(shade_ramp: &'static [[u8; 4]]) -> Self {
        Self {
            shade_ramp,
            brightness_shift: 0.0,
            saturation: 1.0,
            glint_color: None,
        }
    }
}

/// Where a pixel sits on the snake's body, in body coordinates.
pub(crate) struct BodyPoint {
    /// Distance from the snout along the body.
    pub(crate) along_pixels: f64,
    /// Side to side across the body, from negative one to one.
    pub(crate) lateral_fraction: f64,
    /// The body's half width at this point.
    pub(crate) half_width_pixels: f64,
    /// The full width of the snake's thickest body.
    pub(crate) body_width_pixels: f64,
    pub(crate) body_length_pixels: f64,
    /// Where the head ends and the neck begins.
    pub(crate) head_length_pixels: f64,
}

impl SnakeCoat {
    /// A language without a recorded color counts as shadow.
    pub(crate) fn new(snake_development: &SnakeDevelopment, snake_morph: SnakeMorph) -> Self {
        Self {
            identity_seed: snake_development.identity_seed,
            snake_morph,
            pigment_shares: calculate_pigment_shares(snake_development),
            color_fraction: (calculate_growth_fraction(snake_development.total_contributions)
                / calculate_growth_fraction(FULL_COLOR_CONTRIBUTIONS))
            .min(1.0),
            streak_marks: calculate_streak_marks(snake_development),
        }
    }

    pub(crate) fn select_body_paint(
        &self,
        body_point: &BodyPoint,
        dither_threshold: f64,
    ) -> CoatPaint {
        const NECK_GAP_WIDTHS: f64 = 1.0;
        const TAIL_TIP_FRACTION: f64 = 0.09;
        let ground_ramp = self.snake_morph.select_ground_ramp();
        let body_width = body_point.body_width_pixels;
        let lateral_fraction = body_point.lateral_fraction;
        if body_point.along_pixels < body_point.head_length_pixels {
            return self.select_head_paint(body_point, dither_threshold);
        }
        if lateral_fraction.abs() < DORSAL_STRIPE_LATERAL_LIMIT
            && dither_threshold < self.streak_marks.streak_stripe_strength
        {
            return CoatPaint::plain(self.snake_morph.select_streak_ramp());
        }
        let body_start = body_point.head_length_pixels + NECK_GAP_WIDTHS * body_width;
        let tail_start = body_point.body_length_pixels * (1.0 - TAIL_TIP_FRACTION);
        if body_point.along_pixels < body_start || body_point.along_pixels > tail_start {
            return CoatPaint::plain(ground_ramp);
        }
        let cell_position = (body_point.along_pixels - body_start) / (BLOTCH_PERIOD_WIDTHS * body_width);
        let cell_index = cell_position.floor() as i64;
        let cell_offset = (cell_position.fract() - 0.5) * 2.0;
        let side_index = i64::from(lateral_fraction > 0.0);
        let side_shift =
            (hash_lattice_point(self.identity_seed ^ 0xb107, cell_index, side_index) - 0.5) * 0.3;
        let across = lateral_fraction.abs();
        let edge_wobble = (sample_fractal_noise(
            self.identity_seed ^ 0xa11ce,
            body_point.along_pixels / (body_width * 0.8) + side_index as f64 * 17.0,
            across * 2.2,
        ) - 0.5)
            * 0.45;
        let blotch_distance =
            ((cell_offset - side_shift) / 0.78).powi(2) + ((across - 0.6) / 0.36).powi(2);
        if blotch_distance >= 1.0 + edge_wobble {
            return CoatPaint::plain(ground_ramp);
        }
        let spot_distance =
            ((cell_offset - side_shift) / 0.2).powi(2) + ((across - 0.6) / 0.13).powi(2);
        if spot_distance < 1.0 {
            return CoatPaint::plain(ground_ramp);
        }
        match self.select_blotch_pigment(cell_index) {
            Some(pigment) => self.paint_pigment(pigment, body_point, dither_threshold),
            None => CoatPaint::plain(self.snake_morph.select_blotch_ramp()),
        }
    }

    /// The head: dark ground with pale lips and, above each eye, a stripe in the second
    /// pigment.
    fn select_head_paint(&self, body_point: &BodyPoint, dither_threshold: f64) -> CoatPaint {
        const LIP_LATERAL_START: f64 = 0.8;
        const STRIPE_LATERAL_SPAN: Range<f64> = 0.3..0.46;
        const STRIPE_ALONG_SPAN: Range<f64> = 0.22..0.95;
        let across = body_point.lateral_fraction.abs();
        let along_fraction = body_point.along_pixels / body_point.head_length_pixels;
        if across > LIP_LATERAL_START && body_point.along_pixels > 1.0 {
            return CoatPaint::plain(self.snake_morph.select_blotch_ramp());
        }
        if STRIPE_LATERAL_SPAN.contains(&across) && STRIPE_ALONG_SPAN.contains(&along_fraction) {
            return match self.pigment_shares.get(1) {
                Some(&(pigment, _)) => self.paint_pigment(pigment, body_point, dither_threshold),
                None => CoatPaint::plain(self.snake_morph.select_blotch_ramp()),
            };
        }
        CoatPaint::plain(self.snake_morph.select_ground_ramp())
    }

    /// The pigment's shades once it has come in, else the morph's native blotch color.
    fn paint_pigment(
        &self,
        pigment: Pigment,
        body_point: &BodyPoint,
        dither_threshold: f64,
    ) -> CoatPaint {
        if dither_threshold > calculate_pigment_development(pigment, self.color_fraction) {
            return CoatPaint::plain(self.snake_morph.select_blotch_ramp());
        }
        CoatPaint {
            shade_ramp: self.snake_morph.select_pigment_ramp(pigment),
            brightness_shift: self.calculate_vibrancy_shift(pigment),
            saturation: self.calculate_vibrancy_saturation(),
            glint_color: self.is_glinting(body_point).then_some(GLINT_COLOR),
        }
    }

    /// Deal blotches to pigments in a seeded low-discrepancy order, so each pigment
    /// covers about its share of them and neighbors differ.
    fn select_blotch_pigment(&self, cell_index: i64) -> Option<Pigment> {
        const GOLDEN_RATIO_FRACTION: f64 = 0.618_033_988_749_895;
        if self.pigment_shares.is_empty() {
            return None;
        }
        let seed_offset = hash_lattice_point(self.identity_seed, 0, 0);
        let draw = (cell_index as f64 * GOLDEN_RATIO_FRACTION + seed_offset).rem_euclid(1.0);
        let mut cumulative_share = 0.0;
        for &(pigment, pigment_share) in &self.pigment_shares {
            cumulative_share += pigment_share;
            if draw < cumulative_share {
                return Some(pigment);
            }
        }
        self.pigment_shares.last().map(|&(pigment, _)| pigment)
    }

    /// Dull language colors darken and vivid ones brighten, except shadow, which never
    /// lightens.
    fn calculate_vibrancy_shift(&self, pigment: Pigment) -> f64 {
        let color_vibrancy = self.streak_marks.color_vibrancy;
        if color_vibrancy < 1.0 {
            -DULL_DARKENING * (1.0 - color_vibrancy)
        } else if pigment == Pigment::Shadow {
            0.0
        } else {
            VIVID_BRIGHTENING * (color_vibrancy - 1.0)
        }
    }

    /// Dull language colors pull toward their own gray; vivid ones push away from it.
    fn calculate_vibrancy_saturation(&self) -> f64 {
        let color_vibrancy = self.streak_marks.color_vibrancy;
        if color_vibrancy < 1.0 {
            DULL_SATURATION + (1.0 - DULL_SATURATION) * color_vibrancy
        } else {
            1.0 + VIVID_SATURATION_GAIN * (color_vibrancy - 1.0)
        }
    }

    /// Iridescence: scattered scales on the blotches glint, more as glimmer builds.
    fn is_glinting(&self, body_point: &BodyPoint) -> bool {
        // Share of scales that glint at full glimmer.
        const FULL_GLINT_SHARE: f64 = 0.36;
        let glimmer_strength = self.streak_marks.glimmer_strength;
        if glimmer_strength <= 0.0 {
            return false;
        }
        let scale_row = (body_point.lateral_fraction + 1.0) * body_point.half_width_pixels / 2.0;
        let scale_column = body_point.along_pixels / 2.4 + scale_row.floor().rem_euclid(2.0) * 0.5;
        hash_lattice_point(
            self.identity_seed ^ 0x6119,
            scale_column.floor() as i64,
            scale_row.floor() as i64,
        ) < glimmer_strength * FULL_GLINT_SHARE
            && scale_row.fract() < 0.5
            && scale_column.fract() < 0.42
    }
}

/// Sum each pigment's language shares, largest first; equal shares keep pigment order.
fn calculate_pigment_shares(snake_development: &SnakeDevelopment) -> Vec<(Pigment, f64)> {
    let total_share: f64 = snake_development.language_shares.values().sum();
    let mut summed_shares = BTreeMap::new();
    for (language_name, language_share) in &snake_development.language_shares {
        let language_color = snake_development
            .language_colors
            .get(language_name)
            .copied()
            .flatten();
        *summed_shares
            .entry(select_pigment(language_color))
            .or_insert(0.0) += language_share / total_share;
    }
    // Sums of equal shares can differ in their last bits, so order by rounded shares.
    let rank_share = |pigment_share: f64| (pigment_share * 1e9).round();
    let mut pigment_shares: Vec<_> = summed_shares.into_iter().collect();
    pigment_shares.sort_by(|left, right| {
        rank_share(right.1)
            .total_cmp(&rank_share(left.1))
            .then(left.0.cmp(&right.0))
    });
    pigment_shares
}

/// Map a GitHub language color to the pigment nearest in hue. Grays and missing colors
/// become shadow.
fn select_pigment(language_color: Option<LanguageColor>) -> Pigment {
    const MIN_CHROMA: f64 = 0.12;
    const MIN_SATURATION: f64 = 0.2;
    let Some(LanguageColor(color_channels)) = language_color else {
        return Pigment::Shadow;
    };
    let [red, green, blue] = color_channels.map(|color_channel| f64::from(color_channel) / 255.0);
    let max_channel = red.max(green).max(blue);
    let min_channel = red.min(green).min(blue);
    let chroma = max_channel - min_channel;
    let lightness = (max_channel + min_channel) / 2.0;
    if chroma < MIN_CHROMA || chroma / (1.0 - (2.0 * lightness - 1.0).abs()) < MIN_SATURATION {
        return Pigment::Shadow;
    }
    let hue_degrees = if max_channel == red {
        60.0 * ((green - blue) / chroma).rem_euclid(6.0)
    } else if max_channel == green {
        60.0 * ((blue - red) / chroma + 2.0)
    } else {
        60.0 * ((red - green) / chroma + 4.0)
    };
    match hue_degrees {
        hue_degrees if !(18.0..330.0).contains(&hue_degrees) => Pigment::Ember,
        hue_degrees if hue_degrees < 45.0 => Pigment::Amber,
        hue_degrees if hue_degrees < 160.0 => Pigment::Citrine,
        _ => Pigment::Azure,
    }
}

/// How far a pigment has come in, from native (0) to full color (1).
fn calculate_pigment_development(pigment: Pigment, color_fraction: f64) -> f64 {
    let (start_fraction, span_fraction) = match pigment {
        Pigment::Shadow => (0.35, 0.6),
        _ => (0.05, 0.55),
    };
    let fraction = ((color_fraction - start_fraction) / span_fraction).clamp(0.0, 1.0);
    fraction * fraction * (3.0 - 2.0 * fraction)
}

fn sample_fractal_noise(identity_seed: u64, coordinate_x: f64, coordinate_y: f64) -> f64 {
    let mut noise_sum = 0.0;
    let mut total_weight = 0.0;
    for octave_index in 0..3 {
        let frequency = 2.0_f64.powi(octave_index);
        let noise_weight = 1.0 / frequency;
        noise_sum += sample_smooth_noise(
            identity_seed,
            coordinate_x * frequency,
            coordinate_y * frequency,
        ) * noise_weight;
        total_weight += noise_weight;
    }
    noise_sum / total_weight
}

fn sample_smooth_noise(identity_seed: u64, coordinate_x: f64, coordinate_y: f64) -> f64 {
    let lattice_x = coordinate_x.floor() as i64;
    let lattice_y = coordinate_y.floor() as i64;
    let fraction_x = coordinate_x - coordinate_x.floor();
    let fraction_y = coordinate_y - coordinate_y.floor();
    let blend_x = fraction_x * fraction_x * (3.0 - 2.0 * fraction_x);
    let blend_y = fraction_y * fraction_y * (3.0 - 2.0 * fraction_y);
    let top_left = hash_lattice_point(identity_seed, lattice_x, lattice_y);
    let top_right = hash_lattice_point(identity_seed, lattice_x + 1, lattice_y);
    let bottom_left = hash_lattice_point(identity_seed, lattice_x, lattice_y + 1);
    let bottom_right = hash_lattice_point(identity_seed, lattice_x + 1, lattice_y + 1);
    let top_noise = top_left + (top_right - top_left) * blend_x;
    let bottom_noise = bottom_left + (bottom_right - bottom_left) * blend_x;
    top_noise + (bottom_noise - top_noise) * blend_y
}

fn hash_lattice_point(identity_seed: u64, lattice_x: i64, lattice_y: i64) -> f64 {
    let mut hash_bits = identity_seed
        ^ (lattice_x as u64).wrapping_mul(0x9e3779b97f4a7c15)
        ^ (lattice_y as u64).wrapping_mul(0xbf58476d1ce4e5b9);
    hash_bits = (hash_bits ^ (hash_bits >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    hash_bits = (hash_bits ^ (hash_bits >> 27)).wrapping_mul(0x94d049bb133111eb);
    hash_bits ^= hash_bits >> 31;
    (hash_bits >> 11) as f64 / (1_u64 << 53) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_MORPHS: [SnakeMorph; 6] = [
        SnakeMorph::Normal,
        SnakeMorph::Pastel,
        SnakeMorph::Banana,
        SnakeMorph::Albino,
        SnakeMorph::Axanthic,
        SnakeMorph::Cinnamon,
    ];
    const ALL_PIGMENTS: [Pigment; 5] = [
        Pigment::Ember,
        Pigment::Shadow,
        Pigment::Citrine,
        Pigment::Amber,
        Pigment::Azure,
    ];
    /// A pigment's shade must differ at least this much in brightness from the ground's.
    const MIN_PIGMENT_CONTRAST: f64 = 40.0;

    #[test]
    fn every_pigment_stands_out_from_every_morphs_ground() {
        for snake_morph in ALL_MORPHS {
            let ground_brightness =
                measure_brightness(middle_color(snake_morph.select_ground_ramp()));
            for pigment in ALL_PIGMENTS {
                let pigment_brightness =
                    measure_brightness(middle_color(snake_morph.select_pigment_ramp(pigment)));
                assert!(
                    (pigment_brightness - ground_brightness).abs() >= MIN_PIGMENT_CONTRAST,
                    "{pigment:?} on {snake_morph:?}"
                );
            }
        }
    }

    #[test]
    fn language_hues_pick_the_nearest_pigment() {
        let pick = |channels: [u8; 3]| select_pigment(Some(LanguageColor(channels)));
        assert_eq!(pick([176, 114, 25]), Pigment::Amber);
        assert_eq!(pick([200, 30, 40]), Pigment::Ember);
        assert_eq!(pick([240, 220, 30]), Pigment::Citrine);
        assert_eq!(pick([49, 120, 198]), Pigment::Azure);
        assert_eq!(pick([128, 128, 128]), Pigment::Shadow);
        assert_eq!(select_pigment(None), Pigment::Shadow);
    }
}

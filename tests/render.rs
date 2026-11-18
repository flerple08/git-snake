use std::collections::BTreeSet;
use std::io::Cursor;

use chrono::{Datelike, Days, NaiveDate};
use git_snake::behavior::Behavior;
use git_snake::coat::SnakeMorph;
use git_snake::history::{AccountSnapshot, ContributionDay, ContributionWeek, LanguageColor};
use git_snake::motion::frame_count;
use git_snake::output::encode_png;
use git_snake::render::{
    FRAME_HEIGHT_PIXELS, FRAME_WIDTH_PIXELS, SnakeFrame, render_slither_frame, render_snake,
};
use git_snake::snake::{
    ADULT_CONTRIBUTIONS, FULL_COLOR_CONTRIBUTIONS, MAX_GROWTH_CONTRIBUTIONS, RecentStreak,
    SnakeDevelopment, derive_snake_development,
};

type LanguageMix<'a> = &'a [(&'a str, Option<&'a str>, f64)];

const TONGUE_COLOR: [u8; 4] = [228, 59, 68, 255];
const ALL_BEHAVIORS: [Behavior; 5] = [
    Behavior::DeepRest,
    Behavior::Rest,
    Behavior::Alert,
    Behavior::Dart,
    Behavior::Cruise,
];
const MORPHS: [SnakeMorph; 6] = [
    SnakeMorph::Normal,
    SnakeMorph::Pastel,
    SnakeMorph::Banana,
    SnakeMorph::Albino,
    SnakeMorph::Axanthic,
    SnakeMorph::Cinnamon,
];
const SINGLE_PIGMENT_MIXES: [LanguageMix; 5] = [
    &[("C++", Some("#f34b7d"), 1.0)],
    &[("Java", Some("#b07219"), 1.0)],
    &[("JavaScript", Some("#f1e05a"), 1.0)],
    &[("Python", Some("#3572a5"), 1.0)],
    &[("C", Some("#555555"), 1.0)],
];
const FOUR_LANGUAGE_MIX: LanguageMix = &[
    ("C", Some("#555555"), 0.25),
    ("Python", Some("#3572a5"), 0.25),
    ("C++", Some("#f34b7d"), 0.25),
    ("JavaScript", Some("#f1e05a"), 0.25),
];

#[test]
fn language_mixes_color_the_same_snake_reproducibly() {
    let uncolored_frame = render_snake(
        &develop_with_languages(ADULT_CONTRIBUTIONS, &[]),
        SnakeMorph::default(),
    );
    let mut distinct_images = BTreeSet::new();
    for language_mix in [
        SINGLE_PIGMENT_MIXES[0],
        &[
            ("Python", Some("#3572a5"), 0.65),
            ("C", Some("#555555"), 0.35),
        ],
        &[
            ("TypeScript", Some("#3178c6"), 0.45),
            ("C++", Some("#f34b7d"), 0.35),
            ("Shell", Some("#89e051"), 0.2),
        ],
        FOUR_LANGUAGE_MIX,
        &[("Rust", Some("#dea584"), 1.0)],
    ] {
        let snake_frame = render_snake(
            &develop_with_languages(ADULT_CONTRIBUTIONS, language_mix),
            SnakeMorph::default(),
        );
        assert_ne!(snake_frame, uncolored_frame);
        assert_same_shape(&uncolored_frame, &snake_frame);
        let png_bytes = encode_png(&snake_frame).unwrap();
        assert_eq!(
            png_bytes,
            encode_png(&render_snake(
                &develop_with_languages(ADULT_CONTRIBUTIONS, language_mix),
                SnakeMorph::default()
            ))
            .unwrap()
        );
        assert!(distinct_images.insert(png_bytes));
    }
}

#[test]
fn languages_on_one_pigment_share_its_blotches() {
    let render_mix = |language_mix| {
        render_snake(
            &develop_with_languages(ADULT_CONTRIBUTIONS, language_mix),
            SnakeMorph::default(),
        )
    };
    // Python and TypeScript are both blue; C's gray and a missing color are both shadow.
    assert_eq!(
        render_mix(&[
            ("Python", Some("#3572a5"), 0.5),
            ("TypeScript", Some("#3178c6"), 0.5),
        ]),
        render_mix(&[("Python", Some("#3572a5"), 1.0)])
    );
    assert_eq!(
        render_mix(&[("C", Some("#555555"), 1.0)]),
        render_mix(&[("Unrecognized", None, 1.0)])
    );
    assert_ne!(
        render_mix(&[
            ("Python", Some("#3572a5"), 0.5),
            ("Rust", Some("#dea584"), 0.5),
        ]),
        render_mix(&[("Python", Some("#3572a5"), 1.0)])
    );
}

#[test]
fn equal_pigment_shares_tie_despite_rounding() {
    let render_mix = |language_mix| {
        render_snake(
            &develop_with_languages(ADULT_CONTRIBUTIONS, language_mix),
            SnakeMorph::default(),
        )
    };
    // Orange sums to 0.30000000000000004 here; it still ties with yellow.
    assert_eq!(
        render_mix(&[
            ("Python", Some("#3572a5"), 0.4),
            ("Rust", Some("#dea584"), 0.1),
            ("Java", Some("#b07219"), 0.2),
            ("JavaScript", Some("#f1e05a"), 0.3),
        ]),
        render_mix(&[
            ("Python", Some("#3572a5"), 0.4),
            ("Java", Some("#b07219"), 0.3),
            ("JavaScript", Some("#f1e05a"), 0.3),
        ])
    );
}

#[test]
fn the_same_languages_form_different_blotches_on_another_account() {
    let mut snake_development = develop_with_languages(ADULT_CONTRIBUTIONS, FOUR_LANGUAGE_MIX);
    let original_frame = render_snake(&snake_development, SnakeMorph::default());
    snake_development.identity_seed += 1_u64 << 32;
    assert_ne!(
        original_frame,
        render_snake(&snake_development, SnakeMorph::default())
    );
}

#[test]
fn young_snakes_keep_their_native_blotches_until_full_color() {
    let render_with_blue = |contribution_total| {
        render_snake(
            &develop_with_languages(contribution_total, SINGLE_PIGMENT_MIXES[3]),
            SnakeMorph::default(),
        )
    };
    let render_native = |contribution_total| {
        render_snake(
            &develop_with_languages(contribution_total, &[]),
            SnakeMorph::default(),
        )
    };
    assert_eq!(render_with_blue(0), render_native(0));
    assert_ne!(
        render_with_blue(FULL_COLOR_CONTRIBUTIONS),
        render_native(FULL_COLOR_CONTRIBUTIONS)
    );
}

#[test]
fn every_morph_shows_every_pigment() {
    for snake_morph in MORPHS {
        let plain_frame = render_snake(
            &develop_with_languages(ADULT_CONTRIBUTIONS, &[]),
            snake_morph,
        );
        for language_mix in SINGLE_PIGMENT_MIXES {
            let colored_frame = render_snake(
                &develop_with_languages(ADULT_CONTRIBUTIONS, language_mix),
                snake_morph,
            );
            assert_ne!(
                colored_frame, plain_frame,
                "{language_mix:?} on {snake_morph:?}"
            );
        }
    }
}

#[test]
fn morphs_change_colors_but_not_shape() {
    let snake_development = develop_with_languages(ADULT_CONTRIBUTIONS, FOUR_LANGUAGE_MIX);
    let reference_frame = render_snake(&snake_development, MORPHS[0]);
    let mut distinct_color_sets = BTreeSet::new();
    for snake_morph in MORPHS {
        let snake_frame = render_snake(&snake_development, snake_morph);
        assert_same_shape(&reference_frame, &snake_frame);
        assert!(distinct_color_sets.insert(collect_opaque_colors(&snake_frame)));
    }
}

#[test]
fn the_current_streak_lights_a_dorsal_stripe_on_every_morph() {
    for snake_morph in MORPHS {
        let mut streaking_development = develop_with_languages(ADULT_CONTRIBUTIONS, &[]);
        streaking_development.streaks.recent = Some(RecentStreak::Current(build_weeks(13)));
        let plain_frame = render_snake(
            &develop_with_languages(ADULT_CONTRIBUTIONS, &[]),
            snake_morph,
        );
        let streaking_frame = render_snake(&streaking_development, snake_morph);
        assert_ne!(streaking_frame, plain_frame, "{snake_morph:?}");
        assert_same_shape(&plain_frame, &streaking_frame);
    }
}

#[test]
fn the_longest_streak_changes_only_language_colors() {
    let render_record = |week_count, language_mix| {
        let mut snake_development = develop_with_languages(ADULT_CONTRIBUTIONS, language_mix);
        snake_development.streaks.longest = build_weeks(week_count);
        render_snake(&snake_development, SnakeMorph::default())
    };
    assert_eq!(render_record(4, &[]), render_record(200, &[]));
    assert_ne!(
        render_record(4, FOUR_LANGUAGE_MIX),
        render_record(200, FOUR_LANGUAGE_MIX)
    );
}

#[test]
fn growth_never_shrinks_stops_at_the_maximum_and_stays_inside_the_frame() {
    let mut earlier_pixel_count = 0;
    let mut last_gain = 1;
    for contribution_total in [
        0,
        100,
        500,
        1_500,
        5_000,
        ADULT_CONTRIBUTIONS,
        40_000,
        MAX_GROWTH_CONTRIBUTIONS,
        500_000,
    ] {
        let snake_frame = render_snake(
            &develop_with_languages(contribution_total, FOUR_LANGUAGE_MIX),
            SnakeMorph::default(),
        );
        let pixel_count = collect_opaque_pixel_count(&snake_frame);
        assert!(pixel_count >= earlier_pixel_count, "{contribution_total}");
        last_gain = pixel_count - earlier_pixel_count;
        earlier_pixel_count = pixel_count;
        assert_inside_frame(&snake_frame, &format!("still at {contribution_total}"));
    }
    assert_eq!(last_gain, 0);
}

#[test]
fn every_behavior_keeps_the_largest_snake_inside_the_frame() {
    let largest_development = develop_with_languages(MAX_GROWTH_CONTRIBUTIONS, FOUR_LANGUAGE_MIX);
    for behavior in ALL_BEHAVIORS {
        for frame_index in (0..frame_count(behavior)).step_by(7) {
            let snake_frame = render_slither_frame(
                &largest_development,
                SnakeMorph::default(),
                behavior,
                frame_index,
            );
            assert_inside_frame(&snake_frame, &format!("{behavior:?} frame {frame_index}"));
        }
    }
}

#[test]
fn every_loop_closes_seamlessly() {
    let snake_development = develop_with_languages(1_500, FOUR_LANGUAGE_MIX);
    for behavior in ALL_BEHAVIORS {
        assert_eq!(
            render_slither_frame(&snake_development, SnakeMorph::default(), behavior, 0),
            render_slither_frame(
                &snake_development,
                SnakeMorph::default(),
                behavior,
                frame_count(behavior)
            ),
            "{behavior:?}"
        );
    }
}

#[test]
fn resting_coils_into_a_ball_and_slithering_stretches_out() {
    let snake_development = develop_with_languages(ADULT_CONTRIBUTIONS, FOUR_LANGUAGE_MIX);
    let extent_of = |behavior| {
        let bounds = measure_opaque_bounds(&render_slither_frame(
            &snake_development,
            SnakeMorph::default(),
            behavior,
            0,
        ));
        (bounds.right_x - bounds.left_x).max(bounds.bottom_y - bounds.top_y)
    };
    assert!(extent_of(Behavior::DeepRest) < 130);
    assert!(extent_of(Behavior::Rest) < 140);
    assert!(extent_of(Behavior::Alert) > 150);
    // A slithering snake's lap sweeps out a wide circle over its loop.
    let loop_extent = (0..frame_count(Behavior::Cruise))
        .step_by(10)
        .map(|frame_index| {
            let bounds = measure_opaque_bounds(&render_slither_frame(
                &snake_development,
                SnakeMorph::default(),
                Behavior::Cruise,
                frame_index,
            ));
            (bounds.right_x - bounds.left_x).max(bounds.bottom_y - bounds.top_y)
        })
        .max()
        .unwrap();
    assert!(loop_extent > 100);
    // The still image is the coil.
    let still_bounds =
        measure_opaque_bounds(&render_snake(&snake_development, SnakeMorph::default()));
    assert!(still_bounds.right_x - still_bounds.left_x < 130);
}

#[test]
fn the_tongue_flicks_out_and_deep_rest_keeps_it_in() {
    let snake_development = develop_with_languages(ADULT_CONTRIBUTIONS, SINGLE_PIGMENT_MIXES[3]);
    let has_tongue =
        |snake_frame: &SnakeFrame| collect_opaque_colors(snake_frame).contains(&TONGUE_COLOR);
    assert!(has_tongue(&render_snake(
        &snake_development,
        SnakeMorph::default()
    )));
    for frame_index in 0..frame_count(Behavior::DeepRest) {
        assert!(!has_tongue(&render_slither_frame(
            &snake_development,
            SnakeMorph::default(),
            Behavior::DeepRest,
            frame_index
        )));
    }
    let flicking_frames = (0..frame_count(Behavior::Alert))
        .filter(|&frame_index| {
            has_tongue(&render_slither_frame(
                &snake_development,
                SnakeMorph::default(),
                Behavior::Alert,
                frame_index,
            ))
        })
        .count();
    assert!(0 < flicking_frames && flicking_frames < frame_count(Behavior::Alert) as usize);
}

#[test]
fn hatchlings_are_far_smaller_than_full_grown_snakes() {
    let hatchling = render_snake(&develop_with_languages(0, &[]), SnakeMorph::default());
    let adult = render_snake(
        &develop_with_languages(MAX_GROWTH_CONTRIBUTIONS, &[]),
        SnakeMorph::default(),
    );
    assert!(collect_opaque_pixel_count(&hatchling) * 8 < collect_opaque_pixel_count(&adult));
}

/// Two frames have opaque pixels in exactly the same places.
fn assert_same_shape(first_frame: &SnakeFrame, second_frame: &SnakeFrame) {
    for (first_pixel, second_pixel) in first_frame
        .as_rgba_bytes()
        .as_chunks::<4>()
        .0
        .iter()
        .zip(second_frame.as_rgba_bytes().as_chunks::<4>().0.iter())
    {
        assert_eq!(first_pixel[3], second_pixel[3]);
    }
}

/// The snake and its outline leave a clear transparent pixel at every frame edge.
fn assert_inside_frame(snake_frame: &SnakeFrame, context: &str) {
    let bounds = measure_opaque_bounds(snake_frame);
    assert!(
        bounds.left_x > 0
            && bounds.top_y > 0
            && bounds.right_x < FRAME_WIDTH_PIXELS - 1
            && bounds.bottom_y < FRAME_HEIGHT_PIXELS - 1,
        "{context}: left {} top {} right {} bottom {}",
        bounds.left_x,
        bounds.top_y,
        bounds.right_x,
        bounds.bottom_y
    );
}

#[test]
fn png_output_doubles_each_pixel_and_repeats_exactly() {
    let mut snake_development = read_fixture_development();
    snake_development.total_contributions = 1_500;
    let snake_frame = render_snake(&snake_development, SnakeMorph::default());
    let png_bytes = encode_png(&snake_frame).unwrap();
    assert_eq!(
        png_bytes,
        encode_png(&render_snake(&snake_development, SnakeMorph::default())).unwrap()
    );
    let mut png_reader = png::Decoder::new(Cursor::new(&png_bytes))
        .read_info()
        .unwrap();
    let mut decoded_pixels = vec![0; png_reader.output_buffer_size().unwrap()];
    let frame_info = png_reader.next_frame(&mut decoded_pixels).unwrap();
    assert_eq!((frame_info.width, frame_info.height), (512, 512));
    assert_eq!(frame_info.color_type, png::ColorType::Rgba);
    for (pixel_index, native_channels) in snake_frame
        .as_rgba_bytes()
        .as_chunks::<4>()
        .0
        .iter()
        .enumerate()
    {
        let block_left = pixel_index as u32 % FRAME_WIDTH_PIXELS * 2;
        let block_top = pixel_index as u32 / FRAME_WIDTH_PIXELS * 2;
        for (offset_x, offset_y) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            let export_index =
                ((block_top + offset_y) * frame_info.width + block_left + offset_x) as usize * 4;
            assert_eq!(
                &decoded_pixels[export_index..export_index + 4],
                native_channels
            );
        }
    }
}

#[test]
fn quiet_days_change_streak_state_without_changing_the_anatomy() {
    let mut account_snapshot: AccountSnapshot =
        serde_json::from_str(include_str!("fixtures/account-with-history.json")).unwrap();
    let active_development = derive_snake_development(&account_snapshot).unwrap();
    let contribution_history = account_snapshot.contribution_history.as_mut().unwrap();
    for day_offset in 1..=14 {
        contribution_history.days.push(ContributionDay {
            date: contribution_history
                .observed_on
                .checked_add_days(Days::new(day_offset))
                .unwrap(),
            contribution_count: 0,
        });
    }
    contribution_history.observed_on = contribution_history.days.last().unwrap().date;
    let quiet_development = derive_snake_development(&account_snapshot).unwrap();
    assert_ne!(active_development.streaks, quiet_development.streaks);
    let active_frame = render_snake(&active_development, SnakeMorph::default());
    let quiet_frame = render_snake(&quiet_development, SnakeMorph::default());
    for (active_pixel, quiet_pixel) in active_frame
        .as_rgba_bytes()
        .as_chunks::<4>()
        .0
        .iter()
        .zip(quiet_frame.as_rgba_bytes().as_chunks::<4>().0.iter())
    {
        assert_eq!(active_pixel[3], quiet_pixel[3]);
    }
}

fn read_fixture_development() -> SnakeDevelopment {
    let account_snapshot =
        serde_json::from_str(include_str!("fixtures/account-with-history.json")).unwrap();
    derive_snake_development(&account_snapshot).unwrap()
}

fn develop_with_languages(contribution_total: u64, language_mix: LanguageMix) -> SnakeDevelopment {
    let mut snake_development = read_fixture_development();
    snake_development.total_contributions = contribution_total;
    snake_development.language_shares = language_mix
        .iter()
        .map(|&(language_name, _, language_share)| (language_name.to_owned(), language_share))
        .collect();
    snake_development.language_colors = language_mix
        .iter()
        .map(|&(language_name, color_text, _)| {
            (
                language_name.to_owned(),
                color_text.map(|color_text| LanguageColor::try_from(color_text).unwrap()),
            )
        })
        .collect();
    // Normal colors and no streak stripe: a four-week record that ended long ago.
    snake_development.streaks.longest = build_weeks(4);
    snake_development.streaks.recent = Some(RecentStreak::Ended {
        weeks: build_weeks(4),
        ended_on: snake_development.observed_on - Days::new(70),
    });
    snake_development
}

/// Fully active weeks; marks depend only on how many there are.
fn build_weeks(week_count: usize) -> Vec<ContributionWeek> {
    let first_monday: NaiveDate = "2024-01-01".parse().unwrap();
    (0..week_count)
        .map(|week_index| ContributionWeek {
            iso_week: (first_monday + Days::new(7 * week_index as u64)).iso_week(),
            active_day_count: 7,
            observed_day_count: 7,
        })
        .collect()
}

fn collect_opaque_pixel_count(snake_frame: &SnakeFrame) -> usize {
    snake_frame
        .as_rgba_bytes()
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|pixel_channels| pixel_channels[3] != 0)
        .count()
}

fn collect_opaque_colors(snake_frame: &SnakeFrame) -> BTreeSet<[u8; 4]> {
    snake_frame
        .as_rgba_bytes()
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|pixel_channels| pixel_channels[3] != 0)
        .copied()
        .collect()
}

struct OpaqueBounds {
    left_x: u32,
    top_y: u32,
    right_x: u32,
    bottom_y: u32,
}

fn measure_opaque_bounds(snake_frame: &SnakeFrame) -> OpaqueBounds {
    let mut opaque_bounds = OpaqueBounds {
        left_x: FRAME_WIDTH_PIXELS,
        top_y: FRAME_HEIGHT_PIXELS,
        right_x: 0,
        bottom_y: 0,
    };
    for (pixel_index, pixel_channels) in snake_frame
        .as_rgba_bytes()
        .as_chunks::<4>()
        .0
        .iter()
        .enumerate()
    {
        if pixel_channels[3] == 0 {
            continue;
        }
        let pixel_x = pixel_index as u32 % FRAME_WIDTH_PIXELS;
        let pixel_y = pixel_index as u32 / FRAME_WIDTH_PIXELS;
        opaque_bounds.left_x = opaque_bounds.left_x.min(pixel_x);
        opaque_bounds.top_y = opaque_bounds.top_y.min(pixel_y);
        opaque_bounds.right_x = opaque_bounds.right_x.max(pixel_x);
        opaque_bounds.bottom_y = opaque_bounds.bottom_y.max(pixel_y);
    }
    assert!(
        opaque_bounds.left_x < opaque_bounds.right_x
            && opaque_bounds.top_y < opaque_bounds.bottom_y
    );
    opaque_bounds
}

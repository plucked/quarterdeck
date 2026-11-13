use super::*;

fn cells(label: &str) -> Vec<Cell> {
    parts(label)
        .into_iter()
        .enumerate()
        .map(|(index, part)| Cell {
            key: part.key,
            text: label[part.range].to_owned(),
            wheel: part.digit.map(f32::from),
            x: f32::from(u16::try_from(index).unwrap()) * 8.,
            y: 0.,
            width: 8.,
            opacity: 1.,
        })
        .collect()
}

fn digit(frame: &[Cell], run: usize, place: i32) -> &Cell {
    frame
        .iter()
        .find(|cell| cell.key == Key::Digit { run, place })
        .unwrap()
}

#[test]
fn first_render_and_unchanged_values_do_not_animate() {
    let now = Instant::now();
    let mut flow = Transition::default();
    flow.update("12", cells("12"), now, true);
    assert!(!flow.is_active(now));
    flow.update("12", cells("12"), now + DURATION, true);
    assert!(!flow.is_active(now + DURATION));
    assert_eq!(flow.frame(now).len(), 2);
}

#[test]
fn carries_and_borrows_roll_in_the_values_direction() {
    let now = Instant::now();
    let mut flow = Transition::default();
    flow.update("99", cells("99"), now, true);
    flow.update("100", cells("100"), now, true);
    assert_eq!(digit(&flow.frame(now), 0, 0).wheel, Some(9.));
    assert_eq!(digit(&flow.frame(now + DURATION), 0, 0).wheel, Some(10.));
    assert_eq!(digit(&flow.frame(now), 0, 2).opacity, 0.);
    flow.update("99", cells("99"), now + DURATION, true);
    assert_eq!(digit(&flow.frame(now + DURATION * 2), 0, 0).wheel, Some(9.));
    assert_eq!(digit(&flow.frame(now + DURATION * 2), 0, 2).opacity, 0.);
}

#[test]
fn multiple_numbers_have_independent_trends_and_preserve_formatting() {
    let label = "−003 / 12 · $1.25 MB";
    let parsed = parts(label);
    assert_eq!(
        parsed
            .iter()
            .map(|part| &label[part.range.clone()])
            .collect::<String>(),
        label
    );
    assert_eq!(trend("3 of 12", "4 of 10", 0), Ordering::Greater);
    assert_eq!(trend("3 of 12", "4 of 10", 1), Ordering::Less);
    assert_eq!(trend("0003", "3", 0), Ordering::Equal);
    assert_eq!(
        trend("18446744073709551615", "18446744073709551616", 0),
        Ordering::Greater
    );
}

#[test]
fn an_interrupted_animation_retargets_without_a_visual_jump() {
    let now = Instant::now();
    let mut flow = Transition::default();
    flow.update("8", cells("8"), now, true);
    flow.update("12", cells("12"), now, true);
    let halfway = now + DURATION / 3;
    let before = flow.frame(halfway);
    flow.update("7", cells("7"), halfway, true);
    let after = flow.frame(halfway);
    assert_eq!(digit(&before, 0, 0).wheel, digit(&after, 0, 0).wheel);
    assert_eq!(digit(&before, 0, 0).x, digit(&after, 0, 0).x);
    assert_eq!(digit(&before, 0, 1).opacity, digit(&after, 0, 1).opacity);
    assert_eq!(digit(&flow.frame(halfway + DURATION), 0, 0).wheel, Some(7.));
}

#[test]
fn reduced_motion_finishes_an_active_roll_immediately() {
    let now = Instant::now();
    let mut flow = Transition::default();
    flow.update("1", cells("1"), now, true);
    flow.update("9", cells("9"), now, true);
    flow.update("9", cells("9"), now + DURATION / 4, false);
    assert!(!flow.is_active(now + DURATION / 4));
    assert_eq!(digit(&flow.frame(now), 0, 0).wheel, Some(9.));
}

#[test]
fn ordinary_rerenders_do_not_restart_a_roll() {
    let now = Instant::now();
    let mut flow = Transition::default();
    flow.update("1", cells("1"), now, true);
    flow.update("9", cells("9"), now, true);
    flow.update("9", cells("9"), now + DURATION / 2, true);
    assert!(!flow.is_active(now + DURATION));
    assert_eq!(digit(&flow.frame(now + DURATION), 0, 0).wheel, Some(9.));
}

#[test]
fn decimals_and_grouping_keep_the_same_place_values() {
    let before = cells("999.9");
    let after = cells("1,000.12");
    assert_eq!(digit(&before, 0, -1).text, "9");
    assert_eq!(digit(&after, 0, -1).text, "1");
    assert_eq!(digit(&after, 0, -2).text, "2");
    assert_eq!(digit(&after, 0, 3).text, "1");
    assert_eq!(trend("1.99", "2.00", 0), Ordering::Greater);
    assert_eq!(trend("1,000.0", "999.99", 0), Ordering::Less);
    assert_eq!(trend("−12", "−13", 0), Ordering::Less);
    assert_eq!(trend("0.20", "0.2", 0), Ordering::Equal);
}

#[test]
fn crossing_zero_never_paints_a_minus_sign() {
    assert_eq!(face(-0., 0), "0");
    assert_eq!(face(-10., 0), "0");
    assert_eq!(face(-0.5, 0), "9");
    assert_eq!(face(-0.5, 1), "0");
}

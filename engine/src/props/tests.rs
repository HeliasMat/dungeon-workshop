use glam::Vec2;

use super::*;
use crate::beat::Frames;

fn words(text: &str) -> Vec<String> {
    text.split_whitespace().map(str::to_string).collect()
}

fn given(prop: PropId, text: &str) -> Result<(Given<String>, usize), ValueError> {
    read(prop.prop(), &words(text))
}

fn is(prop: PropId, text: &str) -> Value {
    match given(prop, text) {
        Ok((Given::Is(value), _)) => value,
        other => panic!("{text}: {other:?}"),
    }
}

#[test]
fn every_property_is_in_the_registry_once_and_whole() {
    assert_eq!(PROPS.len(), PropId::ALL.len());
    for id in PropId::ALL {
        assert_eq!(PROPS.iter().filter(|prop| prop.id == id).count(), 1, "{id:?}");
    }
    for (at, prop) in PROPS.iter().enumerate() {
        // (Names unique.)
        for other in &PROPS[..at] {
            assert!(prop.name != other.name, "“{}” twice", prop.name);
        }
        assert!(!prop.name.is_empty() && !prop.doc.is_empty(), "{:?}", prop.id);
        assert!(prop.name.chars().all(|letter| letter.is_ascii_lowercase() || letter == '_'), "{}", prop.name);
        let fits = match (prop.ty, prop.default) {
            (Type::Number, Value::Number(_) | Value::Nothing) | (Type::Bool, Value::Bool(_)) | (Type::Point, Value::Point(_)) => true,
            (Type::Colour, Value::Colour(_) | Value::Nothing) => true,
            (Type::Choice(choices), Value::Choice(word)) => choices.contains(&word),
            _ => false,
        };
        assert!(fits, "{}: its default is of its type", prop.name);
        assert!(!(prop.animatable && matches!(prop.ty, Type::Bool | Type::Choice(_))), "{}: a switch does not go by degrees", prop.name);
        assert!(prop.kinds.iter().next().is_some(), "{}: of some kind", prop.name);
        if let Some((least, most)) = prop.advised {
            assert!(least <= most, "{}", prop.name);
        }
        assert_eq!(prop.check(&prop.default), Ok(None), "{}: its default is fine", prop.name);
        // (Its default written is read back.)
        if prop.default != Value::Nothing {
            assert_eq!(read(prop, &words(&prop.default.to_string())).map(|(given, _)| given), Ok(Given::Is(prop.default)), "{}", prop.name);
        }
    }
}

#[test]
fn every_kind_has_its_place_and_its_looks() {
    for kind in NodeKind::ALL {
        let props: Vec<PropId> = of_kind(kind).map(|prop| prop.id).collect();
        for id in [PropId::Position, PropId::Angle, PropId::Scale, PropId::Opacity, PropId::Tint, PropId::Z, PropId::Visible] {
            assert!(props.contains(&id), "{kind:?} has {id:?}");
        }
        assert_eq!(NodeKind::named(kind.name()), Some(kind));
    }
    assert!(!PropId::Radius.prop().fits(NodeKind::Body));
    assert!(PropId::Sink.prop().fits(NodeKind::Thing));
}

#[test]
fn properties_by_name_and_the_nearest() {
    assert_eq!(find("opacity").map(|prop| prop.id), Some(PropId::Opacity));
    assert_eq!(find("scale_x").map(|prop| prop.id), Some(PropId::ScaleX));
    assert_eq!(find("opactiy"), None);
    assert_eq!(closest("opactiy").first(), Some(&"opacity"));
    assert_eq!(closest("scal").first(), Some(&"scale"));
    assert!(closest("zzzzzzzzzz").is_empty());
    let wrong = unknown("opactiy");
    assert_eq!((wrong.code, wrong.fix.as_deref()), (codes::PROP_UNKNOWN, Some("opacity")));
}

#[test]
fn durations_distances_angles_factors_shares() {
    assert_eq!(duration("2s"), Some(Frames(120.0)));
    assert_eq!(duration("1.5s"), Some(Frames(90.0)));
    assert_eq!(duration("250ms"), Some(Frames(15.0)));
    assert_eq!(duration("30f"), Some(Frames(30.0)));
    assert_eq!(duration("12"), Some(Frames(12.0)));
    assert_eq!(duration("two"), None);
    assert_eq!(number_in("12px", Unit::Px), Some(12.0));
    assert_eq!(number_in("12", Unit::Px), Some(12.0));
    assert_eq!(number_in("12deg", Unit::Px), None);
    assert_eq!(number_in("45deg", Unit::Deg), Some(45.0));
    assert_eq!(number_in("45°", Unit::Deg), Some(45.0));
    assert_eq!(number_in("0.5turn", Unit::Deg), Some(180.0));
    assert_eq!(number_in("2turns", Unit::Deg), Some(720.0));
    assert_eq!(number_in("150%", Unit::Factor), Some(1.5));
    assert_eq!(number_in("50%", Unit::Ratio), Some(0.5));
    assert_eq!(number_in("5/s", Unit::PerSecond), Some(5.0));
    assert_eq!(number_in("2s", Unit::Frames), Some(120.0));
    assert_eq!(is(PropId::Angle, "0.25turn"), Value::Number(90.0));
    assert_eq!(is(PropId::Opacity, "50%"), Value::Number(0.5));
    assert_eq!(is(PropId::Scale, "150%"), Value::Number(1.5));
    assert_eq!(is(PropId::Height, "12px"), Value::Number(12.0));
}

#[test]
fn colours_every_way_they_are_written() {
    assert_eq!(is(PropId::Tint, "#ff8800"), Value::Colour([255, 136, 0, 255]));
    assert_eq!(is(PropId::Tint, "#ff880080"), Value::Colour([255, 136, 0, 128]));
    assert_eq!(is(PropId::Tint, "#f80"), Value::Colour([255, 136, 0, 255]));
    assert_eq!(is(PropId::Tint, "#f808"), Value::Colour([255, 136, 0, 136]));
    assert_eq!(is(PropId::Tint, "red"), Value::Colour([255, 40, 40, 255]));
    assert_eq!(is(PropId::Tint, "rgb(255,136,0)"), Value::Colour([255, 136, 0, 255]));
    assert_eq!(is(PropId::Tint, "rgba(255,136,0,0.5)"), Value::Colour([255, 136, 0, 128]));
    assert_eq!(is(PropId::Tint, "rgba(255,136,0,50%)"), Value::Colour([255, 136, 0, 128]));
    // (Spread over words: all of them taken.)
    assert_eq!(given(PropId::Tint, "rgb(255, 136, 0) angle 3"), Ok((Given::Is(Value::Colour([255, 136, 0, 255])), 3)));
    for wrong in ["#ff888", "#gg8800", "rgb(300,0,0)", "rgb(1,2)", "reddish", "12"] {
        assert_eq!(given(PropId::Tint, wrong).map_err(|wrong| wrong.code), Err(codes::VALUE_TYPE), "{wrong}");
    }
    assert_eq!(is(PropId::Fill, "none"), Value::Nothing);
    assert_eq!(Value::Colour([255, 136, 0, 255]).to_string(), "#ff8800");
    assert_eq!(Value::Colour([255, 136, 0, 128]).to_string(), "#ff880080");
}

#[test]
fn points_as_numbers_as_a_node_and_beside_it() {
    assert_eq!(given(PropId::Position, "10 20"), Ok((Given::Is(Value::Point(Vec2::new(10.0, 20.0))), 2)));
    assert_eq!(given(PropId::Position, "10px,20px"), Ok((Given::Is(Value::Point(Vec2::new(10.0, 20.0))), 1)));
    assert_eq!(given(PropId::Position, "@a"), Ok((Given::At { node: "a".to_string(), by: Vec2::ZERO }, 1)));
    assert_eq!(given(PropId::Position, "@a+10,-4"), Ok((Given::At { node: "a".to_string(), by: Vec2::new(10.0, -4.0) }, 1)));
    assert_eq!(given(PropId::Position, "@m25.1-3,2"), Ok((Given::At { node: "m25.1".to_string(), by: Vec2::new(-3.0, 2.0) }, 1)));
    assert_eq!(given(PropId::Position, "+10,-4"), Ok((Given::Plus(Value::Point(Vec2::new(10.0, -4.0))), 1)));
    assert_eq!(given(PropId::Position, "10").map_err(|wrong| wrong.code), Err(codes::VALUE_TYPE));
    assert_eq!(given(PropId::Position, "").map_err(|wrong| wrong.code), Err(codes::VALUE_MISSING));
}

#[test]
fn relative_values_switches_choices_and_the_default() {
    assert_eq!(given(PropId::Angle, "+30"), Ok((Given::Plus(Value::Number(30.0)), 1)));
    assert_eq!(given(PropId::Angle, "+-30"), Ok((Given::Plus(Value::Number(-30.0)), 1)));
    assert_eq!(given(PropId::Angle, "-30"), Ok((Given::Is(Value::Number(-30.0)), 1)));
    assert_eq!(given(PropId::Scale, "x1.5"), Ok((Given::Times(1.5), 1)));
    assert_eq!(given(PropId::Scale, "default"), Ok((Given::Default, 1)));
    assert_eq!(is(PropId::FlipX, "on"), Value::Bool(true));
    assert_eq!(is(PropId::Visible, "no"), Value::Bool(false));
    assert_eq!(is(PropId::Blend, "add"), Value::Choice("add"));
    assert_eq!(is(PropId::Facing, "upleft"), Value::Choice("upleft"));
    assert_eq!(is(PropId::Z, "auto"), Value::Nothing);
    assert_eq!(given(PropId::Blend, "fade").map_err(|wrong| wrong.code), Err(codes::VALUE_TYPE));
    assert_eq!(Value::Number(2.0).plus(Value::Number(3.0)), Some(Value::Number(5.0)));
    assert_eq!(Value::Point(Vec2::ONE).times(2.0), Some(Value::Point(Vec2::splat(2.0))));
    assert_eq!(Value::Colour([250, 0, 0, 255]).plus(Value::Colour([10, 0, 0, 0])), Some(Value::Colour([255, 0, 0, 255])));
}

#[test]
fn what_is_given_is_read_back_as_written() {
    for (prop, given) in [
        (PropId::Angle, Given::Is(Value::Number(-12.5))),
        (PropId::Angle, Given::Plus(Value::Number(-30.0))),
        (PropId::Scale, Given::Times(1.5)),
        (PropId::Scale, Given::Default),
        (PropId::Position, Given::Is(Value::Point(Vec2::new(3.0, -4.5)))),
        (PropId::Position, Given::Plus(Value::Point(Vec2::new(3.0, -4.5)))),
        (PropId::Position, Given::At { node: "m25".to_string(), by: Vec2::ZERO }),
        (PropId::Position, Given::At { node: "m25".to_string(), by: Vec2::new(-10.0, 4.0) }),
        (PropId::Tint, Given::Is(Value::Colour([1, 2, 3, 4]))),
        (PropId::Fill, Given::Is(Value::Nothing)),
        (PropId::Blend, Given::Is(Value::Choice("screen"))),
        (PropId::Visible, Given::Is(Value::Bool(false))),
    ] {
        let written = words(&given.to_string());
        assert_eq!(read(prop.prop(), &written), Ok((given, written.len())), "{written:?}");
    }
}

#[test]
fn impossible_values_are_errors_and_strange_ones_remarks() {
    let scale = PropId::Scale.prop();
    assert_eq!(scale.check(&Value::Number(-1.0)).map_err(|wrong| wrong.code), Err(codes::VALUE_IMPOSSIBLE));
    assert_eq!(scale.check(&Value::Number(f32::NAN)).map_err(|wrong| wrong.code), Err(codes::VALUE_IMPOSSIBLE));
    assert_eq!(scale.check(&Value::Number(20.0)).map(|remark| remark.map(|remark| remark.code)), Ok(Some(codes::OUT_OF_RANGE)));
    assert_eq!(scale.check(&Value::Number(2.0)), Ok(None));
    assert_eq!(PropId::Slowmo.prop().check(&Value::Number(0.0)).map(|remark| remark.map(|remark| remark.code)), Ok(Some(codes::OUT_OF_RANGE)));
    // (No floor: anything finite.)
    assert_eq!(PropId::Angle.prop().check(&Value::Number(-7200.0)), Ok(None));
    assert_eq!(PropId::Position.prop().check(&Value::Point(Vec2::new(f32::INFINITY, 0.0))).map_err(|wrong| wrong.code), Err(codes::VALUE_IMPOSSIBLE));
}

#[test]
fn blends_by_name() {
    for name in Blend::NAMES {
        assert_eq!(Blend::named(name).map(Blend::name), Some(*name));
    }
    assert_eq!(Blend::default(), Blend::Normal);
}

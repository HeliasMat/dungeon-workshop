//! What the language is, for a program: every kind of node and property of
//! the registry (`properties`), every word with its forms and the value
//! grammar (`schema`). Both are read from where they are kept (the
//! registry, the film's words, the grammar, the engine's forms), never listed
//! again here.

use engine::props::{self, NodeKind, Prop, Type, Unit};

use super::json::{self, Value, json};

fn unit_name(unit: Unit) -> &'static str {
    match unit {
        Unit::None => "none",
        Unit::Px => "px",
        Unit::Deg => "deg",
        Unit::Factor => "factor",
        Unit::Ratio => "ratio",
        Unit::PerSecond => "per_second",
        Unit::Frames => "frames",
    }
}

const UNITS: [Unit; 7] = [Unit::None, Unit::Px, Unit::Deg, Unit::Factor, Unit::Ratio, Unit::PerSecond, Unit::Frames];

/// A property of the registry, for a program.
pub fn prop_json(prop: &Prop) -> Value {
    let choices = match prop.ty {
        Type::Choice(words) => Some(words),
        _ => None,
    };
    json!({
        "name": prop.name,
        "type": prop.ty.name(),
        "choices": choices,
        "unit": unit_name(prop.unit),
        "unit_written": prop.unit.written(),
        "default": json::prop_value(&prop.default),
        "default_written": prop.default.to_string(),
        "advised": prop.advised.map(|(least, most)| json!([json::number(least), json::number(most)])),
        "floor": prop.floor.map(json::number),
        "animatable": prop.animatable,
        "kinds": prop.kinds.iter().map(NodeKind::name).collect::<Vec<&str>>(),
        "doc": prop.doc,
    })
}

/// Every kind of node (or that one) and every property of the registry.
pub fn properties_value(kind: Option<&str>) -> Result<Value, String> {
    let kinds: Vec<NodeKind> = match kind {
        Some(name) => vec![NodeKind::named(name).ok_or_else(|| {
            let all: Vec<&str> = NodeKind::ALL.iter().map(|kind| kind.name()).collect();
            format!("no kind of node \"{name}\": {}", all.join(", "))
        })?],
        None => NodeKind::ALL.to_vec(),
    };
    let sorts: Vec<Value> =
        kinds.iter().map(|kind| json!({"name": kind.name(), "properties": props::of_kind(*kind).map(|prop| prop.name).collect::<Vec<&str>>()})).collect();
    let listed: Vec<Value> = props::PROPS.iter().filter(|prop| kinds.iter().any(|kind| prop.fits(*kind))).map(prop_json).collect();
    Ok(json::answer("properties", json!({"kinds": sorts, "properties": listed})))
}

/// `properties [kind] [--json]`.
pub fn properties(kind: Option<&str>, as_json: bool) -> Result<(), String> {
    let value = properties_value(kind)?;
    if as_json {
        json::print(&value);
        return Ok(());
    }
    for sort in value["kinds"].as_array().into_iter().flatten() {
        let names: Vec<&str> = sort["properties"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
        println!("{}: {}", sort["name"].as_str().unwrap_or(""), names.join(", "));
    }
    println!();
    for prop in value["properties"].as_array().into_iter().flatten() {
        let advised = match (&prop["advised"], &prop["floor"]) {
            (Value::Array(range), _) => format!(", advised {} to {}", range[0], range[1]),
            (_, Value::Number(floor)) => format!(", {floor} at least"),
            _ => String::new(),
        };
        println!(
            "{:<14} {:<8} {} default {}{advised}{}: {}",
            prop["name"].as_str().unwrap_or(""),
            prop["type"].as_str().unwrap_or(""),
            prop["unit_written"].as_str().unwrap_or(""),
            prop["default_written"].as_str().unwrap_or(""),
            if prop["animatable"] == json!(true) { ", animatable" } else { "" },
            prop["doc"].as_str().unwrap_or("")
        );
    }
    Ok(())
}

/// The forms the engine declares for each of its commands (`engine::forms`),
/// each written with its name first: `musicspeed percent:word`.
fn engine_forms() -> std::collections::BTreeMap<String, Vec<String>> {
    engine::forms()
        .map(|form| (form.own.to_string(), form.forms.iter().map(|written| format!("{} {written}", form.own).trim_end().to_string()).collect()))
        .collect()
}

/// The words of the language: each film word with its forms, the engine's
/// commands (their names, their forms), the head lines,
/// the grammar's kinds and the value grammar.
pub fn schema_value() -> Value {
    let words: Vec<Value> = ::film::every_word()
        .map(|(family, word, forms, writes)| {
            let forms: Vec<String> = forms.iter().map(|form| format!("{word} {form}").trim_end().to_string()).collect();
            json!({"word": word, "family": family, "forms": forms, "written": writes})
        })
        .collect();
    let forms = engine_forms();
    let engine: Vec<Value> = engine::commands()
        .iter()
        .map(|own| {
            let written: Vec<&String> = forms.get(*own).into_iter().flatten().collect();
            json!({"name": own, "forms": written})
        })
        .collect();
    let kinds: Vec<Value> = ::film::grammar::Kind::ALL
        .iter()
        .map(|kind| {
            let (name, said) = kind.told();
            json!({"name": name, "says": said})
        })
        .collect();
    let types: Vec<Value> = [Type::Number, Type::Bool, Type::Point, Type::Colour, Type::Choice(&[])]
        .iter()
        .map(|ty| json!({"name": ty.name(), "says": ty.said().split(" : ").next().unwrap_or_default()}))
        .collect();
    let units: Vec<Value> = UNITS.iter().map(|unit| json!({"name": unit_name(*unit), "written": unit.written()})).collect();
    json::answer(
        "schema",
        json!({
            "header": ["title: <text>", "transition: <word>", "<name> = <species>", "scene: <place> <who...>", "moment: <name>"],
            "reserves": language::line::RESERVED,
            "words": words,
            "engine": engine,
            "genres": kinds,
            "values": {
                "types": types,
                "units": units,
                "durations": ["2s", "1.5s", "250ms", "30f"],
                "distances": ["12", "12px"],
                "angles": ["45", "45deg", "0.5turn"],
                "factors": ["1.5", "150%"],
                "parts": ["0.5", "50%"],
                "per_second": ["5", "5/s"],
                "colours": ["#f80", "#ff8800", "#ff880080", "red", "rgb(255,136,0)", "rgba(255,136,0,0.5)"],
                "points": ["x y", "x,y", "@a", "@a+10,-4"],
                "relative": ["+30", "+-30", "+10,-4", "x1.5"],
                "words": ["default", "none", "on", "off"],
                "merges": engine::props::Blend::NAMES,
            },
            "codes": super::server::codes::every().iter().map(|(code, meaning)| json!({"code": code, "says": meaning})).collect::<Vec<Value>>(),
        }),
    )
}

/// `schema --json` (and without: a short summary).
pub fn schema(as_json: bool) -> Result<(), String> {
    let value = schema_value();
    if as_json {
        json::print(&value);
        return Ok(());
    }
    let count = |field: &str| value[field].as_array().map_or(0, Vec::len);
    println!("{} film words, {} engine commands, {} genres, {} codes. --json: everything.", count("words"), count("engine"), count("genres"), count("codes"));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_property_of_the_registry_is_told() {
        let value = properties_value(None).expect("all");
        assert_eq!(value["properties"].as_array().map(Vec::len), Some(props::PROPS.len()));
        assert_eq!(value["kinds"].as_array().map(Vec::len), Some(NodeKind::ALL.len()));
        let halo = properties_value(Some("halo")).expect("halos");
        assert!(halo["properties"].as_array().expect("a list").iter().any(|prop| prop["name"] == json!("radius")));
        assert!(!halo["properties"].as_array().expect("a list").iter().any(|prop| prop["name"] == json!("height")));
        assert!(properties_value(Some("whatever")).is_err());
        let scale = prop_json(props::PropId::Scale.prop());
        assert_eq!((scale["default"].clone(), scale["advised"].clone(), scale["animatable"].clone()), (json!(1.0), json!([0.05, 8.0]), json!(true)));
    }

    #[test]
    fn the_schema_has_every_word_and_the_engine_s_forms() {
        let value = schema_value();
        let words = value["words"].as_array().expect("words");
        assert!(words.iter().any(|word| word["word"] == json!("prop") && word["forms"].as_array().is_some_and(|forms| !forms.is_empty())));
        let engine = value["engine"].as_array().expect("engine");
        assert!(engine.iter().any(|command| command["name"] == json!("put") && command["forms"].as_array().is_some_and(|forms| !forms.is_empty())));
        assert_eq!(value["genres"].as_array().map(Vec::len), Some(::film::grammar::Kind::ALL.len()));
        assert!(value["codes"].as_array().expect("codes").iter().any(|code| code["code"] == json!("E-READ")));
    }
}

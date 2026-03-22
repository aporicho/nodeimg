mod helpers;

use std::collections::HashMap;
use nodeimg_processing::test_helpers::make_test_image;
use nodeimg_types::value::Value;

#[test]
fn test_histogram_default_channel() {
    let img = make_test_image(64, 64, 128, 128, 128, 255);
    let result = helpers::run_node_test(
        "histogram",
        HashMap::new(),
        HashMap::from([("image".into(), img)]),
        None,
    );
    assert!(
        result.contains_key("image"),
        "histogram should output 'image', got keys: {:?}",
        result.keys().collect::<Vec<_>>()
    );
    match result.get("image").unwrap() {
        Value::Image(img) => {
            assert!(img.width() > 0 && img.height() > 0, "histogram image should have non-zero dimensions");
        }
        other => panic!("expected Value::Image, got {:?}", other),
    }
}

#[test]
fn test_histogram_red_channel() {
    let img = make_test_image(64, 64, 255, 0, 0, 255);
    let result = helpers::run_node_test(
        "histogram",
        HashMap::from([("channel".into(), Value::String("red".into()))]),
        HashMap::from([("image".into(), img)]),
        None,
    );
    assert!(
        result.contains_key("image"),
        "histogram (red channel) should output 'image', got keys: {:?}",
        result.keys().collect::<Vec<_>>()
    );
    match result.get("image").unwrap() {
        Value::Image(_) => {}
        other => panic!("expected Value::Image, got {:?}", other),
    }
}

#[test]
fn test_preview() {
    let img = make_test_image(32, 32, 200, 100, 50, 255);
    let result = helpers::run_node_test(
        "preview",
        HashMap::new(),
        HashMap::from([("image".into(), img)]),
        None,
    );
    assert!(
        result.contains_key("image"),
        "preview should output 'image', got keys: {:?}",
        result.keys().collect::<Vec<_>>()
    );
    match result.get("image").unwrap() {
        Value::Image(out_img) => {
            assert_eq!(out_img.width(), 32, "preview should preserve width");
            assert_eq!(out_img.height(), 32, "preview should preserve height");
        }
        other => panic!("expected Value::Image, got {:?}", other),
    }
}

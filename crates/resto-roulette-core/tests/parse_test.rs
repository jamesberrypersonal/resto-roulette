use resto_roulette_core::parse::parse_file;
use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

#[test]
fn geojson_parses_five_restaurants() {
    let restaurants = parse_file(&fixtures_dir().join("sample.geojson")).unwrap();
    assert_eq!(restaurants.len(), 5);
}

#[test]
fn geojson_first_restaurant_name() {
    let restaurants = parse_file(&fixtures_dir().join("sample.geojson")).unwrap();
    assert_eq!(restaurants[0].name, "Hà");
}

#[test]
fn geojson_coordinates_are_lat_lng_order() {
    let restaurants = parse_file(&fixtures_dir().join("sample.geojson")).unwrap();
    let ha = restaurants.iter().find(|r| r.name == "Hà").unwrap();
    let loc = ha.location.unwrap();
    // lat should be ~45.5, lng should be ~-73.5
    assert!((loc.lat - 45.509).abs() < 0.01, "lat={}", loc.lat);
    assert!((loc.lng - (-73.5605)).abs() < 0.01, "lng={}", loc.lng);
}

#[test]
fn geojson_null_geometry_yields_none_location() {
    let restaurants = parse_file(&fixtures_dir().join("sample.geojson")).unwrap();
    let r = restaurants
        .iter()
        .find(|r| r.name == "Restaurant Sans Adresse")
        .unwrap();
    assert!(r.location.is_none());
}

#[test]
fn csv_parses_five_restaurants() {
    let restaurants = parse_file(&fixtures_dir().join("sample.csv")).unwrap();
    assert_eq!(restaurants.len(), 5);
}

#[test]
fn csv_all_locations_are_none() {
    let restaurants = parse_file(&fixtures_dir().join("sample.csv")).unwrap();
    assert!(restaurants.iter().all(|r| r.location.is_none()));
}

#[test]
fn csv_quoted_address_with_comma() {
    let restaurants = parse_file(&fixtures_dir().join("sample.csv")).unwrap();
    let r = restaurants
        .iter()
        .find(|r| r.name.contains("Cabane"))
        .unwrap();
    assert!(r.address.contains("Fresnière"), "address={}", r.address);
}

#[test]
fn unsupported_extension_returns_error() {
    let result = parse_file(&fixtures_dir().join("sample.txt"));
    assert!(result.is_err());
}

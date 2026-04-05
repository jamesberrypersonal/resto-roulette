/// Maps a Google Places type to a normalized lowercase display name.
/// Returns `None` for non-cuisine types (e.g. `"point_of_interest"`).
pub fn display_name(google_type: &str) -> Option<&'static str> {
    match google_type {
        // Cafes & coffee
        "cafe" | "coffee_shop" | "coffee_roastery" => Some("cafe"),
        "tea_house" => Some("tea"),

        // Bakeries & pastry
        "bakery" | "pastry_shop" | "cake_shop" => Some("bakery"),
        "confectionery" => Some("confectionery"),
        "dessert_shop" | "dessert_restaurant" | "ice_cream_shop" => Some("desserts"),

        // Bars & pubs
        "brewpub" | "brewery" | "beer_garden" => Some("brewpub"),
        "pub" => Some("pub"),
        "wine_bar" => Some("wine bar"),

        // American / casual
        "american_restaurant" | "bar_and_grill" => Some("american"),
        "diner" => Some("diner"),
        "fast_food_restaurant" => Some("fast food"),
        "sandwich_shop" | "deli" => Some("deli"),
        "snack_bar" => Some("snack bar"),
        "chicken_wings_restaurant" => Some("wings"),

        // Mexican & Tex-Mex
        "mexican_restaurant" | "taco_restaurant" => Some("mexican"),
        "tex_mex_restaurant" => Some("tex-mex"),

        // South American
        "latin_american_restaurant" | "south_american_restaurant" => Some("latin american"),
        "argentinian_restaurant" => Some("argentinian"),
        "peruvian_restaurant" => Some("peruvian"),

        // European
        "french_restaurant" | "bistro" => Some("french"),
        "italian_restaurant" => Some("italian"),
        "spanish_restaurant" | "tapas_restaurant" => Some("spanish"),
        "portuguese_restaurant" => Some("portuguese"),
        "greek_restaurant" => Some("greek"),
        "turkish_restaurant" => Some("turkish"),
        "mediterranean_restaurant" => Some("mediterranean"),
        "austrian_restaurant" | "european_restaurant" => Some("european"),
        "eastern_european_restaurant" | "british_restaurant" => Some("european"),

        // Middle Eastern
        "lebanese_restaurant" => Some("lebanese"),
        "middle_eastern_restaurant" | "persian_restaurant" | "halal_restaurant" => {
            Some("middle eastern")
        }

        // South & Southeast Asian
        "indian_restaurant" | "south_indian_restaurant" | "bangladeshi_restaurant" => {
            Some("indian")
        }
        "thai_restaurant" => Some("thai"),
        "vietnamese_restaurant" => Some("vietnamese"),
        "cambodian_restaurant" => Some("cambodian"),

        // East Asian
        "japanese_restaurant" => Some("japanese"),
        "sushi_restaurant" => Some("sushi"),
        "ramen_restaurant" => Some("ramen"),
        "chinese_restaurant" | "cantonese_restaurant" => Some("chinese"),
        "korean_restaurant" => Some("korean"),
        "dim_sum_restaurant" | "dumpling_restaurant" | "noodle_shop" | "hot_pot_restaurant" => {
            Some("chinese")
        }

        // Pan-Asian / fusion
        "asian_restaurant" => Some("asian"),
        "asian_fusion_restaurant" | "fusion_restaurant" => Some("fusion"),

        // Other
        "seafood_restaurant" => Some("seafood"),
        "steak_house" => Some("steakhouse"),
        "pizza_restaurant" => Some("pizza"),
        "hamburger_restaurant" => Some("burger"),
        "barbecue_restaurant" => Some("barbecue"),
        "brunch_restaurant" | "breakfast_restaurant" => Some("brunch"),
        "vegan_restaurant" => Some("vegan"),
        "vegetarian_restaurant" => Some("vegetarian"),

        _ => None,
    }
}

/// Extract all recognized cuisine names from a list of Google Places types.
/// Returns normalized lowercase display names (e.g. `["vietnamese"]`).
/// Non-cuisine types are silently ignored.
pub fn extract_cuisines(types: &[String]) -> Vec<String> {
    types
        .iter()
        .filter_map(|t| display_name(t))
        .map(|s| s.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_type_maps_to_display_name() {
        assert_eq!(display_name("japanese_restaurant"), Some("japanese"));
        assert_eq!(display_name("vietnamese_restaurant"), Some("vietnamese"));
        assert_eq!(display_name("fast_food_restaurant"), Some("fast food"));
        assert_eq!(display_name("cafe"), Some("cafe"));
    }

    #[test]
    fn unknown_type_returns_none() {
        assert_eq!(display_name("point_of_interest"), None);
        assert_eq!(display_name("establishment"), None);
        assert_eq!(display_name("restaurant"), None);
        assert_eq!(display_name("food"), None);
    }

    #[test]
    fn multiple_types_extract_multiple_cuisines() {
        let types = vec![
            "japanese_restaurant".to_string(),
            "sushi_restaurant".to_string(),
        ];
        let result = extract_cuisines(&types);
        assert_eq!(result, vec!["japanese", "sushi"]);
    }

    #[test]
    fn non_cuisine_types_are_filtered_out() {
        let types = vec![
            "vietnamese_restaurant".to_string(),
            "point_of_interest".to_string(),
            "establishment".to_string(),
            "restaurant".to_string(),
        ];
        let result = extract_cuisines(&types);
        assert_eq!(result, vec!["vietnamese"]);
    }

    #[test]
    fn empty_types_returns_empty() {
        assert!(extract_cuisines(&[]).is_empty());
    }

    #[test]
    fn all_non_cuisine_types_returns_empty() {
        let types = vec!["point_of_interest".to_string(), "food".to_string()];
        assert!(extract_cuisines(&types).is_empty());
    }
}

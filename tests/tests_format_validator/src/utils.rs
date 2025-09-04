use convert_case::{Case, Casing};

/// Convert string to snake_case
pub fn to_snake_case(s: &str) -> String {
    s.to_case(Case::Snake)
}

/// Convert string to PascalCase
pub fn to_pascal_case(s: &str) -> String {
    s.to_case(Case::Pascal)
}

/// Check if two strings match when normalized (ignoring case, spaces, underscores)
pub fn strings_match_normalized(s1: &str, s2: &str) -> bool {
    let normalize = |s: &str| {
        s.to_lowercase()
            .replace(" ", "")
            .replace("_", "")
            .replace("-", "")
    };

    normalize(s1) == normalize(s2)
}

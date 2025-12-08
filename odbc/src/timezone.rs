pub fn normalize_timezone_name(tz: &str) -> String {
    let trimmed = tz.trim();
    if trimmed.is_empty() {
        return trimmed.to_string();
    }

    let upper = trimmed.to_ascii_uppercase();
    let normalized = match upper.as_str() {
        "UTC" => "UTC",
        "GMT" => "GMT",
        "EST" | "EDT" => "America/New_York",
        "CST" | "CDT" => "America/Chicago",
        "MST" | "MDT" => "America/Denver",
        "PST" | "PDT" => "America/Los_Angeles",
        "AKST" | "AKDT" => "America/Anchorage",
        "HST" | "HAST" | "HADT" => "Pacific/Honolulu",
        _ => trimmed,
    };

    normalized.to_string()
}

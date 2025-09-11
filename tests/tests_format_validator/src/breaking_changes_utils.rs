use std::collections::HashMap;

/// Parse Breaking Changes descriptions from markdown content.
///
/// Looks for headers like "## 1: Title" and collects both the title
/// and any additional detail lines that follow until the next header.
pub fn parse_breaking_changes_descriptions(
    content: &str,
) -> Result<HashMap<String, String>, anyhow::Error> {
    let mut descriptions = HashMap::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        // Look for markdown headers like "## 1: Description"
        if line.starts_with("## ") && line.contains(": ") {
            if let Some(colon_pos) = line.find(": ") {
                let number_part = line[3..colon_pos].trim(); // Skip "## " and get number
                let title_part = line[colon_pos + 2..].trim(); // Get title after ": "

                if !number_part.is_empty() && !title_part.is_empty() {
                    let breaking_change_id = format!("BC#{number_part}");

                    // Collect the title and any additional details that follow
                    let mut full_description = title_part.to_string();

                    // Look ahead for additional detail lines
                    let mut j = i + 1;
                    while j < lines.len() {
                        let detail_line = lines[j].trim();

                        // Stop if we hit another header
                        if detail_line.starts_with("## ") || detail_line.starts_with("# ") {
                            break;
                        }

                        // Skip completely empty lines
                        if detail_line.is_empty() {
                            j += 1;
                            continue;
                        }

                        // Add detail line with proper spacing
                        if !full_description.is_empty() {
                            full_description.push_str("\n");
                        }
                        full_description.push_str(detail_line);

                        j += 1;
                    }

                    descriptions.insert(breaking_change_id, full_description);
                    i = j - 1; // Skip processed lines
                }
            }
        }
        i += 1;
    }

    Ok(descriptions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_breaking_changes() {
        let content = r#"# Breaking Changes

## 1: Simple Breaking Change title

## 2: Another Breaking Change title
"#;

        let result = parse_breaking_changes_descriptions(content).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(
            result.get("BC#1"),
            Some(&"Simple Breaking Change title".to_string())
        );
        assert_eq!(
            result.get("BC#2"),
            Some(&"Another Breaking Change title".to_string())
        );
    }

    #[test]
    fn test_parse_breaking_changes_with_details() {
        let content = r#"# Breaking Changes

## 1: Breaking Change with details
Some additional details here
More details on another line

## 2: Simple Breaking Change
"#;

        let result = parse_breaking_changes_descriptions(content).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(
            result.get("BC#1"),
            Some(
                &"Breaking Change with details\nSome additional details here\nMore details on another line"
                    .to_string()
            )
        );
        assert_eq!(
            result.get("BC#2"),
            Some(&"Simple Breaking Change".to_string())
        );
    }

    #[test]
    fn test_parse_breaking_changes_with_empty_lines() {
        let content = r#"# Breaking Changes

## 1: Breaking Change title
Some details

More details after empty line

## 2: Another Breaking Change
"#;

        let result = parse_breaking_changes_descriptions(content).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(
            result.get("BC#1"),
            Some(&"Breaking Change title\nSome details\nMore details after empty line".to_string())
        );
    }
}

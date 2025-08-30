use anyhow::Result;
use jiff::Timestamp;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

#[derive(Debug)]
pub struct ConfigMigration {
    pub backup_path: Option<PathBuf>,
    pub changes_made: Vec<String>,
}

impl ConfigMigration {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            backup_path: None,
            changes_made: Vec::new(),
        }
    }

    /// Migrate a configuration file, handling deprecated fields
    pub fn migrate_config_file(path: &Path) -> Result<Option<Self>> {
        let content = fs::read_to_string(path)?;

        // Check if migration is needed
        if !Self::needs_migration(&content) {
            return Ok(None);
        }

        let mut migration = Self::new();

        // Create backup
        migration.backup_path = Some(Self::create_backup(path)?);
        info!("Created config backup at: {:?}", migration.backup_path);

        // Perform migration based on file type
        let migrated_content = if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            Self::migrate_toml_content(&content, &mut migration)
        } else if path.extension().and_then(|s| s.to_str()) == Some("jsonc")
            || path.extension().and_then(|s| s.to_str()) == Some("json")
        {
            Self::migrate_json_content(&content, &mut migration)
        } else {
            return Ok(None);
        };

        // Write migrated content back
        fs::write(path, migrated_content)?;

        // Log migration summary
        for change in &migration.changes_made {
            info!("Config migration: {}", change);
        }

        Ok(Some(migration))
    }

    /// Check if a config file needs migration
    fn needs_migration(content: &str) -> bool {
        content.contains("scroll_down_many")
            || content.contains("scroll_up_many")
            || content.contains("scroll_down_one")
            || content.contains("scroll_up_one")
    }

    /// Create a backup of the config file
    fn create_backup(path: &Path) -> Result<PathBuf> {
        let timestamp = Timestamp::now().to_string().replace([':', '.'], "-");

        let backup_name = format!(
            "{}.backup.{}",
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("config"),
            timestamp
        );

        let backup_path = path.parent().map_or_else(
            || PathBuf::from(backup_name.clone()),
            |p| p.join(&backup_name),
        );

        fs::copy(path, &backup_path)?;
        Ok(backup_path)
    }

    /// Migrate TOML content
    fn migrate_toml_content(content: &str, migration: &mut Self) -> String {
        let mut lines: Vec<String> = content
            .lines()
            .map(std::string::ToString::to_string)
            .collect();
        let mut i = 0;

        while i < lines.len() {
            let line = &lines[i].clone();

            // Check for deprecated fields
            if line.trim_start().starts_with("scroll_down_many") {
                // Extract the value
                if let Some(value) = Self::extract_toml_value(line) {
                    // Comment out the old line
                    lines[i] = format!(
                        "# {line} # Deprecated - use scroll_many modifier with scroll_down"
                    );
                    migration.changes_made.push(format!("Deprecated scroll_down_many = {value} (use scroll_many modifier with scroll_down)"));
                }
            } else if line.trim_start().starts_with("scroll_up_many") {
                // Extract the value
                if let Some(value) = Self::extract_toml_value(line) {
                    // Comment out the old line
                    lines[i] =
                        format!("# {line} # Deprecated - use scroll_many modifier with scroll_up");
                    migration.changes_made.push(format!(
                        "Deprecated scroll_up_many = {value} (use scroll_many modifier with scroll_up)"
                    ));
                }
            } else if line.trim_start().starts_with("scroll_down_one") {
                // Rename to scroll_down
                if let Some(value) = Self::extract_toml_value(line) {
                    lines[i] = format!("scroll_down = {value}");
                    migration
                        .changes_made
                        .push("Renamed scroll_down_one to scroll_down".to_string());
                }
            } else if line.trim_start().starts_with("scroll_up_one") {
                // Rename to scroll_up
                if let Some(value) = Self::extract_toml_value(line) {
                    lines[i] = format!("scroll_up = {value}");
                    migration
                        .changes_made
                        .push("Renamed scroll_up_one to scroll_up".to_string());
                }
            }

            i += 1;
        }

        // Add scroll_many if not present and we had scroll_*_many fields
        if migration
            .changes_made
            .iter()
            .any(|c| c.contains("scroll_down_many") || c.contains("scroll_up_many"))
        {
            let mut has_scroll_many = false;
            for line in &lines {
                if line.trim_start().starts_with("scroll_many") {
                    has_scroll_many = true;
                    break;
                }
            }

            if !has_scroll_many {
                // Find the keymap section and add scroll_many
                for (i, line) in lines.iter().enumerate() {
                    if line.contains("[keymap]") {
                        // Insert scroll_many after the [keymap] section
                        lines.insert(i + 1, "scroll_many = [\"control\"]  # Use with scroll_up/scroll_down for page scrolling".to_string());
                        migration.changes_made.push(
                            "Added scroll_many = [\"control\"] for page scrolling".to_string(),
                        );
                        break;
                    }
                }
            }
        }

        lines.join("\n")
    }

    /// Migrate JSON/JSONC content
    fn migrate_json_content(content: &str, migration: &mut Self) -> String {
        // Parse as JSON value to preserve structure
        let mut lines: Vec<String> = content
            .lines()
            .map(std::string::ToString::to_string)
            .collect();
        let mut i = 0;

        while i < lines.len() {
            let line = &lines[i].clone();

            // Check for deprecated fields in JSON format
            if line.contains("\"scroll_down_many\"") {
                // Comment out the line
                lines[i] = format!(
                    "    // {} // Deprecated - use scroll_many modifier",
                    line.trim()
                );
                migration
                    .changes_made
                    .push("Deprecated scroll_down_many field".to_string());
            } else if line.contains("\"scroll_up_many\"") {
                // Comment out the line
                lines[i] = format!(
                    "    // {} // Deprecated - use scroll_many modifier",
                    line.trim()
                );
                migration
                    .changes_made
                    .push("Deprecated scroll_up_many field".to_string());
            } else if line.contains("\"scroll_down_one\"") {
                // Rename to scroll_down
                lines[i] = line.replace("\"scroll_down_one\"", "\"scroll_down\"");
                migration
                    .changes_made
                    .push("Renamed scroll_down_one to scroll_down".to_string());
            } else if line.contains("\"scroll_up_one\"") {
                // Rename to scroll_up
                lines[i] = line.replace("\"scroll_up_one\"", "\"scroll_up\"");
                migration
                    .changes_made
                    .push("Renamed scroll_up_one to scroll_up".to_string());
            }

            i += 1;
        }

        // Add scroll_many if needed
        if migration
            .changes_made
            .iter()
            .any(|c| c.contains("scroll_down_many") || c.contains("scroll_up_many"))
        {
            let mut has_scroll_many = false;
            let mut keymap_end_index = None;

            for (i, line) in lines.iter().enumerate() {
                if line.contains("\"scroll_many\"") {
                    has_scroll_many = true;
                    break;
                }
                if line.contains("\"keymap\"") {
                    // Find the closing brace of keymap section
                    let mut brace_count = 0;
                    for (j, l) in lines.iter().enumerate().skip(i) {
                        if l.contains('{') {
                            brace_count += l.matches('{').count();
                        }
                        if l.contains('}') {
                            brace_count -= l.matches('}').count();
                            if brace_count == 0 {
                                keymap_end_index = Some(j);
                                break;
                            }
                        }
                    }
                }
            }

            if !has_scroll_many && let Some(end_idx) = keymap_end_index {
                // Insert scroll_many before the closing brace
                lines.insert(end_idx, "    \"scroll_many\": [\"control\"]  // Use with scroll_up/scroll_down for page scrolling".to_string());
                migration
                    .changes_made
                    .push("Added scroll_many = [\"control\"] for page scrolling".to_string());
            }
        }

        lines.join("\n")
    }

    /// Extract value from a TOML line (e.g., "key = value" -> "value")
    fn extract_toml_value(line: &str) -> Option<String> {
        line.find('=')
            .map(|eq_pos| line[eq_pos + 1..].trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_needs_migration() {
        assert!(ConfigMigration::needs_migration(
            "scroll_down_many = [\"PageDown\"]"
        ));
        assert!(ConfigMigration::needs_migration(
            "scroll_up_many = [\"PageUp\"]"
        ));
        assert!(ConfigMigration::needs_migration(
            "scroll_down_one = [\"Down\"]"
        ));
        assert!(ConfigMigration::needs_migration("scroll_up_one = [\"Up\"]"));
        assert!(!ConfigMigration::needs_migration(
            "scroll_down = [\"Down\"]"
        ));
        assert!(!ConfigMigration::needs_migration("scroll_up = [\"Up\"]"));
    }

    #[test]
    fn test_migrate_toml_content() {
        let content = r#"[keymap]
scroll_down_one = ["Down", "j"]
scroll_up_one = ["Up", "k"]
scroll_down_many = ["PageDown"]
scroll_up_many = ["PageUp"]
quit = ["q"]"#;

        let mut migration = ConfigMigration::new();
        let result = ConfigMigration::migrate_toml_content(content, &mut migration);

        assert!(result.contains("scroll_down = [\"Down\", \"j\"]"));
        assert!(result.contains("scroll_up = [\"Up\", \"k\"]"));
        assert!(result.contains("# scroll_down_many"));
        assert!(result.contains("# scroll_up_many"));
        assert!(result.contains("scroll_many = [\"control\"]"));
        assert_eq!(migration.changes_made.len(), 5); // 4 field changes + 1 scroll_many addition
    }

    #[test]
    fn test_migrate_json_content() {
        let content = r#"{
  "keymap": {
    "scroll_down_one": ["Down", "j"],
    "scroll_up_one": ["Up", "k"],
    "scroll_down_many": ["PageDown"],
    "scroll_up_many": ["PageUp"]
  }
}"#;

        let mut migration = ConfigMigration::new();
        let result = ConfigMigration::migrate_json_content(content, &mut migration);

        assert!(result.contains("\"scroll_down\""));
        assert!(result.contains("\"scroll_up\""));
        assert!(result.contains("// Deprecated"));
        assert_eq!(migration.changes_made.len(), 5); // 4 field changes + 1 scroll_many addition
    }
}

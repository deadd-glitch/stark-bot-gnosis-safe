use crate::skills::loader::load_skills_from_directory;
use crate::skills::types::{Skill, SkillSource};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;

/// Registry that holds all available skills
pub struct SkillRegistry {
    skills: RwLock<HashMap<String, Skill>>,
    /// Paths to skill directories
    bundled_path: Option<PathBuf>,
    managed_path: Option<PathBuf>,
    workspace_path: Option<PathBuf>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        SkillRegistry {
            skills: RwLock::new(HashMap::new()),
            bundled_path: None,
            managed_path: None,
            workspace_path: None,
        }
    }

    /// Create a registry with configured paths
    pub fn with_paths(
        bundled_path: Option<PathBuf>,
        managed_path: Option<PathBuf>,
        workspace_path: Option<PathBuf>,
    ) -> Self {
        SkillRegistry {
            skills: RwLock::new(HashMap::new()),
            bundled_path,
            managed_path,
            workspace_path,
        }
    }

    /// Register a skill (higher priority sources override lower priority)
    pub fn register(&self, skill: Skill) {
        let mut skills = self.skills.write().unwrap();
        let name = skill.metadata.name.clone();

        if let Some(existing) = skills.get(&name) {
            // Only replace if new skill has higher priority
            if skill.source.priority() >= existing.source.priority() {
                log::info!(
                    "Skill '{}' from {:?} overrides {:?} version",
                    name,
                    skill.source,
                    existing.source
                );
                skills.insert(name, skill);
            }
        } else {
            skills.insert(name, skill);
        }
    }

    /// Get a skill by name
    pub fn get(&self, name: &str) -> Option<Skill> {
        self.skills.read().unwrap().get(name).cloned()
    }

    /// List all registered skills
    pub fn list(&self) -> Vec<Skill> {
        self.skills.read().unwrap().values().cloned().collect()
    }

    /// List enabled skills
    pub fn list_enabled(&self) -> Vec<Skill> {
        self.skills
            .read()
            .unwrap()
            .values()
            .filter(|s| s.enabled)
            .cloned()
            .collect()
    }

    /// Enable or disable a skill
    pub fn set_enabled(&self, name: &str, enabled: bool) -> bool {
        let mut skills = self.skills.write().unwrap();
        if let Some(skill) = skills.get_mut(name) {
            skill.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// Check if a skill exists
    pub fn has_skill(&self, name: &str) -> bool {
        self.skills.read().unwrap().contains_key(name)
    }

    /// Get count of registered skills
    pub fn len(&self) -> usize {
        self.skills.read().unwrap().len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.skills.read().unwrap().is_empty()
    }

    /// Load skills from all configured paths
    pub async fn load_all(&self) -> Result<usize, String> {
        let mut loaded = 0;

        // Load bundled skills (lowest priority)
        if let Some(ref path) = self.bundled_path {
            match load_skills_from_directory(path, SkillSource::Bundled).await {
                Ok(skills) => {
                    for skill in skills {
                        self.register(skill);
                        loaded += 1;
                    }
                }
                Err(e) => log::warn!("Failed to load bundled skills: {}", e),
            }
        }

        // Load managed skills (medium priority)
        if let Some(ref path) = self.managed_path {
            match load_skills_from_directory(path, SkillSource::Managed).await {
                Ok(skills) => {
                    for skill in skills {
                        self.register(skill);
                        loaded += 1;
                    }
                }
                Err(e) => log::warn!("Failed to load managed skills: {}", e),
            }
        }

        // Load workspace skills (highest priority)
        if let Some(ref path) = self.workspace_path {
            match load_skills_from_directory(path, SkillSource::Workspace).await {
                Ok(skills) => {
                    for skill in skills {
                        self.register(skill);
                        loaded += 1;
                    }
                }
                Err(e) => log::warn!("Failed to load workspace skills: {}", e),
            }
        }

        log::info!("Loaded {} skills total ({} unique)", loaded, self.len());
        Ok(loaded)
    }

    /// Reload all skills from disk
    pub async fn reload(&self) -> Result<usize, String> {
        // Clear existing skills
        self.skills.write().unwrap().clear();
        // Load all again
        self.load_all().await
    }

    /// Get skills that require specific tools
    pub fn get_skills_requiring_tools(&self, tool_names: &[String]) -> Vec<Skill> {
        self.skills
            .read()
            .unwrap()
            .values()
            .filter(|s| {
                s.metadata
                    .requires_tools
                    .iter()
                    .any(|t| tool_names.contains(t))
            })
            .cloned()
            .collect()
    }

    /// Search skills by name or tag
    pub fn search(&self, query: &str) -> Vec<Skill> {
        let query_lower = query.to_lowercase();
        self.skills
            .read()
            .unwrap()
            .values()
            .filter(|s| {
                s.metadata.name.to_lowercase().contains(&query_lower)
                    || s.metadata.description.to_lowercase().contains(&query_lower)
                    || s.metadata
                        .tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect()
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a default skill registry with standard paths
pub fn create_default_registry() -> SkillRegistry {
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    SkillRegistry::with_paths(
        Some(current_dir.join("skills/bundled")),
        Some(current_dir.join("skills/managed")),
        Some(current_dir.join("workspace/.skills")),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::types::SkillMetadata;

    fn create_test_skill(name: &str, source: SkillSource) -> Skill {
        Skill {
            metadata: SkillMetadata {
                name: name.to_string(),
                description: format!("Test skill {}", name),
                ..Default::default()
            },
            prompt_template: "Test prompt".to_string(),
            source,
            path: format!("/test/{}/SKILL.md", name),
            enabled: true,
        }
    }

    #[test]
    fn test_registry_register_and_get() {
        let registry = SkillRegistry::new();
        let skill = create_test_skill("test-skill", SkillSource::Bundled);
        registry.register(skill);

        assert!(registry.has_skill("test-skill"));
        assert!(!registry.has_skill("nonexistent"));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_registry_priority_override() {
        let registry = SkillRegistry::new();

        // Register bundled version
        let bundled = create_test_skill("my-skill", SkillSource::Bundled);
        registry.register(bundled);

        // Register workspace version (should override)
        let workspace = create_test_skill("my-skill", SkillSource::Workspace);
        registry.register(workspace);

        let skill = registry.get("my-skill").unwrap();
        assert_eq!(skill.source, SkillSource::Workspace);
    }

    #[test]
    fn test_registry_enable_disable() {
        let registry = SkillRegistry::new();
        let skill = create_test_skill("test-skill", SkillSource::Bundled);
        registry.register(skill);

        assert!(registry.get("test-skill").unwrap().enabled);

        registry.set_enabled("test-skill", false);
        assert!(!registry.get("test-skill").unwrap().enabled);

        assert_eq!(registry.list_enabled().len(), 0);
    }
}

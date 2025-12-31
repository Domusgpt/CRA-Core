//! Atlas Loader
//!
//! Provides functionality to load atlases from various sources:
//! - JSON files
//! - Directories (atlas package format)
//! - In-memory JSON strings

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{CRAError, Result};

use super::manifest::AtlasManifest;

/// Atlas loader for loading atlases from various sources
pub struct AtlasLoader {
    /// Loaded atlases by ID
    atlases: HashMap<String, LoadedAtlas>,

    /// Search paths for atlas discovery
    search_paths: Vec<PathBuf>,

    /// Whether to validate on load
    validate_on_load: bool,
}

/// A loaded atlas with its source information
#[derive(Debug, Clone)]
pub struct LoadedAtlas {
    /// The atlas manifest
    pub manifest: AtlasManifest,

    /// Source path (if loaded from file)
    pub source_path: Option<PathBuf>,

    /// Context files (if loaded from directory)
    pub context_files: HashMap<String, String>,
}

impl AtlasLoader {
    /// Create a new atlas loader
    pub fn new() -> Self {
        Self {
            atlases: HashMap::new(),
            search_paths: vec![],
            validate_on_load: true,
        }
    }

    /// Add a search path for atlas discovery
    pub fn with_search_path(mut self, path: PathBuf) -> Self {
        self.search_paths.push(path);
        self
    }

    /// Disable validation on load
    pub fn skip_validation(mut self) -> Self {
        self.validate_on_load = false;
        self
    }

    /// Load an atlas from a JSON string
    pub fn load_from_json(&mut self, json: &str) -> Result<String> {
        let manifest: AtlasManifest = serde_json::from_str(json).map_err(|e| {
            CRAError::InvalidAtlasManifest {
                reason: e.to_string(),
            }
        })?;

        if self.validate_on_load {
            manifest.validate().map_err(|errors| {
                CRAError::InvalidAtlasManifest {
                    reason: errors.join("; "),
                }
            })?;
        }

        let atlas_id = manifest.atlas_id.clone();

        self.atlases.insert(
            atlas_id.clone(),
            LoadedAtlas {
                manifest,
                source_path: None,
                context_files: HashMap::new(),
            },
        );

        Ok(atlas_id)
    }

    /// Load an atlas from a JSON file
    pub fn load_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<String> {
        let path = path.as_ref();
        let content = fs::read_to_string(path).map_err(|e| CRAError::AtlasLoadError {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;

        let manifest: AtlasManifest = serde_json::from_str(&content).map_err(|e| {
            CRAError::InvalidAtlasManifest {
                reason: format!("{}: {}", path.display(), e),
            }
        })?;

        if self.validate_on_load {
            manifest.validate().map_err(|errors| {
                CRAError::InvalidAtlasManifest {
                    reason: errors.join("; "),
                }
            })?;
        }

        let atlas_id = manifest.atlas_id.clone();

        self.atlases.insert(
            atlas_id.clone(),
            LoadedAtlas {
                manifest,
                source_path: Some(path.to_path_buf()),
                context_files: HashMap::new(),
            },
        );

        Ok(atlas_id)
    }

    /// Load an atlas from a directory (atlas package format)
    ///
    /// Expected structure:
    /// ```text
    /// atlas-name/
    /// ├── atlas.json          # Manifest (required)
    /// ├── context/            # Context documents
    /// │   └── *.md
    /// └── adapters/           # Platform-specific configs
    ///     └── *.json
    /// ```
    pub fn load_from_directory<P: AsRef<Path>>(&mut self, path: P) -> Result<String> {
        let path = path.as_ref();

        if !path.is_dir() {
            return Err(CRAError::AtlasLoadError {
                path: path.display().to_string(),
                reason: "Not a directory".to_string(),
            });
        }

        // Load manifest
        let manifest_path = path.join("atlas.json");
        if !manifest_path.exists() {
            return Err(CRAError::AtlasLoadError {
                path: path.display().to_string(),
                reason: "atlas.json not found".to_string(),
            });
        }

        let manifest_content = fs::read_to_string(&manifest_path).map_err(|e| {
            CRAError::AtlasLoadError {
                path: manifest_path.display().to_string(),
                reason: e.to_string(),
            }
        })?;

        let manifest: AtlasManifest =
            serde_json::from_str(&manifest_content).map_err(|e| CRAError::InvalidAtlasManifest {
                reason: format!("{}: {}", manifest_path.display(), e),
            })?;

        if self.validate_on_load {
            manifest.validate().map_err(|errors| {
                CRAError::InvalidAtlasManifest {
                    reason: errors.join("; "),
                }
            })?;
        }

        // Load context files
        let mut context_files = HashMap::new();
        let context_dir = path.join("context");
        if context_dir.is_dir() {
            for entry in fs::read_dir(&context_dir).map_err(|e| CRAError::AtlasLoadError {
                path: context_dir.display().to_string(),
                reason: e.to_string(),
            })? {
                let entry = entry.map_err(|e| CRAError::AtlasLoadError {
                    path: context_dir.display().to_string(),
                    reason: e.to_string(),
                })?;
                let file_path = entry.path();
                if file_path.is_file() {
                    if let Some(name) = file_path.file_name() {
                        let content = fs::read_to_string(&file_path).map_err(|e| {
                            CRAError::AtlasLoadError {
                                path: file_path.display().to_string(),
                                reason: e.to_string(),
                            }
                        })?;
                        context_files.insert(name.to_string_lossy().to_string(), content);
                    }
                }
            }
        }

        let atlas_id = manifest.atlas_id.clone();

        self.atlases.insert(
            atlas_id.clone(),
            LoadedAtlas {
                manifest,
                source_path: Some(path.to_path_buf()),
                context_files,
            },
        );

        Ok(atlas_id)
    }

    /// Load an atlas directly from a manifest struct
    pub fn load_from_manifest(&mut self, manifest: AtlasManifest) -> Result<String> {
        if self.validate_on_load {
            manifest.validate().map_err(|errors| {
                CRAError::InvalidAtlasManifest {
                    reason: errors.join("; "),
                }
            })?;
        }

        let atlas_id = manifest.atlas_id.clone();

        self.atlases.insert(
            atlas_id.clone(),
            LoadedAtlas {
                manifest,
                source_path: None,
                context_files: HashMap::new(),
            },
        );

        Ok(atlas_id)
    }

    /// Get a loaded atlas by ID
    pub fn get(&self, atlas_id: &str) -> Option<&LoadedAtlas> {
        self.atlases.get(atlas_id)
    }

    /// Get the manifest for an atlas
    pub fn get_manifest(&self, atlas_id: &str) -> Option<&AtlasManifest> {
        self.atlases.get(atlas_id).map(|a| &a.manifest)
    }

    /// Unload an atlas
    pub fn unload(&mut self, atlas_id: &str) -> Option<LoadedAtlas> {
        self.atlases.remove(atlas_id)
    }

    /// List all loaded atlas IDs
    pub fn list_ids(&self) -> Vec<&str> {
        self.atlases.keys().map(|s| s.as_str()).collect()
    }

    /// Check if an atlas is loaded
    pub fn is_loaded(&self, atlas_id: &str) -> bool {
        self.atlases.contains_key(atlas_id)
    }

    /// Get all loaded atlases
    pub fn all(&self) -> &HashMap<String, LoadedAtlas> {
        &self.atlases
    }

    /// Discover atlases in search paths
    ///
    /// Searches for atlas.json files or directories containing atlas.json
    pub fn discover(&self) -> Vec<PathBuf> {
        let mut found = vec![];

        for search_path in &self.search_paths {
            if !search_path.is_dir() {
                continue;
            }

            // Look for atlas.json files directly
            let direct = search_path.join("atlas.json");
            if direct.exists() {
                found.push(search_path.clone());
                continue;
            }

            // Look in subdirectories
            if let Ok(entries) = fs::read_dir(search_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let manifest = path.join("atlas.json");
                        if manifest.exists() {
                            found.push(path);
                        }
                    }
                }
            }
        }

        found
    }

    /// Load all discovered atlases
    pub fn load_discovered(&mut self) -> Result<Vec<String>> {
        let paths = self.discover();
        let mut loaded = vec![];

        for path in paths {
            match self.load_from_directory(&path) {
                Ok(atlas_id) => loaded.push(atlas_id),
                Err(e) => {
                    // Log but continue
                    eprintln!("Warning: Failed to load atlas from {:?}: {}", path, e);
                }
            }
        }

        Ok(loaded)
    }

    /// Reload an atlas from its source
    pub fn reload(&mut self, atlas_id: &str) -> Result<()> {
        let atlas = self.atlases.get(atlas_id).ok_or_else(|| CRAError::AtlasNotFound {
            atlas_id: atlas_id.to_string(),
        })?;

        let source_path = atlas.source_path.clone().ok_or_else(|| CRAError::AtlasLoadError {
            path: atlas_id.to_string(),
            reason: "No source path available for reload".to_string(),
        })?;

        // Unload first
        self.atlases.remove(atlas_id);

        // Reload from source
        if source_path.is_dir() {
            self.load_from_directory(&source_path)?;
        } else {
            self.load_from_file(&source_path)?;
        }

        Ok(())
    }
}

impl Default for AtlasLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_from_json() {
        let mut loader = AtlasLoader::new();

        let json = r#"{
            "atlas_version": "1.0",
            "atlas_id": "com.test.example",
            "version": "1.0.0",
            "name": "Test Atlas",
            "description": "A test atlas",
            "domains": ["test"],
            "capabilities": [],
            "policies": [],
            "actions": []
        }"#;

        let result = loader.load_from_json(json);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "com.test.example");
        assert!(loader.is_loaded("com.test.example"));
    }

    #[test]
    fn test_load_invalid_json() {
        let mut loader = AtlasLoader::new();

        let result = loader.load_from_json("not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_on_load() {
        let mut loader = AtlasLoader::new();

        // Invalid: missing atlas_id
        let json = r#"{
            "atlas_version": "1.0",
            "atlas_id": "",
            "version": "1.0.0",
            "name": "Test",
            "description": "",
            "domains": [],
            "capabilities": [],
            "policies": [],
            "actions": []
        }"#;

        let result = loader.load_from_json(json);
        assert!(result.is_err());

        // Skip validation
        let mut skip_loader = AtlasLoader::new().skip_validation();
        let result = skip_loader.load_from_json(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_and_unload() {
        let mut loader = AtlasLoader::new();

        loader
            .load_from_json(
                r#"{
            "atlas_version": "1.0",
            "atlas_id": "com.test.one",
            "version": "1.0.0",
            "name": "One",
            "description": "",
            "domains": [],
            "capabilities": [],
            "policies": [],
            "actions": []
        }"#,
            )
            .unwrap();

        loader
            .load_from_json(
                r#"{
            "atlas_version": "1.0",
            "atlas_id": "com.test.two",
            "version": "1.0.0",
            "name": "Two",
            "description": "",
            "domains": [],
            "capabilities": [],
            "policies": [],
            "actions": []
        }"#,
            )
            .unwrap();

        assert_eq!(loader.list_ids().len(), 2);

        loader.unload("com.test.one");
        assert_eq!(loader.list_ids().len(), 1);
        assert!(!loader.is_loaded("com.test.one"));
        assert!(loader.is_loaded("com.test.two"));
    }
}

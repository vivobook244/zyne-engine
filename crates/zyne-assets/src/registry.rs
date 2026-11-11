//! Persistent asset metadata registry.
 //!
 //! The registry owns stable metadata only. It does not load source files,
 //! allocate GPU resources, or create runtime handles.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};
use zyne_core::ResourceId;

const CURRENT_REGISTRY_SCHEMA_VERSION: u32 = 1;

/// Broad resource category stored in an asset registry entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceKind {
    Model,
    Mesh,
    Texture,
    Material,
    Audio,
    Scene,
}

/// A normalized path relative to the project root.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectRelativePath(String);

impl ProjectRelativePath {
    /// Creates a normalized, portable project-relative path.
    ///
    /// Both slash styles are accepted as input and stored using forward slashes.
    ///
    /// # Errors
    ///
    /// Returns an error for empty paths, absolute paths, parent traversal, or
    /// invalid path characters.
    pub fn new(path: impl AsRef<str>) -> Result<Self, RegistryError> {
        let raw = path.as_ref();
        if raw.trim().is_empty() {
            return Err(RegistryError::EmptySourcePath);
        }

        let normalized = raw.replace('\\', "/");
        let bytes = normalized.as_bytes();
        let has_windows_drive_prefix =
            bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
        if normalized.starts_with('/') || has_windows_drive_prefix {
            return Err(RegistryError::AbsoluteSourcePath);
        }

        let mut components = Vec::new();
        for component in normalized.split('/') {
            match component {
                "" | "." => {}
                ".." => return Err(RegistryError::ParentDirectoryTraversal),
                value => {
                    if value.contains(['\0', ':']) {
                        return Err(RegistryError::InvalidSourcePath);
                    }
                    components.push(value);
                }
            }
        }

        if components.is_empty() {
            return Err(RegistryError::EmptySourcePath);
        }

        Ok(Self(components.join("/")))
    }

    /// Returns the normalized project-relative path.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Importer identity and deterministic settings stored with an asset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImporterSettings {
    importer: String,
    version: u32,
    options: BTreeMap<String, String>,
}

impl ImporterSettings {
    /// Creates settings for a named importer and version.
    ///
    /// # Errors
    ///
    /// Returns an error when the importer name is blank or the version is zero.
    pub fn new(importer: impl Into<String>, version: u32) -> Result<Self, RegistryError> {
        let importer = importer.into();
        if importer.trim().is_empty() {
            return Err(RegistryError::EmptyImporterName);
        }
        if version == 0 {
            return Err(RegistryError::InvalidImporterVersion);
        }
        Ok(Self {
            importer: importer.trim().to_owned(),
            version,
            options: BTreeMap::new(),
        })
    }

    /// Adds or replaces a deterministic string option.
    ///
    /// # Errors
    ///
    /// Returns an error when the option key is blank.
    pub fn set_option(
        &mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<(), RegistryError> {
        let key = key.into();
        if key.trim().is_empty() {
            return Err(RegistryError::EmptyImporterOptionKey);
        }
        self.options.insert(key.trim().to_owned(), value.into());
        Ok(())
    }

    #[must_use]
    pub fn importer(&self) -> &str {
        &self.importer
    }

    #[must_use]
    pub const fn version(&self) -> u32 {
        self.version
    }

    #[must_use]
    pub fn options(&self) -> &BTreeMap<String, String> {
        &self.options
    }

    fn validate(&self) -> Result<(), RegistryError> {
        if self.importer.trim().is_empty() {
            return Err(RegistryError::EmptyImporterName);
        }
        if self.version == 0 {
            return Err(RegistryError::InvalidImporterVersion);
        }
        if self.options.keys().any(|key| key.trim().is_empty()) {
            return Err(RegistryError::EmptyImporterOptionKey);
        }
        Ok(())
    }
}

/// Persistent metadata for one source asset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetMetadata {
    id: ResourceId,
    name: String,
    kind: ResourceKind,
    source_path: ProjectRelativePath,
    importer: ImporterSettings,
}

impl AssetMetadata {
    /// Creates validated metadata for an asset.
    ///
    /// # Errors
    ///
    /// Returns an error when the name, source path, or importer settings are invalid.
    pub fn new(
        id: ResourceId,
        name: impl Into<String>,
        kind: ResourceKind,
        source_path: ProjectRelativePath,
        importer: ImporterSettings,
    ) -> Result<Self, RegistryError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(RegistryError::EmptyAssetName);
        }
        importer.validate()?;
        Ok(Self {
            id,
            name: name.trim().to_owned(),
            kind,
            source_path,
            importer,
        })
    }

    #[must_use]
    pub const fn id(&self) -> ResourceId {
        self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn kind(&self) -> ResourceKind {
        self.kind
    }

    #[must_use]
    pub fn source_path(&self) -> &ProjectRelativePath {
        &self.source_path
    }

    #[must_use]
    pub fn importer(&self) -> &ImporterSettings {
        &self.importer
    }

    fn validate(&self) -> Result<(), RegistryError> {
        if self.name.trim().is_empty() {
            return Err(RegistryError::EmptyAssetName);
        }
        let normalized = ProjectRelativePath::new(self.source_path.as_str())?;
        if normalized != self.source_path {
            return Err(RegistryError::InvalidSourcePath);
        }
        self.importer.validate()
    }
}

/// In-memory index of persistent asset metadata.
#[derive(Debug, Default)]
pub struct AssetRegistry {
    by_id: HashMap<ResourceId, AssetMetadata>,
    by_source_path: HashMap<ProjectRelativePath, ResourceId>,
}

impl AssetRegistry {
    /// Creates an empty asset registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    /// Returns metadata by its stable resource identity.
    #[must_use]
    pub fn find(&self, id: ResourceId) -> Option<&AssetMetadata> {
        self.by_id.get(&id)
    }

    /// Returns metadata by its normalized source path.
    #[must_use]
    pub fn find_by_source_path(&self, path: &ProjectRelativePath) -> Option<&AssetMetadata> {
        let id = self.by_source_path.get(path)?;
        self.by_id.get(id)
    }

    /// Returns metadata in deterministic resource-ID order.
    #[must_use]
    pub fn assets(&self) -> Vec<&AssetMetadata> {
        let mut assets: Vec<_> = self.by_id.values().collect();
        assets.sort_unstable_by_key(|asset| *asset.id().as_uuid().as_bytes());
        assets
    }

    /// Registers metadata if its ID and source path are both unclaimed.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid metadata, a duplicate ID, or a source path
    /// already owned by another resource.
    pub fn register(&mut self, metadata: AssetMetadata) -> Result<(), RegistryError> {
        metadata.validate()?;
        if self.by_id.contains_key(&metadata.id) {
            return Err(RegistryError::DuplicateResourceId { id: metadata.id });
        }
        if self.by_source_path.contains_key(&metadata.source_path) {
            return Err(RegistryError::SourcePathInUse {
                path: metadata.source_path,
            });
        }

        self.by_source_path
            .insert(metadata.source_path.clone(), metadata.id);
        self.by_id.insert(metadata.id, metadata);
        Ok(())
    }

    /// Renames an asset without changing its identity or source path.
    ///
    /// # Errors
    ///
    /// Returns an error for a blank name or an unknown resource ID.
    pub fn rename(
        &mut self,
        id: ResourceId,
        name: impl Into<String>,
    ) -> Result<(), RegistryError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(RegistryError::EmptyAssetName);
        }
        let metadata = self
            .by_id
            .get_mut(&id)
            .ok_or(RegistryError::ResourceNotFound { id })?;
        metadata.name = name.trim().to_owned();
        Ok(())
    }

    /// Moves an asset source path without changing its identity.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown resource ID or a path already owned by
    /// another resource.
    pub fn move_source(
        &mut self,
        id: ResourceId,
        new_path: ProjectRelativePath,
    ) -> Result<(), RegistryError> {
        let current_path = self
            .by_id
            .get(&id)
            .ok_or(RegistryError::ResourceNotFound { id })?
            .source_path
            .clone();

        if current_path == new_path {
            return Ok(());
        }
        if self.by_source_path.contains_key(&new_path) {
            return Err(RegistryError::SourcePathInUse { path: new_path });
        }

        self.by_source_path.remove(&current_path);
        self.by_source_path.insert(new_path.clone(), id);
        if let Some(metadata) = self.by_id.get_mut(&id) {
            metadata.source_path = new_path;
        }
        Ok(())
    }

    /// Removes metadata and releases its source-path ownership.
    pub fn remove(&mut self, id: ResourceId) -> Option<AssetMetadata> {
        let metadata = self.by_id.remove(&id)?;
        self.by_source_path.remove(&metadata.source_path);
        Some(metadata)
    }

    /// Serializes registry metadata to a deterministic RON document.
    ///
    /// # Errors
    ///
    /// Returns a RON serialization error.
    pub fn to_ron(&self) -> Result<String, ron::Error> {
        let document = AssetRegistryDocument {
            schema_version: CURRENT_REGISTRY_SCHEMA_VERSION,
            assets: self.assets().into_iter().cloned().collect(),
        };
        ron::ser::to_string_pretty(&document, ron::ser::PrettyConfig::default())
    }

    /// Loads and validates registry metadata from a RON document.
    ///
    /// # Errors
    ///
    /// Returns a RON decoding error, unsupported schema version, invalid
    /// metadata, duplicate resource ID, or conflicting source path.
    pub fn from_ron(source: &str) -> Result<Self, RegistryLoadError> {
        let document: AssetRegistryDocument =
            ron::de::from_str(source).map_err(RegistryLoadError::Ron)?;
        if document.schema_version != CURRENT_REGISTRY_SCHEMA_VERSION {
            return Err(RegistryLoadError::Registry(
                RegistryError::UnsupportedSchemaVersion {
                    found: document.schema_version,
                    expected: CURRENT_REGISTRY_SCHEMA_VERSION,
                },
            ));
        }

        let mut registry = Self::new();
        for metadata in document.assets {
            registry
                .register(metadata)
                .map_err(RegistryLoadError::Registry)?;
        }
        Ok(registry)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct AssetRegistryDocument {
    schema_version: u32,
    assets: Vec<AssetMetadata>,
}

/// Errors produced by asset metadata and registry operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    EmptyAssetName,
    EmptyImporterName,
    InvalidImporterVersion,
    EmptyImporterOptionKey,
    EmptySourcePath,
    AbsoluteSourcePath,
    ParentDirectoryTraversal,
    InvalidSourcePath,
    DuplicateResourceId { id: ResourceId },
    SourcePathInUse { path: ProjectRelativePath },
    ResourceNotFound { id: ResourceId },
    UnsupportedSchemaVersion { found: u32, expected: u32 },
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyAssetName => formatter.write_str("asset name must not be empty"),
            Self::EmptyImporterName => formatter.write_str("importer name must not be empty"),
            Self::InvalidImporterVersion => formatter.write_str("importer version must be greater than zero"),
            Self::EmptyImporterOptionKey => formatter.write_str("importer option key must not be empty"),
            Self::EmptySourcePath => formatter.write_str("asset source path must not be empty"),
            Self::AbsoluteSourcePath => formatter.write_str("asset source path must be project-relative"),
            Self::ParentDirectoryTraversal => formatter.write_str("asset source path must not traverse parent directories"),
            Self::InvalidSourcePath => formatter.write_str("asset source path contains invalid characters or is not normalized"),
            Self::DuplicateResourceId { id } => write!(formatter, "duplicate asset resource ID {id}"),
            Self::SourcePathInUse { path } => write!(formatter, "asset source path is already registered: {}", path.as_str()),
            Self::ResourceNotFound { id } => write!(formatter, "asset resource ID was not found: {id}"),
            Self::UnsupportedSchemaVersion { found, expected } => write!(
                formatter,
                "unsupported asset registry schema version {found}; expected {expected}"
            ),
        }
    }
}

impl std::error::Error for RegistryError {}

/// Errors produced while decoding and validating a persisted asset registry.
#[derive(Debug)]
pub enum RegistryLoadError {
    Ron(ron::error::SpannedError),
    Registry(RegistryError),
}

impl std::fmt::Display for RegistryLoadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ron(error) => write!(formatter, "failed to decode RON asset registry: {error}"),
            Self::Registry(error) => write!(formatter, "invalid asset registry: {error}"),
        }
    }
}

impl std::error::Error for RegistryLoadError {}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use zyne_core::ResourceId;

    use super::{
        AssetMetadata, AssetRegistry, ImporterSettings, ProjectRelativePath, RegistryError,
        RegistryLoadError, ResourceKind,
    };

    fn sample_metadata(id: ResourceId, path: &str) -> AssetMetadata {
        AssetMetadata::new(
            id,
            "Crate Model",
            ResourceKind::Model,
            ProjectRelativePath::new(path).unwrap(),
            ImporterSettings::new("gltf", 1).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn normalizes_portable_project_relative_paths() {
        let path = ProjectRelativePath::new("models\\./props//crate.glb").unwrap();
        assert_eq!(path.as_str(), "models/props/crate.glb");
    }

    #[test]
    fn rejects_absolute_and_traversing_paths() {
        assert_eq!(
            ProjectRelativePath::new("/assets/model.glb"),
            Err(RegistryError::AbsoluteSourcePath)
        );
        assert_eq!(
            ProjectRelativePath::new("C:\\assets\\model.glb"),
            Err(RegistryError::AbsoluteSourcePath)
        );
        assert_eq!(
            ProjectRelativePath::new("assets/../model.glb"),
            Err(RegistryError::ParentDirectoryTraversal)
        );
        assert_eq!(
            ProjectRelativePath::new(""),
            Err(RegistryError::EmptySourcePath)
        );
    }

    #[test]
    fn rejects_invalid_importer_settings_and_asset_names() {
        assert_eq!(
            ImporterSettings::new(" ", 1),
            Err(RegistryError::EmptyImporterName)
        );
        assert_eq!(
            ImporterSettings::new("gltf", 0),
            Err(RegistryError::InvalidImporterVersion)
        );
        assert_eq!(
            AssetMetadata::new(
                ResourceId::new(),
                " ",
                ResourceKind::Model,
                ProjectRelativePath::new("models/crate.glb").unwrap(),
                ImporterSettings::new("gltf", 1).unwrap(),
            ),
            Err(RegistryError::EmptyAssetName)
        );
    }

    #[test]
    fn registers_and_finds_metadata_by_id_and_source_path() {
        let id = ResourceId::new();
        let metadata = sample_metadata(id, "models/crate.glb");
        let path = metadata.source_path().clone();
        let mut registry = AssetRegistry::new();

        registry.register(metadata.clone()).unwrap();

        assert_eq!(registry.len(), 1);
        assert_eq!(registry.find(id), Some(&metadata));
        assert_eq!(registry.find_by_source_path(&path), Some(&metadata));
    }

    #[test]
    fn rejects_duplicate_ids_and_conflicting_source_paths() {
        let id = ResourceId::new();
        let mut registry = AssetRegistry::new();
        registry
            .register(sample_metadata(id, "models/first.glb"))
            .unwrap();

        assert_eq!(
            registry.register(sample_metadata(id, "models/second.glb")),
            Err(RegistryError::DuplicateResourceId { id })
        );

        assert!(matches!(
            registry.register(sample_metadata(ResourceId::new(), "models/first.glb")),
            Err(RegistryError::SourcePathInUse { .. })
        ));
    }

    #[test]
    fn rename_and_move_preserve_resource_identity() {
        let id = ResourceId::new();
        let mut registry = AssetRegistry::new();
        registry
            .register(sample_metadata(id, "models/crate.glb"))
            .unwrap();

        registry.rename(id, "Wooden Crate").unwrap();
        let new_path = ProjectRelativePath::new("props/wooden-crate.glb").unwrap();
        registry.move_source(id, new_path.clone()).unwrap();

        let metadata = registry.find(id).unwrap();
        assert_eq!(metadata.id(), id);
        assert_eq!(metadata.name(), "Wooden Crate");
        assert_eq!(metadata.source_path(), &new_path);
        assert!(registry
            .find_by_source_path(&ProjectRelativePath::new("models/crate.glb").unwrap())
            .is_none());
        assert_eq!(registry.find_by_source_path(&new_path).unwrap().id(), id);
    }

    #[test]
    fn failed_move_does_not_change_registry_indexes() {
        let first_id = ResourceId::new();
        let second_id = ResourceId::new();
        let mut registry = AssetRegistry::new();
        registry
            .register(sample_metadata(first_id, "models/first.glb"))
            .unwrap();
        registry
            .register(sample_metadata(second_id, "models/second.glb"))
            .unwrap();

        let conflicting_path = ProjectRelativePath::new("models/second.glb").unwrap();
        assert_eq!(
            registry.move_source(first_id, conflicting_path.clone()),
            Err(RegistryError::SourcePathInUse {
                path: conflicting_path
            })
        );
        assert_eq!(
            registry.find(first_id).unwrap().source_path().as_str(),
            "models/first.glb"
        );
    }

    #[test]
    fn removes_metadata_and_releases_source_path() {
        let id = ResourceId::new();
        let mut registry = AssetRegistry::new();
        let metadata = sample_metadata(id, "models/crate.glb");
        let path = metadata.source_path().clone();
        registry.register(metadata.clone()).unwrap();

        assert_eq!(registry.remove(id), Some(metadata.clone()));
        assert!(registry.find(id).is_none());
        assert!(registry.find_by_source_path(&path).is_none());
        registry.register(metadata).unwrap();
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn importer_options_are_stored_in_sorted_map() {
        let mut importer = ImporterSettings::new("gltf", 1).unwrap();
        importer.set_option("z_scale", "1.0").unwrap();
        importer.set_option("a_flip_uv", "false").unwrap();
        assert_eq!(
            importer.options(),
            &BTreeMap::from([
                ("a_flip_uv".to_owned(), "false".to_owned()),
                ("z_scale".to_owned(), "1.0".to_owned()),
            ])
        );
    }

    #[test]
    fn ron_round_trip_preserves_metadata_and_identity() {
        let id = ResourceId::new();
        let mut registry = AssetRegistry::new();
        let mut metadata = sample_metadata(id, "models/crate.glb");
        let mut importer = ImporterSettings::new("gltf", 2).unwrap();
        importer.set_option("flip_uv", "true").unwrap();
        metadata.importer = importer;
        registry.register(metadata.clone()).unwrap();

        let encoded = registry.to_ron().unwrap();
        let decoded = AssetRegistry::from_ron(&encoded).unwrap();

        assert_eq!(decoded.find(id), Some(&metadata));
    }

    #[test]
    fn rejects_malformed_ron_and_unsupported_schema_versions() {
        assert!(matches!(
            AssetRegistry::from_ron("not a registry"),
            Err(RegistryLoadError::Ron(_))
        ));

        let unsupported = ron::ser::to_string(&super::AssetRegistryDocument {
            schema_version: 999,
            assets: Vec::new(),
        })
        .unwrap();
        assert!(matches!(
            AssetRegistry::from_ron(&unsupported),
            Err(RegistryLoadError::Registry(
                RegistryError::UnsupportedSchemaVersion {
                    found: 999,
                    expected: 1
                }
            ))
        ));
    }
}

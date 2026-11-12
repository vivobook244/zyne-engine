//! Scene identity and serializable document contracts for ZYNE.

mod world;

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use zyne_core::ResourceId;
use zyne_math::Transform3d;

pub use world::{Scene, SceneLoadError, SceneSaveError};

const CURRENT_SCENE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SceneId(Uuid);

impl SceneId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    #[must_use]
    pub const fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for SceneId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SceneEntityId(Uuid);

impl SceneEntityId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    #[must_use]
    pub const fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for SceneEntityId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneEntityDocument {
    id: SceneEntityId,
    name: String,
    transform: Transform3d,
    #[serde(default)]
    asset_references: Vec<ResourceId>,
}

impl SceneEntityDocument {
    /// Creates an entity document with a newly generated persistent ID.
    ///
    /// # Errors
    ///
    /// Returns `EmptyEntityName` when the name is blank.
    pub fn new(name: impl Into<String>, transform: Transform3d) -> Result<Self, SceneError> {
        Self::with_id(SceneEntityId::new(), name, transform)
    }

    /// Creates an entity document using an explicit persistent ID.
    ///
    /// # Errors
    ///
    /// Returns `EmptyEntityName` when the name is blank.
    pub fn with_id(
        id: SceneEntityId,
        name: impl Into<String>,
        transform: Transform3d,
    ) -> Result<Self, SceneError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(SceneError::EmptyEntityName { id });
        }
        Ok(Self {
            id,
            name,
            transform,
            asset_references: Vec::new(),
        })
    }

    #[must_use]
    pub const fn id(&self) -> SceneEntityId {
        self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn transform(&self) -> &Transform3d {
        &self.transform
    }

    /// Returns stable asset identities referenced by this entity.
    #[must_use]
    pub fn asset_references(&self) -> &[ResourceId] {
        &self.asset_references
    }

    /// Adds a persistent asset reference while rejecting duplicates.
    ///
    /// # Errors
    ///
    /// Returns `DuplicateAssetReference` when the ID is already attached.
    pub fn add_asset_reference(&mut self, id: ResourceId) -> Result<(), SceneError> {
        if self.asset_references.contains(&id) {
            return Err(SceneError::DuplicateAssetReference {
                entity_id: self.id,
                resource_id: id,
            });
        }
        self.asset_references.push(id);
        Ok(())
    }

    /// Removes a persistent asset reference, returning whether it existed.
    pub fn remove_asset_reference(&mut self, id: ResourceId) -> bool {
        let Some(index) = self.asset_references.iter().position(|existing| *existing == id) else {
            return false;
        };
        self.asset_references.remove(index);
        true
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneDocument {
    schema_version: u32,
    id: SceneId,
    name: String,
    entities: Vec<SceneEntityDocument>,
}

impl SceneDocument {
    /// Creates an empty scene document with a generated persistent ID.
    ///
    /// # Errors
    ///
    /// Returns `EmptySceneName` when the name is blank.
    pub fn new(name: impl Into<String>) -> Result<Self, SceneError> {
        Self::with_id(SceneId::new(), name)
    }

    /// Creates an empty scene document with an explicit persistent ID.
    ///
    /// # Errors
    ///
    /// Returns `EmptySceneName` when the name is blank.
    pub fn with_id(id: SceneId, name: impl Into<String>) -> Result<Self, SceneError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(SceneError::EmptySceneName);
        }
        Ok(Self {
            schema_version: CURRENT_SCENE_SCHEMA_VERSION,
            id,
            name,
            entities: Vec::new(),
        })
    }

    #[must_use]
    pub const fn id(&self) -> SceneId {
        self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    #[must_use]
    pub fn entities(&self) -> &[SceneEntityDocument] {
        &self.entities
    }

    /// Adds an entity document while enforcing persistent-ID uniqueness.
    ///
    /// # Errors
    ///
    /// Returns `DuplicateEntityId` when the document already contains the ID.
    pub fn add_entity(&mut self, entity: SceneEntityDocument) -> Result<(), SceneError> {
        if self
            .entities
            .iter()
            .any(|existing| existing.id == entity.id)
        {
            return Err(SceneError::DuplicateEntityId { id: entity.id });
        }
        self.entities.push(entity);
        Ok(())
    }

    /// Validates schema version, names, and persistent entity and asset identities.
    ///
    /// # Errors
    ///
    /// Returns an error when a name is blank, an entity or asset ID is duplicated,
    /// or the schema version is unsupported.
    pub fn validate(&self) -> Result<(), SceneError> {
        if self.schema_version != CURRENT_SCENE_SCHEMA_VERSION {
            return Err(SceneError::UnsupportedSchemaVersion {
                found: self.schema_version,
                expected: CURRENT_SCENE_SCHEMA_VERSION,
            });
        }
        if self.name.trim().is_empty() {
            return Err(SceneError::EmptySceneName);
        }
        let mut ids = HashSet::with_capacity(self.entities.len());
        for entity in &self.entities {
            if entity.name.trim().is_empty() {
                return Err(SceneError::EmptyEntityName { id: entity.id });
            }
            if !ids.insert(entity.id) {
                return Err(SceneError::DuplicateEntityId { id: entity.id });
            }
            let mut resource_ids = HashSet::with_capacity(entity.asset_references.len());
            for resource_id in &entity.asset_references {
                if !resource_ids.insert(*resource_id) {
                    return Err(SceneError::DuplicateAssetReference {
                        entity_id: entity.id,
                        resource_id: *resource_id,
                    });
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneError {
    EmptySceneName,
    EmptyEntityName { id: SceneEntityId },
    DuplicateEntityId { id: SceneEntityId },
    DuplicateAssetReference {
        entity_id: SceneEntityId,
        resource_id: ResourceId,
    },
    UnsupportedSchemaVersion { found: u32, expected: u32 },
}

impl std::fmt::Display for SceneError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptySceneName => formatter.write_str("scene name must not be empty"),
            Self::EmptyEntityName { id } => {
                write!(formatter, "scene entity {id:?} name must not be empty")
            }
            Self::DuplicateEntityId { id } => {
                write!(formatter, "duplicate scene entity identity {id:?}")
            }
            Self::DuplicateAssetReference {
                entity_id,
                resource_id,
            } => write!(
                formatter,
                "scene entity {entity_id:?} contains duplicate asset reference {resource_id}"
            ),
            Self::UnsupportedSchemaVersion { found, expected } => write!(
                formatter,
                "unsupported scene schema version {found}; expected {expected}"
            ),
        }
    }
}

impl std::error::Error for SceneError {}

#[cfg(test)]
mod tests {
    use super::{SceneDocument, SceneEntityDocument, SceneEntityId, SceneError, SceneId};
    use serde::Serialize;
    use uuid::Uuid;
    use zyne_core::ResourceId;
    use zyne_math::Transform3d;

    #[test]
    fn creates_empty_scene_with_stable_identity_and_schema_version() {
        let scene = SceneDocument::new("Main Level").expect("scene name should be valid");
        assert_eq!(scene.name(), "Main Level");
        assert_eq!(scene.schema_version(), 1);
        assert_eq!(scene.entities(), []);
        assert_eq!(scene.validate(), Ok(()));
    }

    #[test]
    fn accepts_multiple_entities_with_distinct_ids_and_transforms() {
        let mut scene = SceneDocument::new("Main Level").expect("scene should be valid");
        let first = SceneEntityDocument::new("Player", Transform3d::identity()).unwrap();
        let second = SceneEntityDocument::new(
            "Prop",
            Transform3d::new([2.0, 0.0, 4.0], [0.0, 0.0, 0.0, 1.0], [2.0; 3]).unwrap(),
        )
        .unwrap();
        scene.add_entity(first).unwrap();
        scene.add_entity(second).unwrap();
        assert_eq!(scene.entities().len(), 2);
        assert_ne!(scene.entities()[0].id(), scene.entities()[1].id());
        assert_eq!(scene.validate(), Ok(()));
    }

    #[test]
    fn rejects_duplicate_entity_ids() {
        let mut scene = SceneDocument::new("Main Level").unwrap();
        let id = SceneEntityId::new();
        let first = SceneEntityDocument::with_id(id, "First", Transform3d::identity()).unwrap();
        let second = SceneEntityDocument::with_id(id, "Second", Transform3d::identity()).unwrap();
        scene.add_entity(first).unwrap();
        assert_eq!(
            scene.add_entity(second),
            Err(SceneError::DuplicateEntityId { id })
        );
    }

    #[test]
    fn rejects_blank_scene_and_entity_names() {
        assert_eq!(SceneDocument::new("   "), Err(SceneError::EmptySceneName));
        let id = SceneEntityId::from_uuid(Uuid::nil());
        assert_eq!(
            SceneEntityDocument::with_id(id, "  ", Transform3d::identity()),
            Err(SceneError::EmptyEntityName { id })
        );
    }

    #[test]
    fn entity_asset_references_are_unique_and_removable() {
        let mut entity = SceneEntityDocument::new("Player", Transform3d::identity()).unwrap();
        let resource_id = ResourceId::new();
        entity.add_asset_reference(resource_id).unwrap();
        assert_eq!(entity.asset_references(), &[resource_id]);
        assert_eq!(
            entity.add_asset_reference(resource_id),
            Err(SceneError::DuplicateAssetReference {
                entity_id: entity.id(),
                resource_id,
            })
        );
        assert!(entity.remove_asset_reference(resource_id));
        assert!(!entity.remove_asset_reference(resource_id));
        assert_eq!(entity.asset_references(), []);
    }

    #[test]
    fn scene_document_round_trips_persistent_asset_references() {
        let mut scene = SceneDocument::new("Asset References").unwrap();
        let mut entity = SceneEntityDocument::new("Player", Transform3d::identity()).unwrap();
        let resource_id = ResourceId::new();
        entity.add_asset_reference(resource_id).unwrap();
        entity.add_asset_reference(ResourceId::new()).unwrap();
        scene.add_entity(entity).unwrap();

        let encoded = ron::ser::to_string(&scene).unwrap();
        let decoded: SceneDocument = ron::de::from_str(&encoded).unwrap();
        assert_eq!(decoded, scene);
        assert_eq!(decoded.entities()[0].asset_references().len(), 2);
        assert_eq!(decoded.validate(), Ok(()));
    }

    #[test]
    fn legacy_scene_documents_without_asset_references_remain_loadable() {
        #[derive(Serialize)]
        struct LegacySceneDocument {
            schema_version: u32,
            id: SceneId,
            name: String,
            entities: Vec<LegacySceneEntityDocument>,
        }

        #[derive(Serialize)]
        struct LegacySceneEntityDocument {
            id: SceneEntityId,
            name: String,
            transform: Transform3d,
        }

        let source = LegacySceneDocument {
            schema_version: 1,
            id: SceneId::from_uuid(Uuid::from_u128(1)),
            name: "Legacy".to_owned(),
            entities: vec![LegacySceneEntityDocument {
                id: SceneEntityId::from_uuid(Uuid::from_u128(2)),
                name: "Player".to_owned(),
                transform: Transform3d::identity(),
            }],
        };
        let encoded = ron::ser::to_string(&source).unwrap();
        let decoded: SceneDocument = ron::de::from_str(&encoded).unwrap();
        assert_eq!(decoded.entities()[0].asset_references(), []);
        assert_eq!(decoded.validate(), Ok(()));
    }

    #[test]
    fn document_round_trips_through_serde_format_for_contract_testing() {
        let mut scene = SceneDocument::new("Round Trip").unwrap();
        scene
            .add_entity(SceneEntityDocument::new("Camera Marker", Transform3d::identity()).unwrap())
            .unwrap();
        let encoded = toml::to_string(&scene).unwrap();
        let decoded: SceneDocument = toml::from_str(&encoded).unwrap();
        assert_eq!(decoded, scene);
        assert_eq!(decoded.validate(), Ok(()));
    }

    #[test]
    fn deserialization_rejects_invalid_transform_rotation() {
        let mut scene = SceneDocument::new("Invalid Transform").unwrap();
        scene
            .add_entity(SceneEntityDocument::new("Object", Transform3d::identity()).unwrap())
            .unwrap();
        let encoded = toml::to_string(&scene).unwrap();
        let invalid = encoded.replace(
            "rotation = [0.0, 0.0, 0.0, 1.0]",
            "rotation = [0.0, 0.0, 0.0, 0.0]",
        );
        assert_ne!(invalid, encoded);
        assert!(toml::from_str::<SceneDocument>(&invalid).is_err());
    }
}

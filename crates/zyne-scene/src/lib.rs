//! Scene identity and serializable document contracts for ZYNE.
//!
//! Runtime ECS ownership and RON persistence are layered on these stable
//! document types in the following Phase 3 implementation steps.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use zyne_math::Transform3d;

const CURRENT_SCENE_SCHEMA_VERSION: u32 = 1;

/// Persistent identity for a scene document.
///
/// This identifier is independent from runtime ECS entities and resource IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SceneId(Uuid);

impl SceneId {
    /// Creates a new globally unique scene identity.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Wraps an existing UUID, for example one read from a scene document.
    #[must_use]
    pub const fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the underlying UUID.
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

/// Persistent identity for an entity inside a scene document.
///
/// It is distinct from a runtime ECS entity handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SceneEntityId(Uuid);

impl SceneEntityId {
    /// Creates a new globally unique scene-entity identity.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Wraps an existing UUID, for example one read from a scene document.
    #[must_use]
    pub const fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the underlying UUID.
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

/// A serializable entity record in a scene document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneEntityDocument {
    id: SceneEntityId,
    name: String,
    transform: Transform3d,
}

impl SceneEntityDocument {
    /// Creates an entity record with a new persistent identity.
    ///
    /// # Errors
    ///
    /// Returns EmptyEntityName when the name is blank.
    pub fn new(
        name: impl Into<String>,
        transform: Transform3d,
    ) -> Result<Self, SceneError> {
        Self::with_id(SceneEntityId::new(), name, transform)
    }

    /// Creates an entity record with an explicit persistent identity.
    ///
    /// This is useful when restoring a document or constructing fixtures.
    ///
    /// # Errors
    ///
    /// Returns EmptyEntityName when the name is blank.
    pub fn with_id(
        id: SceneEntityId,
        name: impl Into<String>,
        transform: Transform3d,
    ) -> Result<Self, SceneError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(SceneError::EmptyEntityName { id });
        }

        Ok(Self { id, name, transform })
    }

    /// Returns the persistent scene-entity identity.
    #[must_use]
    pub const fn id(&self) -> SceneEntityId {
        self.id
    }

    /// Returns the entity display name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the local transform.
    #[must_use]
    pub const fn transform(&self) -> &Transform3d {
        &self.transform
    }
}

/// A versioned, serializable scene document independent from runtime handles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneDocument {
    schema_version: u32,
    id: SceneId,
    name: String,
    entities: Vec<SceneEntityDocument>,
}

impl SceneDocument {
    /// Creates an empty scene document with a new persistent identity.
    ///
    /// # Errors
    ///
    /// Returns EmptySceneName when the name is blank.
    pub fn new(name: impl Into<String>) -> Result<Self, SceneError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(SceneError::EmptySceneName);
        }

        Ok(Self {
            schema_version: CURRENT_SCENE_SCHEMA_VERSION,
            id: SceneId::new(),
            name,
            entities: Vec::new(),
        })
    }

    /// Returns the persistent scene identity.
    #[must_use]
    pub const fn id(&self) -> SceneId {
        self.id
    }

    /// Returns the human-readable scene name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the serialized schema version.
    #[must_use]
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    /// Returns scene entity records in document order.
    #[must_use]
    pub fn entities(&self) -> &[SceneEntityDocument] {
        &self.entities
    }

    /// Adds an entity while preserving unique persistent IDs.
    ///
    /// # Errors
    ///
    /// Returns DuplicateEntityId if the document already contains the identity.
    pub fn add_entity(&mut self, entity: SceneEntityDocument) -> Result<(), SceneError> {
        if self.entities.iter().any(|existing| existing.id == entity.id) {
            return Err(SceneError::DuplicateEntityId { id: entity.id });
        }

        self.entities.push(entity);
        Ok(())
    }

    /// Validates a document after deserialization or before persistence.
    ///
    /// # Errors
    ///
    /// Returns an error for an unsupported schema version, blank names, or
    /// duplicate persistent entity identities.
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
        }

        Ok(())
    }
}

/// Errors returned while constructing or validating scene documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneError {
    /// The scene name is empty or whitespace-only.
    EmptySceneName,
    /// An entity name is empty or whitespace-only.
    EmptyEntityName { id: SceneEntityId },
    /// The document contains the same persistent entity identity more than once.
    DuplicateEntityId { id: SceneEntityId },
    /// The document schema is newer or older than this implementation supports.
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
    use super::{SceneDocument, SceneEntityDocument, SceneEntityId, SceneError};
    use uuid::Uuid;
    use zyne_math::Transform3d;

    #[test]
    fn creates_empty_scene_with_stable_identity_and_schema_version() {
        let scene = SceneDocument::new("Main Level").expect("scene name should be valid");

        assert_eq!(scene.name(), "Main Level");
        assert_eq!(scene.schema_version(), 1);
        assert!(scene.entities().is_empty());
        assert_eq!(scene.validate(), Ok(()));
    }

    #[test]
    fn accepts_multiple_entities_with_distinct_ids_and_transforms() {
        let mut scene = SceneDocument::new("Main Level").expect("scene should be valid");
        let first = SceneEntityDocument::new("Player", Transform3d::identity())
            .expect("entity name should be valid");
        let second = SceneEntityDocument::new(
            "Prop",
            Transform3d::new([2.0, 0.0, 4.0], [0.0, 0.0, 0.0, 1.0], [2.0; 3])
                .expect("transform should be valid"),
        )
        .expect("entity name should be valid");

        scene.add_entity(first).expect("first ID should be unique");
        scene.add_entity(second).expect("second ID should be unique");

        assert_eq!(scene.entities().len(), 2);
        assert_ne!(scene.entities()[0].id(), scene.entities()[1].id());
        assert_eq!(scene.validate(), Ok(()));
    }

    #[test]
    fn rejects_duplicate_entity_ids() {
        let mut scene = SceneDocument::new("Main Level").expect("scene should be valid");
        let id = SceneEntityId::new();
        let first = SceneEntityDocument::with_id(id, "First", Transform3d::identity())
            .expect("entity should be valid");
        let second = SceneEntityDocument::with_id(id, "Second", Transform3d::identity())
            .expect("entity should be valid");

        scene.add_entity(first).expect("first ID should be unique");
        assert_eq!(
            scene.add_entity(second),
            Err(SceneError::DuplicateEntityId { id })
        );
    }

    #[test]
    fn rejects_blank_scene_and_entity_names() {
        assert_eq!(
            SceneDocument::new("   "),
            Err(SceneError::EmptySceneName)
        );
        let nil_id = SceneEntityId::from_uuid(Uuid::nil());
        assert_eq!(
            SceneEntityDocument::with_id(nil_id, "  ", Transform3d::identity()),
            Err(SceneError::EmptyEntityName { id: nil_id })
        );
    }

    #[test]
    fn document_round_trips_through_serde_format_for_contract_testing() {
        let mut scene = SceneDocument::new("Round Trip").expect("scene should be valid");
        scene
            .add_entity(
                SceneEntityDocument::new("Camera Marker", Transform3d::identity())
                    .expect("entity should be valid"),
            )
            .expect("entity ID should be unique");

        let encoded = toml::to_string(&scene).expect("scene should serialize");
        let decoded: SceneDocument =
            toml::from_str(&encoded).expect("scene should deserialize");

        assert_eq!(decoded, scene);
        assert_eq!(decoded.validate(), Ok(()));
    }

    #[test]
    fn deserialization_rejects_invalid_transform_rotation() {
        let mut scene = SceneDocument::new("Invalid Transform").expect("scene should be valid");
        scene
            .add_entity(
                SceneEntityDocument::new("Object", Transform3d::identity())
                    .expect("entity should be valid"),
            )
            .expect("entity ID should be unique");
        let encoded = toml::to_string(&scene).expect("scene should serialize");
        let invalid = encoded.replace(
            "rotation = [0.0, 0.0, 0.0, 1.0]",
            "rotation = [0.0, 0.0, 0.0, 0.0]",
        );

        assert_ne!(invalid, encoded);
        assert!(toml::from_str::<SceneDocument>(&invalid).is_err());
    }
}

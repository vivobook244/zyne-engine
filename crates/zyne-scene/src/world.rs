//! Runtime ECS ownership and RON persistence for scenes.

use std::collections::HashMap;

use bevy_ecs::{component::Component, entity::Entity, world::World};
use zyne_math::Transform3d;

use crate::{SceneDocument, SceneEntityDocument, SceneEntityId, SceneError, SceneId};

#[derive(Component, Debug, Clone, Copy)]
struct PersistentEntityId(SceneEntityId);

#[derive(Component, Debug, Clone)]
struct EntityName(String);

#[derive(Component, Debug, Clone, Copy)]
struct LocalTransform(Transform3d);

/// An in-memory scene that owns its ECS world.
pub struct Scene {
    id: SceneId,
    name: String,
    world: World,
    entities: HashMap<SceneEntityId, Entity>,
}

impl Scene {
    /// Creates an empty scene with a new persistent identity.
    ///
    /// # Errors
    ///
    /// Returns `EmptySceneName` when the name is blank.
    pub fn new(name: impl Into<String>) -> Result<Self, SceneError> {
        let document = SceneDocument::new(name)?;
        Ok(Self::from_validated_document(&document))
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
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    #[must_use]
    pub fn contains_entity(&self, id: SceneEntityId) -> bool {
        self.entities.contains_key(&id)
    }

    #[must_use]
    pub fn entity_ids(&self) -> Vec<SceneEntityId> {
        let mut ids: Vec<_> = self.entities.keys().copied().collect();
        ids.sort_unstable_by_key(|id| *id.as_uuid().as_bytes());
        ids
    }

    /// Extracts live local transforms in deterministic persistent-ID order.
    ///
    /// The returned data contains no ECS handles or component references.
    #[must_use]
    pub fn entity_transforms(&self) -> Vec<Transform3d> {
        self.entity_ids()
            .into_iter()
            .filter_map(|id| self.entity_transform(id))
            .collect()
    }

    /// Spawns an entity with a generated persistent identity.
    ///
    /// # Errors
    ///
    /// Returns `EmptyEntityName` when the name is blank.
    pub fn spawn_entity(
        &mut self,
        name: impl Into<String>,
        transform: Transform3d,
    ) -> Result<SceneEntityId, SceneError> {
        self.spawn_entity_with_id(SceneEntityId::new(), name, transform)
    }

    /// Spawns an entity using a caller-provided persistent identity.
    ///
    /// # Errors
    ///
    /// Returns `EmptyEntityName` for a blank name or `DuplicateEntityId` when
    /// the ID is already present in this scene.
    pub fn spawn_entity_with_id(
        &mut self,
        id: SceneEntityId,
        name: impl Into<String>,
        transform: Transform3d,
    ) -> Result<SceneEntityId, SceneError> {
        let document = SceneEntityDocument::with_id(id, name, transform)?;
        if self.entities.contains_key(&id) {
            return Err(SceneError::DuplicateEntityId { id });
        }
        let handle = self
            .world
            .spawn((
                PersistentEntityId(document.id()),
                EntityName(document.name().to_owned()),
                LocalTransform(*document.transform()),
            ))
            .id();
        self.entities.insert(id, handle);
        Ok(id)
    }

    pub fn despawn_entity(&mut self, id: SceneEntityId) -> bool {
        let Some(handle) = self.entities.remove(&id) else {
            return false;
        };
        self.world.despawn(handle)
    }

    #[must_use]
    pub fn entity_name(&self, id: SceneEntityId) -> Option<&str> {
        let handle = *self.entities.get(&id)?;
        let persistent_id = self.world.get::<PersistentEntityId>(handle)?;
        if persistent_id.0 != id {
            return None;
        }
        self.world
            .get::<EntityName>(handle)
            .map(|name| name.0.as_str())
    }

    #[must_use]
    pub fn entity_transform(&self, id: SceneEntityId) -> Option<Transform3d> {
        let handle = *self.entities.get(&id)?;
        let persistent_id = self.world.get::<PersistentEntityId>(handle)?;
        if persistent_id.0 != id {
            return None;
        }
        self.world
            .get::<LocalTransform>(handle)
            .map(|value| value.0)
    }

    pub fn set_entity_transform(&mut self, id: SceneEntityId, transform: Transform3d) -> bool {
        let Some(&handle) = self.entities.get(&id) else {
            return false;
        };
        let Some(persistent_id) = self.world.get::<PersistentEntityId>(handle) else {
            return false;
        };
        if persistent_id.0 != id {
            return false;
        }
        let Some(mut local_transform) = self.world.get_mut::<LocalTransform>(handle) else {
            return false;
        };
        local_transform.0 = transform;
        true
    }

    /// Extracts a validated, ECS-independent scene document.
    ///
    /// # Errors
    ///
    /// Returns an error if the in-memory scene violates document invariants.
    pub fn to_document(&self) -> Result<SceneDocument, SceneError> {
        let mut document = SceneDocument::with_id(self.id, self.name.clone())?;
        for id in self.entity_ids() {
            let name = self
                .entity_name(id)
                .ok_or(SceneError::DuplicateEntityId { id })?;
            let transform = self
                .entity_transform(id)
                .ok_or(SceneError::DuplicateEntityId { id })?;
            document.add_entity(SceneEntityDocument::with_id(id, name, transform)?)?;
        }
        document.validate()?;
        Ok(document)
    }

    /// Reconstructs an ECS scene from a validated scene document.
    ///
    /// # Errors
    ///
    /// Returns an error when the document contains invalid names, duplicate
    /// IDs, or an unsupported schema version.
    pub fn from_document(document: &SceneDocument) -> Result<Self, SceneError> {
        document.validate()?;
        let mut scene = Self::from_validated_document(document);
        for entity in document.entities() {
            scene.spawn_entity_with_id(entity.id(), entity.name(), *entity.transform())?;
        }
        Ok(scene)
    }

    /// Serializes the scene to a pretty-printed RON document.
    ///
    /// # Errors
    ///
    /// Returns a scene validation or RON serialization error.
    pub fn to_ron(&self) -> Result<String, SceneSaveError> {
        let document = self.to_document().map_err(SceneSaveError::Scene)?;
        ron::ser::to_string_pretty(&document, ron::ser::PrettyConfig::default())
            .map_err(SceneSaveError::Ron)
    }

    /// Loads a scene from a RON document.
    ///
    /// # Errors
    ///
    /// Returns a decoding error or a scene validation error.
    pub fn from_ron(source: &str) -> Result<Self, SceneLoadError> {
        let document: SceneDocument = ron::de::from_str(source).map_err(SceneLoadError::Ron)?;
        Self::from_document(&document).map_err(SceneLoadError::Scene)
    }

    fn from_validated_document(document: &SceneDocument) -> Self {
        Self {
            id: document.id(),
            name: document.name().to_owned(),
            world: World::new(),
            entities: HashMap::new(),
        }
    }
}

#[derive(Debug)]
pub enum SceneLoadError {
    Ron(ron::error::SpannedError),
    Scene(SceneError),
}

impl std::fmt::Display for SceneLoadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ron(error) => write!(formatter, "failed to decode RON scene: {error}"),
            Self::Scene(error) => write!(formatter, "invalid scene document: {error}"),
        }
    }
}

impl std::error::Error for SceneLoadError {}

#[derive(Debug)]
pub enum SceneSaveError {
    Scene(SceneError),
    Ron(ron::Error),
}

impl std::fmt::Display for SceneSaveError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Scene(error) => write!(formatter, "invalid scene state: {error}"),
            Self::Ron(error) => write!(formatter, "failed to encode RON scene: {error}"),
        }
    }
}

impl std::error::Error for SceneSaveError {}

#[cfg(test)]
mod tests {
    use super::{Scene, SceneLoadError};
    use crate::{SceneEntityId, SceneError, SceneId};
    use uuid::Uuid;
    use zyne_math::Transform3d;

    #[test]
    fn creates_scene_and_manages_entity_lifecycle() {
        let mut scene = Scene::new("Main").unwrap();
        let id = scene
            .spawn_entity("Player", Transform3d::identity())
            .unwrap();
        assert_eq!(scene.entity_count(), 1);
        assert!(scene.contains_entity(id));
        assert_eq!(scene.entity_name(id), Some("Player"));
        assert_eq!(scene.entity_transform(id), Some(Transform3d::identity()));

        let updated = Transform3d::new([1.0, 2.0, 3.0], [0.0, 0.0, 0.0, 1.0], [2.0; 3]).unwrap();
        assert!(scene.set_entity_transform(id, updated));
        assert_eq!(scene.entity_transform(id), Some(updated));
        assert!(scene.despawn_entity(id));
        assert!(!scene.despawn_entity(id));
        assert_eq!(scene.entity_count(), 0);
    }

    #[test]
    fn rejects_duplicate_ids_and_blank_entity_names() {
        let mut scene = Scene::new("Main").unwrap();
        let id = SceneEntityId::new();
        scene
            .spawn_entity_with_id(id, "First", Transform3d::identity())
            .unwrap();
        assert_eq!(
            scene.spawn_entity_with_id(id, "Second", Transform3d::identity()),
            Err(SceneError::DuplicateEntityId { id })
        );
        assert!(matches!(
            scene.spawn_entity("  ", Transform3d::identity()),
            Err(SceneError::EmptyEntityName { .. })
        ));
    }

    #[test]
    fn sorts_entity_ids_deterministically() {
        let mut scene = Scene::new("Main").unwrap();
        let second = SceneEntityId::from_uuid(Uuid::from_u128(2));
        let first = SceneEntityId::from_uuid(Uuid::from_u128(1));
        scene
            .spawn_entity_with_id(second, "Second", Transform3d::identity())
            .unwrap();
        scene
            .spawn_entity_with_id(first, "First", Transform3d::identity())
            .unwrap();
        assert_eq!(scene.entity_ids(), vec![first, second]);
    }

    #[test]
    fn extracts_entity_transforms_in_persistent_id_order() {
        let mut scene = Scene::new("Render Extraction").unwrap();
        let second_id = SceneEntityId::from_uuid(Uuid::from_u128(2));
        let first_id = SceneEntityId::from_uuid(Uuid::from_u128(1));
        let second_transform =
            Transform3d::new([2.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], [1.0; 3]).unwrap();
        let first_transform =
            Transform3d::new([1.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], [1.0; 3]).unwrap();
        scene
            .spawn_entity_with_id(second_id, "Second", second_transform)
            .unwrap();
        scene
            .spawn_entity_with_id(first_id, "First", first_transform)
            .unwrap();

        assert_eq!(
            scene.entity_transforms(),
            vec![first_transform, second_transform]
        );
    }

    #[test]
    fn document_conversion_preserves_scene_and_entity_data() {
        let scene_id = SceneId::from_uuid(Uuid::from_u128(10));
        let entity_id = SceneEntityId::from_uuid(Uuid::from_u128(20));
        let document = crate::SceneDocument::with_id(scene_id, "Persistent Scene").unwrap();
        let mut scene = Scene::from_document(&document).unwrap();
        let transform =
            Transform3d::new([3.0, 4.0, 5.0], [0.0, 0.0, 0.0, 1.0], [1.0, 2.0, 3.0]).unwrap();
        scene
            .spawn_entity_with_id(entity_id, "Prop", transform)
            .unwrap();

        let document = scene.to_document().unwrap();
        assert_eq!(document.id(), scene_id);
        assert_eq!(document.entities().len(), 1);
        assert_eq!(document.entities()[0].id(), entity_id);
        assert_eq!(document.entities()[0].name(), "Prop");
        assert_eq!(*document.entities()[0].transform(), transform);
    }

    #[test]
    fn ron_round_trip_preserves_scene_and_entity_identity() {
        let scene_id = SceneId::from_uuid(Uuid::from_u128(100));
        let entity_id = SceneEntityId::from_uuid(Uuid::from_u128(200));
        let mut scene =
            Scene::from_document(&crate::SceneDocument::with_id(scene_id, "RON Scene").unwrap())
                .unwrap();
        let transform =
            Transform3d::new([5.0, 1.0, -2.0], [0.0, 0.0, 0.0, 1.0], [1.5, 1.0, 0.5]).unwrap();
        scene
            .spawn_entity_with_id(entity_id, "Crate", transform)
            .unwrap();

        let encoded = scene.to_ron().unwrap();
        let decoded = Scene::from_ron(&encoded).unwrap();
        assert_eq!(decoded.id(), scene_id);
        assert_eq!(decoded.name(), "RON Scene");
        assert_eq!(decoded.entity_count(), 1);
        assert_eq!(decoded.entity_name(entity_id), Some("Crate"));
        assert_eq!(decoded.entity_transform(entity_id), Some(transform));
    }

    #[test]
    fn loads_empty_scene() {
        let scene = Scene::new("Empty").unwrap();
        let decoded = Scene::from_ron(&scene.to_ron().unwrap()).unwrap();
        assert_eq!(decoded.id(), scene.id());
        assert_eq!(decoded.entity_count(), 0);
    }

    #[test]
    fn rejects_malformed_ron() {
        assert!(matches!(
            Scene::from_ron("not a scene"),
            Err(SceneLoadError::Ron(_))
        ));
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let mut document = crate::SceneDocument::new("Future").unwrap();
        document.schema_version = 999;
        let encoded = ron::ser::to_string(&document).unwrap();
        assert!(matches!(
            Scene::from_ron(&encoded),
            Err(SceneLoadError::Scene(
                SceneError::UnsupportedSchemaVersion {
                    found: 999,
                    expected: 1
                }
            ))
        ));
    }
}

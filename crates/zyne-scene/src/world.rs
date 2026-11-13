//! Runtime ECS ownership and RON persistence for scenes.

use std::collections::HashMap;

use bevy_ecs::{component::Component, entity::Entity, world::World};
use zyne_core::ResourceId;
use zyne_math::Transform3d;

use crate::{
    DirectionalLight, PointLight, SceneDocument, SceneEntityDocument, SceneEntityId, SceneError,
    SceneId, SpotLight,
};

#[derive(Component, Debug, Clone, Copy)]
struct PersistentEntityId(SceneEntityId);

#[derive(Component, Debug, Clone)]
struct EntityName(String);

#[derive(Component, Debug, Clone, Copy)]
struct LocalTransform(Transform3d);

#[derive(Component, Debug, Clone, Default)]
struct EntityAssetReferences(Vec<ResourceId>);

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
                EntityAssetReferences::default(),
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

    /// Returns persistent asset references attached to an entity.
    #[must_use]
    pub fn entity_asset_references(&self, id: SceneEntityId) -> Option<&[ResourceId]> {
        let handle = *self.entities.get(&id)?;
        let persistent_id = self.world.get::<PersistentEntityId>(handle)?;
        if persistent_id.0 != id {
            return None;
        }
        self.world
            .get::<EntityAssetReferences>(handle)
            .map(|references| references.0.as_slice())
    }

    /// Replaces an entity's persistent asset references.
    ///
    /// # Errors
    ///
    /// Returns `DuplicateAssetReference` when the supplied IDs contain duplicates.
    pub fn set_entity_asset_references(
        &mut self,
        id: SceneEntityId,
        references: &[ResourceId],
    ) -> Result<bool, SceneError> {
        let mut unique = std::collections::HashSet::with_capacity(references.len());
        for resource_id in references {
            if !unique.insert(*resource_id) {
                return Err(SceneError::DuplicateAssetReference {
                    entity_id: id,
                    resource_id: *resource_id,
                });
            }
        }
        let Some(&handle) = self.entities.get(&id) else {
            return Ok(false);
        };
        let Some(persistent_id) = self.world.get::<PersistentEntityId>(handle) else {
            return Ok(false);
        };
        if persistent_id.0 != id {
            return Ok(false);
        }
        let Some(mut asset_references) = self.world.get_mut::<EntityAssetReferences>(handle) else {
            return Ok(false);
        };
        asset_references.0 = references.to_vec();
        Ok(true)
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

    /// Adds, replaces, or removes a directional light on an entity.
    pub fn set_entity_directional_light(
        &mut self,
        id: SceneEntityId,
        light: Option<DirectionalLight>,
    ) -> bool {
        let Some(&handle) = self.entities.get(&id) else {
            return false;
        };
        if self
            .world
            .get::<PersistentEntityId>(handle)
            .is_none_or(|persistent_id| persistent_id.0 != id)
        {
            return false;
        }
        let mut entity = self.world.entity_mut(handle);
        if let Some(light) = light {
            entity.insert(light);
        } else {
            entity.remove::<DirectionalLight>();
        }
        true
    }

    /// Returns an entity's directional light, if present.
    #[must_use]
    pub fn entity_directional_light(&self, id: SceneEntityId) -> Option<DirectionalLight> {
        let handle = *self.entities.get(&id)?;
        if self
            .world
            .get::<PersistentEntityId>(handle)
            .is_none_or(|persistent_id| persistent_id.0 != id)
        {
            return None;
        }
        self.world.get::<DirectionalLight>(handle).copied()
    }

    /// Adds, replaces, or removes a point light on an entity.
    pub fn set_entity_point_light(
        &mut self,
        id: SceneEntityId,
        light: Option<PointLight>,
    ) -> bool {
        let Some(&handle) = self.entities.get(&id) else {
            return false;
        };
        if self
            .world
            .get::<PersistentEntityId>(handle)
            .is_none_or(|persistent_id| persistent_id.0 != id)
        {
            return false;
        }
        let mut entity = self.world.entity_mut(handle);
        if let Some(light) = light {
            entity.insert(light);
        } else {
            entity.remove::<PointLight>();
        }
        true
    }

    /// Returns an entity's point light, if present.
    #[must_use]
    pub fn entity_point_light(&self, id: SceneEntityId) -> Option<PointLight> {
        let handle = *self.entities.get(&id)?;
        if self
            .world
            .get::<PersistentEntityId>(handle)
            .is_none_or(|persistent_id| persistent_id.0 != id)
        {
            return None;
        }
        self.world.get::<PointLight>(handle).copied()
    }

    /// Adds, replaces, or removes a spot light on an entity.
    pub fn set_entity_spot_light(
        &mut self,
        id: SceneEntityId,
        light: Option<SpotLight>,
    ) -> bool {
        let Some(&handle) = self.entities.get(&id) else {
            return false;
        };
        if self
            .world
            .get::<PersistentEntityId>(handle)
            .is_none_or(|persistent_id| persistent_id.0 != id)
        {
            return false;
        }
        let mut entity = self.world.entity_mut(handle);
        if let Some(light) = light {
            entity.insert(light);
        } else {
            entity.remove::<SpotLight>();
        }
        true
    }

    /// Returns an entity's spot light, if present.
    #[must_use]
    pub fn entity_spot_light(&self, id: SceneEntityId) -> Option<SpotLight> {
        let handle = *self.entities.get(&id)?;
        if self
            .world
            .get::<PersistentEntityId>(handle)
            .is_none_or(|persistent_id| persistent_id.0 != id)
        {
            return None;
        }
        self.world.get::<SpotLight>(handle).copied()
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
            let mut entity = SceneEntityDocument::with_id(id, name, transform)?;
            if let Some(references) = self.entity_asset_references(id) {
                for resource_id in references {
                    entity.add_asset_reference(*resource_id)?;
                }
            }
            entity.set_directional_light(self.entity_directional_light(id));
            entity.set_point_light(self.entity_point_light(id));
            entity.set_spot_light(self.entity_spot_light(id));
            document.add_entity(entity)?;
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
            let references = entity.asset_references();
            if !references.is_empty() {
                scene.set_entity_asset_references(entity.id(), references)?;
            }
            scene.set_entity_directional_light(entity.id(), entity.directional_light());
            scene.set_entity_point_light(entity.id(), entity.point_light());
            scene.set_entity_spot_light(entity.id(), entity.spot_light());
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
    use zyne_core::ResourceId;
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
    fn entities_manage_directional_point_and_spot_light_components() {
        use crate::{DirectionalLight, PointLight, SpotLight};

        let mut scene = Scene::new("Lights").unwrap();
        let entity = scene
            .spawn_entity("Light Source", Transform3d::identity())
            .unwrap();
        let directional = DirectionalLight::new([0.0, -1.0, 0.0], [1.0; 3], 2.0).unwrap();
        let point = PointLight::new([1.0, 0.5, 0.25], 3.0, 10.0).unwrap();
        let spot = SpotLight::new(
            [0.0, 0.0, -1.0],
            [1.0; 3],
            4.0,
            12.0,
            15.0_f32.to_radians(),
            25.0_f32.to_radians(),
        )
        .unwrap();

        assert!(scene.set_entity_directional_light(entity, Some(directional)));
        assert!(scene.set_entity_point_light(entity, Some(point)));
        assert!(scene.set_entity_spot_light(entity, Some(spot)));
        assert_eq!(scene.entity_directional_light(entity), Some(directional));
        assert_eq!(scene.entity_point_light(entity), Some(point));
        assert_eq!(scene.entity_spot_light(entity), Some(spot));

        assert!(scene.set_entity_point_light(entity, None));
        assert_eq!(scene.entity_point_light(entity), None);
        assert!(scene.despawn_entity(entity));
        assert_eq!(scene.entity_directional_light(entity), None);
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
    fn scene_preserves_asset_references_through_ron_round_trip() {
        let mut scene = Scene::new("Asset Scene").unwrap();
        let entity_id = scene
            .spawn_entity("Player", Transform3d::identity())
            .unwrap();
        let first = ResourceId::new();
        let second = ResourceId::new();
        assert!(
            scene
                .set_entity_asset_references(entity_id, &[first, second])
                .unwrap()
        );

        let encoded = scene.to_ron().unwrap();
        let decoded = Scene::from_ron(&encoded).unwrap();
        assert_eq!(
            decoded.entity_asset_references(entity_id),
            Some(&[first, second][..])
        );
    }

    #[test]
    fn scene_rejects_duplicate_asset_references() {
        let mut scene = Scene::new("Duplicate References").unwrap();
        let entity_id = scene
            .spawn_entity("Player", Transform3d::identity())
            .unwrap();
        let resource_id = ResourceId::new();

        assert_eq!(
            scene.set_entity_asset_references(entity_id, &[resource_id, resource_id]),
            Err(SceneError::DuplicateAssetReference {
                entity_id,
                resource_id,
            })
        );
        assert_eq!(scene.entity_asset_references(entity_id), Some(&[][..]));
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

//! Persistent resource identities and typed runtime handles.

use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Stable identity for an asset or other persisted engine resource.
///
/// A resource ID is suitable for project and scene serialization. It is not a
/// runtime slot reference and does not imply that the resource is currently loaded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ResourceId(Uuid);

impl ResourceId {
    /// Creates a new randomly generated resource identity.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Creates an identity from an existing UUID.
    #[must_use]
    pub const fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the underlying UUID.
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for ResourceId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ResourceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// A typed reference to a resource stored in a runtime slot.
///
/// The index identifies a slot and the generation distinguishes successive
/// occupants of that slot. A handle is valid only while its storage confirms
/// that both parts match. It is not stable across process runs or serialization.
pub struct Handle<T> {
    index: u32,
    generation: u32,
    marker: PhantomData<fn() -> T>,
}

impl<T> Handle<T> {
    fn from_raw_parts(index: u32, generation: u32) -> Self {
        Self {
            index,
            generation,
            marker: PhantomData,
        }
    }

    /// Returns the runtime slot index.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.index
    }

    /// Returns the slot generation.
    #[must_use]
    pub const fn generation(self) -> u32 {
        self.generation
    }
}

impl<T> Copy for Handle<T> {}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.generation == other.generation
    }
}

impl<T> Eq for Handle<T> {}

impl<T> Hash for Handle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        self.generation.hash(state);
    }
}

impl<T> fmt::Debug for Handle<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Handle")
            .field("index", &self.index)
            .field("generation", &self.generation)
            .finish()
    }
}

struct Slot<T> {
    generation: u32,
    value: Option<T>,
}

/// Typed, generational storage for runtime resources.
///
/// Each storage instance manages one resource type. Removing a value advances
/// its slot generation before that slot can be reused, making previous handles
/// stale. A slot whose generation reaches the integer limit is never reused.
pub struct ResourceStorage<T> {
    slots: Vec<Slot<T>>,
    free_slots: Vec<u32>,
    len: usize,
}

impl<T> ResourceStorage<T> {
    /// Creates an empty resource storage.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            slots: Vec::new(),
            free_slots: Vec::new(),
            len: 0,
        }
    }

    /// Returns the number of currently stored resources.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns whether this storage contains no resources.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Inserts a resource and returns its typed runtime handle.
    ///
    /// Reuses a released slot when possible. Returns an error if the storage
    /// cannot represent another slot with a 32-bit index.
    pub fn insert(&mut self, value: T) -> Result<Handle<T>, ResourceStorageError> {
        self.len += 1;
        if let Some(index) = self.free_slots.pop() {
            let slot_index = usize::try_from(index).expect("u32 slot index must fit usize");
            let slot = &mut self.slots[slot_index];
            debug_assert!(slot.value.is_none());
            slot.value = Some(value);
            return Ok(Handle::from_raw_parts(index, slot.generation));
        }

        let index = match u32::try_from(self.slots.len()) {
            Ok(index) => index,
            Err(_) => {
                self.len -= 1;
                return Err(ResourceStorageError::CapacityExceeded);
            }
        };
        self.slots.push(Slot {
            generation: 0,
            value: Some(value),
        });
        Ok(Handle::from_raw_parts(index, 0))
    }

    /// Returns an immutable resource reference when the handle is current.
    #[must_use]
    pub fn get(&self, handle: Handle<T>) -> Option<&T> {
        let slot = self.slots.get(usize::try_from(handle.index).ok()?)?;
        (slot.generation == handle.generation)
            .then_some(slot.value.as_ref())
            .flatten()
    }

    /// Returns a mutable resource reference when the handle is current.
    pub fn get_mut(&mut self, handle: Handle<T>) -> Option<&mut T> {
        let slot = self.slots.get_mut(usize::try_from(handle.index).ok()?)?;
        (slot.generation == handle.generation)
            .then_some(slot.value.as_mut())
            .flatten()
    }

    /// Removes and returns a resource when the handle is current.
    ///
    /// The slot generation advances before the slot is made available for reuse.
    /// If advancing would overflow, the empty slot is permanently retired.
    pub fn remove(&mut self, handle: Handle<T>) -> Option<T> {
        let slot = self.slots.get_mut(usize::try_from(handle.index).ok()?)?;
        if slot.generation != handle.generation {
            return None;
        }
        let value = slot.value.take()?;
        self.len -= 1;
        if let Some(next_generation) = slot.generation.checked_add(1) {
            slot.generation = next_generation;
            self.free_slots.push(handle.index);
        }
        Some(value)
    }

    /// Returns whether the handle currently identifies a stored resource.
    #[must_use]
    pub fn contains(&self, handle: Handle<T>) -> bool {
        self.get(handle).is_some()
    }
}

impl<T> Default for ResourceStorage<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors produced by typed runtime resource storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceStorageError {
    /// The storage exhausted its representable 32-bit slot index space.
    CapacityExceeded,
}

impl fmt::Display for ResourceStorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityExceeded => {
                formatter.write_str("runtime resource storage capacity exceeded")
            }
        }
    }
}

impl std::error::Error for ResourceStorageError {}

#[cfg(test)]
mod tests {
    use super::{Handle, ResourceId, ResourceStorage, ResourceStorageError};
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    #[derive(Debug, PartialEq, Eq)]
    struct Texture(&'static str);

    #[derive(Debug)]
    struct Mesh;

    #[test]
    fn resource_id_is_unique_and_serializable() {
        let first = ResourceId::new();
        let second = ResourceId::new();

        assert_ne!(first, second);

        let serialized = toml::to_string(&ResourceIdDocument { id: first });
        assert!(serialized.is_ok());

        if let Ok(serialized) = serialized {
            let deserialized = toml::from_str::<ResourceIdDocument>(&serialized);
            assert!(matches!(deserialized, Ok(document) if document.id == first));
        }
    }

    #[test]
    fn resource_id_can_wrap_existing_uuid() {
        let uuid = Uuid::nil();
        let id = ResourceId::from_uuid(uuid);

        assert_eq!(id.as_uuid(), &uuid);
    }

    #[test]
    fn handle_equality_includes_generation() {
        let current = Handle::<Texture>::from_raw_parts(4, 2);
        let same = Handle::<Texture>::from_raw_parts(4, 2);
        let stale = Handle::<Texture>::from_raw_parts(4, 1);

        assert_eq!(current, same);
        assert_ne!(current, stale);
        assert_eq!(current.index(), 4);
        assert_eq!(current.generation(), 2);
    }

    #[test]
    fn handles_are_type_distinct() {
        let texture = Handle::<Texture>::from_raw_parts(1, 0);
        let mesh = Handle::<Mesh>::from_raw_parts(1, 0);

        assert_eq!(texture.index(), mesh.index());
        assert_eq!(texture.generation(), mesh.generation());
    }

    #[test]
    fn storage_inserts_and_resolves_typed_resources() {
        let mut storage = ResourceStorage::new();
        let handle = storage.insert(Texture("albedo")).unwrap();

        assert_eq!(storage.len(), 1);
        assert!(!storage.is_empty());
        assert_eq!(storage.get(handle), Some(&Texture("albedo")));
        assert!(storage.contains(handle));
    }

    #[test]
    fn storage_mutates_and_removes_current_resources() {
        let mut storage = ResourceStorage::new();
        let handle = storage.insert(Texture("before")).unwrap();

        storage.get_mut(handle).unwrap().0 = "after";
        assert_eq!(storage.get(handle), Some(&Texture("after")));
        assert_eq!(storage.remove(handle), Some(Texture("after")));
        assert!(storage.is_empty());
        assert!(!storage.contains(handle));
        assert_eq!(storage.remove(handle), None);
    }

    #[test]
    fn reusing_a_slot_invalidates_the_previous_handle() {
        let mut storage = ResourceStorage::new();
        let old_handle = storage.insert(Texture("old")).unwrap();
        assert_eq!(storage.remove(old_handle), Some(Texture("old")));

        let new_handle = storage.insert(Texture("new")).unwrap();

        assert_eq!(old_handle.index(), new_handle.index());
        assert_ne!(old_handle.generation(), new_handle.generation());
        assert_eq!(storage.get(old_handle), None);
        assert_eq!(storage.get(new_handle), Some(&Texture("new")));
    }

    #[test]
    fn storage_instances_keep_resource_types_separate() {
        let mut textures = ResourceStorage::new();
        let mut meshes = ResourceStorage::new();
        let texture = textures.insert(Texture("albedo")).unwrap();
        let mesh = meshes.insert(Mesh).unwrap();

        assert_eq!(texture.index(), mesh.index());
        assert_eq!(texture.generation(), mesh.generation());
        assert_eq!(textures.get(texture), Some(&Texture("albedo")));
        assert!(meshes.get(mesh).is_some());
    }

    #[test]
    fn default_storage_is_empty() {
        let storage = ResourceStorage::<Texture>::default();
        assert!(storage.is_empty());
        assert_eq!(storage.len(), 0);
    }

    #[test]
    fn storage_capacity_error_is_descriptive() {
        assert_eq!(
            ResourceStorageError::CapacityExceeded.to_string(),
            "runtime resource storage capacity exceeded"
        );
    }

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct ResourceIdDocument {
        id: ResourceId,
    }
}

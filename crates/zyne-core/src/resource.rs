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
/// occupants of that slot. A handle is valid only while a registry confirms
/// that both parts match. It is not stable across process runs or serialization.
pub struct Handle<T> {
    index: u32,
    generation: u32,
    marker: PhantomData<fn() -> T>,
}

impl<T> Handle<T> {
    /// Creates a handle from registry-owned slot coordinates.
    ///
    /// Registry implementations should be the only code constructing handles
    /// in normal engine usage.
    #[must_use]
    pub const fn from_raw_parts(index: u32, generation: u32) -> Self {
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

#[cfg(test)]
mod tests {
    use super::{Handle, ResourceId};
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    #[derive(Debug)]
    struct Texture;

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
            assert_eq!(deserialized.map(|document| document.id), Ok(ResourceIdDocument { id: first }.id));
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

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct ResourceIdDocument {
        id: ResourceId,
    }
}

//! Typed CPU-side asset cache and explicit resource lifecycle.
//!
//! A cache instance owns resources of exactly one `ResourceKind`. Persistent
//! metadata remains in `AssetRegistry`, while runtime values live in
//! `ResourceStorage`. Runtime handles are never persisted.

use std::collections::HashMap;
use std::fmt;

use zyne_core::{Handle, ResourceId, ResourceStorage, ResourceStorageError};

use crate::{AssetRegistry, ResourceKind};

/// Lifecycle state of one resource in an asset cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetState {
    /// Metadata exists, but no runtime value is resident.
    Unloaded,
    /// The caller has started loading and has not completed or failed it.
    Loading,
    /// A runtime value is resident and can be accessed through its handle.
    Loaded,
    /// The most recent load attempt failed.
    Failed,
}

struct CacheEntry<T> {
    state: AssetState,
    handle: Option<Handle<T>>,
    failure_reason: Option<String>,
}

/// A typed runtime cache for assets of one resource kind.
///
/// The cache does not perform filesystem I/O or import work itself. Callers
/// start a load, perform the import, then complete or fail that load. This
/// explicit two-step contract also provides a safe foundation for future
/// asynchronous loading without introducing concurrency prematurely.
pub struct AssetCache<T> {
    kind: ResourceKind,
    storage: ResourceStorage<T>,
    entries: HashMap<ResourceId, CacheEntry<T>>,
}

impl<T> AssetCache<T> {
    /// Creates an empty cache dedicated to one resource kind.
    #[must_use]
    pub fn new(kind: ResourceKind) -> Self {
        Self {
            kind,
            storage: ResourceStorage::new(),
            entries: HashMap::new(),
        }
    }

    /// Returns the resource kind accepted by this cache.
    #[must_use]
    pub const fn kind(&self) -> ResourceKind {
        self.kind
    }

    /// Returns the number of resident runtime values.
    #[must_use]
    pub fn len(&self) -> usize {
        self.storage.len()
    }

    /// Returns whether the cache has no resident runtime values.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.storage.is_empty()
    }

    /// Returns the lifecycle state for a registered resource.
    ///
    /// A registered resource not yet seen by this cache is Unloaded.
    ///
    /// # Errors
    ///
    /// Returns `ResourceNotFound` when the ID is absent from the registry, or
    /// `ResourceKindMismatch` when the metadata belongs to another cache kind.
    pub fn state(
        &self,
        registry: &AssetRegistry,
        id: ResourceId,
    ) -> Result<AssetState, AssetCacheError> {
        self.validate_metadata(registry, id)?;
        Ok(self.entry_state(id))
    }

    /// Marks an unloaded resource as loading.
    ///
    /// # Errors
    ///
    /// Returns an error when the resource is not registered for this cache or
    /// is not currently Unloaded.
    pub fn begin_loading(
        &mut self,
        registry: &AssetRegistry,
        id: ResourceId,
    ) -> Result<(), AssetCacheError> {
        self.validate_metadata(registry, id)?;
        let state = self.entry_state(id);
        if state != AssetState::Unloaded {
            return Err(AssetCacheError::InvalidState {
                id,
                state,
                operation: "begin loading",
            });
        }

        self.entries.insert(
            id,
            CacheEntry {
                state: AssetState::Loading,
                handle: None,
                failure_reason: None,
            },
        );
        Ok(())
    }

    /// Stores a successfully loaded value and returns its runtime handle.
    ///
    /// Repeated completion is rejected, so a resource ID cannot acquire
    /// duplicate resident values through this cache.
    ///
    /// # Errors
    ///
    /// Returns an error when metadata is missing or mismatched, the resource is
    /// not Loading, or runtime storage capacity is exhausted.
    pub fn complete_loading(
        &mut self,
        registry: &AssetRegistry,
        id: ResourceId,
        value: T,
    ) -> Result<Handle<T>, AssetCacheError> {
        self.validate_metadata(registry, id)?;
        let state = self.entry_state(id);
        if state != AssetState::Loading {
            return Err(AssetCacheError::InvalidState {
                id,
                state,
                operation: "complete loading",
            });
        }

        let handle = match self.storage.insert(value) {
            Ok(handle) => handle,
            Err(error) => {
                if let Some(entry) = self.entries.get_mut(&id) {
                    entry.state = AssetState::Failed;
                    entry.handle = None;
                    entry.failure_reason = Some(error.to_string());
                }
                return Err(AssetCacheError::Storage(error));
            }
        };

        if let Some(entry) = self.entries.get_mut(&id) {
            entry.state = AssetState::Loaded;
            entry.handle = Some(handle);
            entry.failure_reason = None;
        }
        Ok(handle)
    }

    /// Marks an in-progress load as failed and stores a diagnostic message.
    ///
    /// # Errors
    ///
    /// Returns an error when the resource is not Loading or the reason is blank.
    pub fn fail_loading(
        &mut self,
        id: ResourceId,
        reason: impl Into<String>,
    ) -> Result<(), AssetCacheError> {
        let state = self.entry_state(id);
        if state != AssetState::Loading {
            return Err(AssetCacheError::InvalidState {
                id,
                state,
                operation: "fail loading",
            });
        }

        let reason = reason.into();
        if reason.trim().is_empty() {
            return Err(AssetCacheError::EmptyFailureReason);
        }

        if let Some(entry) = self.entries.get_mut(&id) {
            entry.state = AssetState::Failed;
            entry.handle = None;
            entry.failure_reason = Some(reason.trim().to_owned());
        }
        Ok(())
    }

    /// Retries a failed resource by transitioning it back to Loading.
    ///
    /// # Errors
    ///
    /// Returns an error when metadata is missing or mismatched, or the resource
    /// is not currently Failed.
    pub fn retry_loading(
        &mut self,
        registry: &AssetRegistry,
        id: ResourceId,
    ) -> Result<(), AssetCacheError> {
        self.validate_metadata(registry, id)?;
        let state = self.entry_state(id);
        if state != AssetState::Failed {
            return Err(AssetCacheError::InvalidState {
                id,
                state,
                operation: "retry loading",
            });
        }

        if let Some(entry) = self.entries.get_mut(&id) {
            entry.state = AssetState::Loading;
            entry.failure_reason = None;
        }
        Ok(())
    }

    /// Releases a loaded value or clears a failed state.
    ///
    /// Registry metadata is intentionally preserved. Calling this for an
    /// already unloaded resource is idempotent. A loading resource cannot be
    /// unloaded until its load attempt has completed or failed.
    ///
    /// # Errors
    ///
    /// Returns `ResourceNotCached` when the cache has no lifecycle entry for
    /// the ID, `InvalidState` while loading, or `RuntimeValueMissing` if the
    /// internal loaded-handle invariant is broken.
    pub fn unload(&mut self, id: ResourceId) -> Result<(), AssetCacheError> {
        let entry = self
            .entries
            .get(&id)
            .ok_or(AssetCacheError::ResourceNotCached { id })?;
        match entry.state {
            AssetState::Loading => {
                return Err(AssetCacheError::InvalidState {
                    id,
                    state: AssetState::Loading,
                    operation: "unload",
                });
            }
            AssetState::Loaded => {
                let handle = entry
                    .handle
                    .ok_or(AssetCacheError::RuntimeValueMissing { id })?;
                if self.storage.remove(handle).is_none() {
                    return Err(AssetCacheError::RuntimeValueMissing { id });
                }
            }
            AssetState::Unloaded | AssetState::Failed => {}
        }

        if let Some(entry) = self.entries.get_mut(&id) {
            entry.state = AssetState::Unloaded;
            entry.handle = None;
            entry.failure_reason = None;
        }
        Ok(())
    }

    /// Returns the runtime handle for a loaded resource.
    #[must_use]
    pub fn handle(&self, id: ResourceId) -> Option<Handle<T>> {
        let entry = self.entries.get(&id)?;
        (entry.state == AssetState::Loaded)
            .then_some(entry.handle)
            .flatten()
    }

    /// Returns an immutable reference to a loaded runtime value.
    #[must_use]
    pub fn get(&self, id: ResourceId) -> Option<&T> {
        self.storage.get(self.handle(id)?)
    }

    /// Returns a mutable reference to a loaded runtime value.
    pub fn get_mut(&mut self, id: ResourceId) -> Option<&mut T> {
        let handle = self.handle(id)?;
        self.storage.get_mut(handle)
    }

    /// Returns the most recent failure reason, if the resource failed.
    #[must_use]
    pub fn failure_reason(&self, id: ResourceId) -> Option<&str> {
        self.entries.get(&id)?.failure_reason.as_deref()
    }

    fn entry_state(&self, id: ResourceId) -> AssetState {
        self.entries
            .get(&id)
            .map_or(AssetState::Unloaded, |entry| entry.state)
    }

    fn validate_metadata(
        &self,
        registry: &AssetRegistry,
        id: ResourceId,
    ) -> Result<(), AssetCacheError> {
        let metadata = registry
            .find(id)
            .ok_or(AssetCacheError::ResourceNotFound { id })?;
        if metadata.kind() != self.kind {
            return Err(AssetCacheError::ResourceKindMismatch {
                id,
                expected: self.kind,
                actual: metadata.kind(),
            });
        }
        Ok(())
    }
}

/// Read-only lifecycle view implemented by every typed asset cache.
pub trait AssetCacheStatus {
    /// Returns the resource kind owned by this cache.
    fn kind(&self) -> ResourceKind;

    /// Returns the lifecycle state for an ID.
    ///
    /// IDs not yet encountered by this cache are considered Unloaded.
    fn state(&self, id: ResourceId) -> AssetState;

    /// Returns the most recent load failure diagnostic, if any.
    fn failure_reason(&self, id: ResourceId) -> Option<&str>;
}

impl<T> AssetCacheStatus for AssetCache<T> {
    fn kind(&self) -> ResourceKind {
        AssetCache::kind(self)
    }

    fn state(&self, id: ResourceId) -> AssetState {
        self.entry_state(id)
    }

    fn failure_reason(&self, id: ResourceId) -> Option<&str> {
        AssetCache::failure_reason(self, id)
    }
}

/// Errors produced by typed asset cache operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetCacheError {
    /// The resource ID is absent from the persistent registry.
    ResourceNotFound { id: ResourceId },
    /// The resource metadata belongs to a different typed cache.
    ResourceKindMismatch {
        id: ResourceId,
        expected: ResourceKind,
        actual: ResourceKind,
    },
    /// The requested operation is not valid for the current lifecycle state.
    InvalidState {
        id: ResourceId,
        state: AssetState,
        operation: &'static str,
    },
    /// The cache has no lifecycle entry for the resource.
    ResourceNotCached { id: ResourceId },
    /// A loaded entry did not resolve to a live runtime value.
    RuntimeValueMissing { id: ResourceId },
    /// A failure diagnostic must contain non-whitespace text.
    EmptyFailureReason,
    /// The underlying typed runtime storage could not allocate a slot.
    Storage(ResourceStorageError),
}

impl fmt::Display for AssetCacheError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResourceNotFound { id } => {
                write!(formatter, "asset resource ID is not registered: {id}")
            }
            Self::ResourceKindMismatch {
                id,
                expected,
                actual,
            } => write!(
                formatter,
                "asset {id} has kind {actual:?}, but this cache accepts {expected:?}"
            ),
            Self::InvalidState {
                id,
                state,
                operation,
            } => write!(
                formatter,
                "cannot {operation} asset {id} while it is {state:?}"
            ),
            Self::ResourceNotCached { id } => {
                write!(formatter, "asset {id} has no cache lifecycle entry")
            }
            Self::RuntimeValueMissing { id } => {
                write!(formatter, "loaded runtime value is missing for asset {id}")
            }
            Self::EmptyFailureReason => {
                formatter.write_str("asset load failure reason must not be empty")
            }
            Self::Storage(error) => write!(formatter, "asset runtime storage failed: {error}"),
        }
    }
}

impl std::error::Error for AssetCacheError {}

#[cfg(test)]
mod tests {
    use zyne_core::ResourceId;

    use crate::{
        AssetMetadata, AssetRegistry, ImporterSettings, ProjectRelativePath, ResourceKind,
    };

    use super::{AssetCache, AssetCacheError, AssetState};

    #[derive(Debug, PartialEq, Eq)]
    struct TestAsset(&'static str);

    fn registry_with_asset(kind: ResourceKind) -> (AssetRegistry, ResourceId) {
        let id = ResourceId::new();
        let metadata = AssetMetadata::new(
            id,
            "Test Asset",
            kind,
            ProjectRelativePath::new("assets/test.asset").unwrap(),
            ImporterSettings::new("test-importer", 1).unwrap(),
        )
        .unwrap();
        let mut registry = AssetRegistry::new();
        registry.register(metadata).unwrap();
        (registry, id)
    }

    #[test]
    fn registered_asset_starts_unloaded() {
        let (registry, id) = registry_with_asset(ResourceKind::Mesh);
        let cache = AssetCache::<TestAsset>::new(ResourceKind::Mesh);

        assert_eq!(cache.state(&registry, id), Ok(AssetState::Unloaded));
        assert!(cache.is_empty());
    }

    #[test]
    fn rejects_unknown_resources_and_kind_mismatches() {
        let (registry, id) = registry_with_asset(ResourceKind::Texture);
        let cache = AssetCache::<TestAsset>::new(ResourceKind::Mesh);

        assert_eq!(
            cache.state(&registry, id),
            Err(AssetCacheError::ResourceKindMismatch {
                id,
                expected: ResourceKind::Mesh,
                actual: ResourceKind::Texture,
            })
        );
        assert!(matches!(
            cache.state(&registry, ResourceId::new()),
            Err(AssetCacheError::ResourceNotFound { .. })
        ));
    }

    #[test]
    fn loading_and_completion_cache_one_runtime_value() {
        let (registry, id) = registry_with_asset(ResourceKind::Mesh);
        let mut cache = AssetCache::new(ResourceKind::Mesh);

        cache.begin_loading(&registry, id).unwrap();
        assert_eq!(cache.state(&registry, id), Ok(AssetState::Loading));
        assert_eq!(
            cache.begin_loading(&registry, id),
            Err(AssetCacheError::InvalidState {
                id,
                state: AssetState::Loading,
                operation: "begin loading",
            })
        );

        let handle = cache
            .complete_loading(&registry, id, TestAsset("mesh"))
            .unwrap();
        assert_eq!(cache.state(&registry, id), Ok(AssetState::Loaded));
        assert_eq!(cache.handle(id), Some(handle));
        assert_eq!(cache.get(id), Some(&TestAsset("mesh")));
        assert_eq!(cache.len(), 1);
        assert_eq!(
            cache.complete_loading(&registry, id, TestAsset("duplicate")),
            Err(AssetCacheError::InvalidState {
                id,
                state: AssetState::Loaded,
                operation: "complete loading",
            })
        );
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn failed_load_retains_reason_and_can_be_retried() {
        let (registry, id) = registry_with_asset(ResourceKind::Texture);
        let mut cache = AssetCache::new(ResourceKind::Texture);

        cache.begin_loading(&registry, id).unwrap();
        cache
            .fail_loading(id, "image decoder rejected data")
            .unwrap();
        assert_eq!(cache.state(&registry, id), Ok(AssetState::Failed));
        assert_eq!(
            cache.failure_reason(id),
            Some("image decoder rejected data")
        );
        assert_eq!(cache.get(id), None);

        cache.retry_loading(&registry, id).unwrap();
        assert_eq!(cache.state(&registry, id), Ok(AssetState::Loading));
        assert_eq!(cache.failure_reason(id), None);

        cache
            .complete_loading(&registry, id, TestAsset("decoded texture"))
            .unwrap();
        assert_eq!(cache.state(&registry, id), Ok(AssetState::Loaded));
    }

    #[test]
    fn rejects_empty_failure_reasons_and_invalid_retries() {
        let (registry, id) = registry_with_asset(ResourceKind::Model);
        let mut cache = AssetCache::<TestAsset>::new(ResourceKind::Model);

        assert_eq!(
            cache.retry_loading(&registry, id),
            Err(AssetCacheError::InvalidState {
                id,
                state: AssetState::Unloaded,
                operation: "retry loading",
            })
        );
        cache.begin_loading(&registry, id).unwrap();
        assert_eq!(
            cache.fail_loading(id, "  "),
            Err(AssetCacheError::EmptyFailureReason)
        );
        assert_eq!(cache.state(&registry, id), Ok(AssetState::Loading));
    }

    #[test]
    fn unload_invalidates_handle_but_preserves_registry_metadata() {
        let (registry, id) = registry_with_asset(ResourceKind::Mesh);
        let mut cache = AssetCache::new(ResourceKind::Mesh);

        cache.begin_loading(&registry, id).unwrap();
        let old_handle = cache
            .complete_loading(&registry, id, TestAsset("resident"))
            .unwrap();
        cache.unload(id).unwrap();

        assert_eq!(cache.state(&registry, id), Ok(AssetState::Unloaded));
        assert_eq!(cache.get(id), None);
        assert_eq!(cache.handle(id), None);
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
        assert!(registry.find(id).is_some());

        cache.begin_loading(&registry, id).unwrap();
        let new_handle = cache
            .complete_loading(&registry, id, TestAsset("reloaded"))
            .unwrap();
        assert_eq!(cache.storage.get(old_handle), None);
        assert_ne!(old_handle.generation(), new_handle.generation());
        assert_eq!(cache.get(id), Some(&TestAsset("reloaded")));
    }

    #[test]
    fn unload_clears_failed_state_and_is_idempotent_when_unloaded() {
        let (registry, id) = registry_with_asset(ResourceKind::Audio);
        let mut cache = AssetCache::<TestAsset>::new(ResourceKind::Audio);

        cache.begin_loading(&registry, id).unwrap();
        cache.fail_loading(id, "decoder error").unwrap();
        cache.unload(id).unwrap();

        assert_eq!(cache.state(&registry, id), Ok(AssetState::Unloaded));
        assert_eq!(cache.failure_reason(id), None);
        cache.unload(id).unwrap();
    }

    #[test]
    fn loading_resource_cannot_be_unloaded_mid_transition() {
        let (registry, id) = registry_with_asset(ResourceKind::Material);
        let mut cache = AssetCache::<TestAsset>::new(ResourceKind::Material);

        cache.begin_loading(&registry, id).unwrap();
        assert_eq!(
            cache.unload(id),
            Err(AssetCacheError::InvalidState {
                id,
                state: AssetState::Loading,
                operation: "unload",
            })
        );
        assert_eq!(cache.state(&registry, id), Ok(AssetState::Loading));
    }

    #[test]
    fn mutable_access_only_affects_the_requested_cached_asset() {
        let (mut registry, first_id) = registry_with_asset(ResourceKind::Mesh);
        let second_id = ResourceId::new();
        registry
            .register(
                AssetMetadata::new(
                    second_id,
                    "Second Asset",
                    ResourceKind::Mesh,
                    ProjectRelativePath::new("assets/second.asset").unwrap(),
                    ImporterSettings::new("test-importer", 1).unwrap(),
                )
                .unwrap(),
            )
            .unwrap();

        let mut cache = AssetCache::new(ResourceKind::Mesh);
        for (id, value) in [
            (first_id, TestAsset("first")),
            (second_id, TestAsset("second")),
        ] {
            cache.begin_loading(&registry, id).unwrap();
            cache.complete_loading(&registry, id, value).unwrap();
        }

        cache.get_mut(first_id).unwrap().0 = "updated";
        assert_eq!(cache.get(first_id), Some(&TestAsset("updated")));
        assert_eq!(cache.get(second_id), Some(&TestAsset("second")));
        assert_eq!(cache.len(), 2);
    }
}

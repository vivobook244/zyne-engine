/// Semantic version of the ZYNE engine API/runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EngineVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl EngineVersion {
    /// Version of this workspace package.
    pub const CURRENT: Self = Self::new(0, 1, 0);

    #[must_use]
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl core::fmt::Display for EngineVersion {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[cfg(test)]
mod tests {
    use super::EngineVersion;

    #[test]
    fn displays_semantic_version() {
        assert_eq!(EngineVersion::CURRENT.to_string(), "0.1.0");
    }
}

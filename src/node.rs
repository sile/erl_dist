//! Node related components.
use crate::DistributionFlags;

/// Local node information.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LocalNode {
    /// Node name.
    pub name: NodeName,

    /// Distribution flags.
    pub flags: DistributionFlags,

    /// Incarnation identifier.
    pub creation: Creation,
}

impl LocalNode {
    /// Makes a new [`LocalNode`] instance with the default distribution flags.
    pub fn new(name: NodeName, creation: Creation) -> Self {
        Self {
            name,
            flags: Default::default(),
            creation,
        }
    }
}

/// Peer node information.
///
/// This is similar to [`LocalNode`] but the `creation` field can be `None` as older nodes may not provide that information during the handshake.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PeerNode {
    /// Node name.
    pub name: NodeName,

    /// Distribution flags.
    pub flags: DistributionFlags,

    /// Incarnation identifier.
    pub creation: Option<Creation>,
}

/// Errors that can occur while parsing node names.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum NodeNameError {
    #[error("node name length must be less than 256, but got {size} characters")]
    TooLongName { size: usize },

    #[error("the name part of a node name is empty")]
    EmptyName,

    #[error("the host part of a node name is empty")]
    EmptyHost,

    #[error("node name must contain an '@' character")]
    MissingAtmark,
}

/// Full node name with the format "{NAME}@{HOST}".
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeName {
    name: String,
    host: String,
}

impl NodeName {
    /// Makes a new [`NodeName`] instance.
    pub fn new(name: &str, host: &str) -> Result<Self, NodeNameError> {
        let size = name.len() + 1 + host.len();
        if size > 255 {
            Err(NodeNameError::TooLongName { size })
        } else if name.is_empty() {
            Err(NodeNameError::EmptyName)
        } else if host.is_empty() {
            Err(NodeNameError::EmptyHost)
        } else {
            Ok(Self {
                name: name.to_owned(),
                host: host.to_owned(),
            })
        }
    }

    /// Returns the name part.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the host part.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Returns the name length.
    ///
    /// Note that the result will never be less than `3`.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.name.len() + 1 + self.host.len()
    }
}

impl std::str::FromStr for NodeName {
    type Err = NodeNameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut tokens = s.splitn(2, '@');
        if let (Some(name), Some(host)) = (tokens.next(), tokens.next()) {
            Self::new(name, host)
        } else {
            Err(NodeNameError::MissingAtmark)
        }
    }
}

impl std::fmt::Display for NodeName {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}@{}", self.name, self.host)
    }
}

/// Incarnation identifier of a node.
///
/// [`Creation`] is used by the node to create its pids, ports and references.
/// If the node restarts, the value of [`Creation`] will be changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Creation(u32);

impl Creation {
    /// Makes a new [`Creation`] instance.
    pub const fn new(n: u32) -> Self {
        Self(n)
    }

    /// Makes a new [`Creation`] instance having a random value.
    pub fn random() -> Self {
        Self(rand::random())
    }

    /// Gets the value.
    pub const fn get(self) -> u32 {
        self.0
    }
}

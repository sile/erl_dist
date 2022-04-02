use crate::capability::DistributionFlags;

#[derive(Debug, thiserror::Error)]
pub enum NodeNameError {
    #[error("node name length must be less than 256, but got {size} characters")]
    TooLongName { size: usize },

    #[error("node name must contain an '@' character")]
    MissingAtmark,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PeerNode {
    pub name: NodeName,
    pub flags: DistributionFlags,
    pub creation: Option<Creation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LocalNode {
    pub name: NodeName,
    pub flags: DistributionFlags,
    pub creation: Creation,
}

impl LocalNode {
    pub fn new(name: NodeName, creation: Creation) -> Self {
        Self {
            name,
            flags: Default::default(),
            creation,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeName {
    name: String,
    host: String,
}

impl NodeName {
    pub fn new(name: &str, host: &str) -> Result<Self, NodeNameError> {
        let size = name.len() + 1 + host.len();
        if size > 255 {
            Err(NodeNameError::TooLongName { size })
        } else {
            Ok(Self {
                name: name.to_owned(),
                host: host.to_owned(),
            })
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        self.name.len() + 1 + self.host.len()
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn host(&self) -> &str {
        &self.host
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

/// Type of a distributed node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum NodeType {
    /// Hidden node (C-node).
    Hidden = 72,

    /// Normal Erlang node.
    Normal = 77,
}

impl TryFrom<u8> for NodeType {
    type Error = crate::epmd::EpmdError; // TODO

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            72 => Ok(Self::Hidden),
            77 => Ok(Self::Normal),
            _ => Err(crate::epmd::EpmdError::UnknownNodeType { value }),
        }
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

    pub fn random() -> Self {
        Self(rand::random())
    }

    /// Gets the value.
    pub const fn get(self) -> u32 {
        self.0
    }
}

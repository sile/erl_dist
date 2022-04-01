#[derive(Debug, thiserror::Error)]
pub enum NodeNameError {
    #[error("node name length must be less than 256, but got {size} characters")]
    TooLongName { size: usize },

    #[error("node name must contain an '@' character")]
    MissingAtmark,
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

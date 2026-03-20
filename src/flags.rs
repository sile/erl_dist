/// Distribution flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DistributionFlags(u64);

impl DistributionFlags {
    /// The node is to be published and part of the global namespace.
    pub const PUBLISHED: Self = Self(0x01);

    /// The node implements an atom cache (obsolete).
    pub const ATOM_CACHE: Self = Self(0x02);

    /// The node implements extended (3 × 32 bits) references (required).
    ///
    /// [NOTE] This flag is mandatory. If not present, the connection is refused.
    pub const EXTENDED_REFERENCES: Self = Self(0x04);

    /// The node implements distributed process monitoring.
    ///
    /// [NOTE] This flag will become mandatory in OTP 25.
    pub const DIST_MONITOR: Self = Self(0x08);

    /// The node uses separate tag for funs (lambdas) in the distribution protocol.
    pub const FUN_TAGS: Self = Self(0x10);

    /// The node implements distributed named process monitoring.
    pub const DIST_MONITOR_NAME: Self = Self(0x20);

    /// The (hidden) node implements atom cache (obsolete).
    pub const HIDDEN_ATOM_CACHE: Self = Self(0x40);

    /// The node understands new fun tags.
    ///
    /// [NOTE] This flag is mandatory. If not present, the connection is refused.
    pub const NEW_FUN_TAGS: Self = Self(0x80);

    /// The node can handle extended pids and ports (required).
    ///
    /// [NOTE] This flag is mandatory. If not present, the connection is refused.
    pub const EXTENDED_PIDS_PORTS: Self = Self(0x100);

    /// This node understands `EXPORT_EXT` tag.
    ///
    /// [NOTE] This flag will become mandatory in OTP 25.
    pub const EXPORT_PTR_TAG: Self = Self(0x200);

    /// The node understands bit binaries.
    ///
    /// [NOTE] This flag will become mandatory in OTP 25.
    pub const BIT_BINARIES: Self = Self(0x400);

    /// The node understandss new float format.
    ///
    /// [NOTE] This flag will become mandatory in OTP 25.
    pub const NEW_FLOATS: Self = Self(0x800);

    /// This node allows unicode characters in I/O operations.
    pub const UNICODE_IO: Self = Self(0x1000);

    /// The node implements atom cache in distribution header.
    ///
    /// Note that currently `erl_dist` can not handle distribution headers.
    pub const DIST_HDR_ATOM_CACHE: Self = Self(0x2000);

    /// The node understands the `SMALL_ATOM_EXT` tag.
    pub const SMALL_ATOM_TAGS: Self = Self(0x4000);

    /// The node understands UTF-8 encoded atoms.
    ///
    /// [NOTE] This flag is mandatory. If not present, the connection is refused.
    pub const UTF8_ATOMS: Self = Self(0x10000);

    /// The node understands maps.
    ///
    /// [NOTE] This flag will become mandatory in OTP 25.
    pub const MAP_TAGS: Self = Self(0x20000);

    /// The node understands big node creation tags `NEW_PID_EXT`, `NEW_PORT_EXT` and `NEWER_REFERENCE_EXT`.
    ///
    /// [NOTE] This flag is mandatory. If not present, the connection is refused.
    pub const BIG_CREATION: Self = Self(0x40000);

    /// Use the `SEND_SENDER` control message instead of the `SEND` control message and use the `SEND_SENDER_TT` control message instead of the `SEND_TT` control message.
    pub const SEND_SENDER: Self = Self(0x80000);

    /// The node understands any term as the seqtrace label.
    pub const BIG_SEQTRACE_LABELS: Self = Self(0x100000);

    /// Use the `PAYLOAD_EXIT`, `PAYLOAD_EXIT_TT`, `PAYLOAD_EXIT2`, `PAYLOAD_EXIT2_TT` and `PAYLOAD_MONITOR_P_EXIT` control messages instead of the non-PAYLOAD variants.
    pub const EXIT_PAYLOAD: Self = Self(0x400000);

    /// Use fragmented distribution messages to send large messages.
    pub const FRAGMENTS: Self = Self(0x800000);

    /// The node supports the new connection setup handshake (version 6) introduced in OTP 23.
    pub const HANDSHAKE_23: Self = Self(0x1000000);

    /// Use the new link protocol.
    ///
    /// Unless both nodes have set the `UNLINK_ID` flag, the old link protocol will be used as a fallback.
    ///
    /// [NOTE] This flag will become mandatory in OTP 25.
    pub const UNLINK_ID: Self = Self(0x2000000);

    /// Set if the `SPAWN_REQUEST`, `SPAWN_REQUEST_TT`, `SPAWN_REPLY`, `SPAWN_REPLY_TT` control messages are supported.
    pub const SPAWN: Self = Self(1 << 32);

    /// Dynamic node name.
    ///
    /// This is not a capability but rather used as a request from the connecting node
    /// to receive its node name from the accepting node as part of the handshake.
    pub const NAME_ME: Self = Self(1 << 33);

    /// The node accepts a larger amount of data in pids, ports and references (node container types version 4).
    ///
    /// In the pid case full 32-bit `ID` and `Serial` fields in `NEW_PID_EXT`,
    /// in the port case a 64-bit integer in `V4_PORT_EXT`, and in the reference case up to 5 32-bit ID words are
    /// now accepted in `NEWER_REFERENCE_EXT`.
    /// Introduced in OTP 24.
    ///
    /// [NOTE] This flag will become mandatory in OTP 26.
    pub const V4_NC: Self = Self(1 << 34);

    /// The node supports process alias and can by this handle the `ALIAS_SEND` and `ALIAS_SEND_TT` control messages.
    ///
    /// Introduced in OTP 24.
    pub const ALIAS: Self = Self(1 << 35);

    /// The node supports all capabilities that are mandatory in OTP 25.
    ///
    /// Introduced in OTP 25.
    /// [NOTE] This flag will become mandatory in OTP 27.
    pub const MANDATORY_25_DIGEST: Self = Self(1 << 36);

    /// Returns the raw bits value.
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// Creates flags from raw bits.
    pub const fn from_bits_truncate(bits: u64) -> Self {
        Self(bits)
    }

    /// Returns `true` if all flags in `other` are contained within `self`.
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Makes a new [`DistributionFlags`] with the default flags.
    ///
    /// This is equivalent to the following code:
    ///
    /// ```
    /// # use erl_dist::DistributionFlags;
    /// DistributionFlags::mandatory();
    /// ```
    pub fn new() -> Self {
        Self::mandatory()
    }

    /// Gets the mandatory flags (in OTP 26).
    pub fn mandatory() -> Self {
        Self::EXTENDED_REFERENCES
            | Self::FUN_TAGS
            | Self::NEW_FUN_TAGS
            | Self::EXTENDED_PIDS_PORTS
            | Self::EXPORT_PTR_TAG
            | Self::BIT_BINARIES
            | Self::NEW_FLOATS
            | Self::UTF8_ATOMS
            | Self::MAP_TAGS
            | Self::BIG_CREATION
            | Self::HANDSHAKE_23
            | Self::MANDATORY_25_DIGEST
            | Self::UNLINK_ID
            | Self::V4_NC
    }
}

impl Default for DistributionFlags {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::BitOr for DistributionFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for DistributionFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl std::ops::BitAnd for DistributionFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_bit_values() {
        assert_eq!(DistributionFlags::PUBLISHED.bits(), 0x01);
        assert_eq!(DistributionFlags::EXTENDED_REFERENCES.bits(), 0x04);
        assert_eq!(DistributionFlags::UTF8_ATOMS.bits(), 0x10000);
        assert_eq!(DistributionFlags::HANDSHAKE_23.bits(), 0x1000000);
        assert_eq!(DistributionFlags::SPAWN.bits(), 1 << 32);
        assert_eq!(DistributionFlags::NAME_ME.bits(), 1 << 33);
        assert_eq!(DistributionFlags::V4_NC.bits(), 1 << 34);
        assert_eq!(DistributionFlags::ALIAS.bits(), 1 << 35);
        assert_eq!(DistributionFlags::MANDATORY_25_DIGEST.bits(), 1 << 36);
    }

    #[test]
    fn from_bits_truncate_roundtrip() {
        let flags = DistributionFlags::mandatory();
        let bits = flags.bits();
        assert_eq!(DistributionFlags::from_bits_truncate(bits), flags);
    }

    #[test]
    fn from_bits_truncate_preserves_unknown_bits() {
        let raw = 0xFFFF_FFFF_FFFF_FFFF;
        assert_eq!(DistributionFlags::from_bits_truncate(raw).bits(), raw);
    }

    #[test]
    fn contains_single_flag() {
        let flags = DistributionFlags::PUBLISHED | DistributionFlags::UTF8_ATOMS;
        assert!(flags.contains(DistributionFlags::PUBLISHED));
        assert!(flags.contains(DistributionFlags::UTF8_ATOMS));
        assert!(!flags.contains(DistributionFlags::SPAWN));
    }

    #[test]
    fn contains_multiple_flags() {
        let flags = DistributionFlags::mandatory();
        let subset = DistributionFlags::EXTENDED_REFERENCES | DistributionFlags::UTF8_ATOMS;
        assert!(flags.contains(subset));
    }

    #[test]
    fn contains_empty_is_always_true() {
        let empty = DistributionFlags::from_bits_truncate(0);
        assert!(DistributionFlags::PUBLISHED.contains(empty));
        assert!(empty.contains(empty));
    }

    #[test]
    fn bitor_combines_flags() {
        let a = DistributionFlags::PUBLISHED;
        let b = DistributionFlags::UTF8_ATOMS;
        let combined = a | b;
        assert_eq!(combined.bits(), 0x01 | 0x10000);
        assert!(combined.contains(a));
        assert!(combined.contains(b));
    }

    #[test]
    fn bitor_assign_combines_flags() {
        let mut flags = DistributionFlags::PUBLISHED;
        flags |= DistributionFlags::NAME_ME;
        assert!(flags.contains(DistributionFlags::PUBLISHED));
        assert!(flags.contains(DistributionFlags::NAME_ME));
    }

    #[test]
    fn bitand_intersects_flags() {
        let a = DistributionFlags::PUBLISHED | DistributionFlags::UTF8_ATOMS;
        let b = DistributionFlags::UTF8_ATOMS | DistributionFlags::SPAWN;
        let intersection = a & b;
        assert!(intersection.contains(DistributionFlags::UTF8_ATOMS));
        assert!(!intersection.contains(DistributionFlags::PUBLISHED));
        assert!(!intersection.contains(DistributionFlags::SPAWN));
    }

    #[test]
    fn default_equals_mandatory() {
        assert_eq!(DistributionFlags::default(), DistributionFlags::mandatory());
        assert_eq!(DistributionFlags::new(), DistributionFlags::mandatory());
    }

    #[test]
    fn mandatory_contains_expected_flags() {
        let m = DistributionFlags::mandatory();
        assert!(m.contains(DistributionFlags::EXTENDED_REFERENCES));
        assert!(m.contains(DistributionFlags::FUN_TAGS));
        assert!(m.contains(DistributionFlags::NEW_FUN_TAGS));
        assert!(m.contains(DistributionFlags::EXTENDED_PIDS_PORTS));
        assert!(m.contains(DistributionFlags::EXPORT_PTR_TAG));
        assert!(m.contains(DistributionFlags::BIT_BINARIES));
        assert!(m.contains(DistributionFlags::NEW_FLOATS));
        assert!(m.contains(DistributionFlags::UTF8_ATOMS));
        assert!(m.contains(DistributionFlags::MAP_TAGS));
        assert!(m.contains(DistributionFlags::BIG_CREATION));
        assert!(m.contains(DistributionFlags::HANDSHAKE_23));
        assert!(m.contains(DistributionFlags::MANDATORY_25_DIGEST));
        assert!(m.contains(DistributionFlags::UNLINK_ID));
        assert!(m.contains(DistributionFlags::V4_NC));

        // Not in mandatory set.
        assert!(!m.contains(DistributionFlags::PUBLISHED));
        assert!(!m.contains(DistributionFlags::NAME_ME));
        assert!(!m.contains(DistributionFlags::SPAWN));
        assert!(!m.contains(DistributionFlags::ALIAS));
    }

    #[test]
    fn high_and_low_bits_coexist() {
        // Verify that flags spanning both u32 halves work correctly together.
        let flags = DistributionFlags::PUBLISHED | DistributionFlags::SPAWN; // bit 0 + bit 32
        assert_eq!(flags.bits(), 0x01 | (1 << 32));
        assert!(flags.contains(DistributionFlags::PUBLISHED));
        assert!(flags.contains(DistributionFlags::SPAWN));

        // Truncating to u32 should lose high bits.
        let low = flags.bits() as u32;
        assert_eq!(low, 0x01);
    }

    #[test]
    fn clone_and_copy() {
        let a = DistributionFlags::mandatory();
        let b = a;
        let c = a.clone();
        assert_eq!(a, b);
        assert_eq!(a, c);
    }

    #[test]
    fn debug_format() {
        // Just ensure it doesn't panic.
        let _ = format!("{:?}", DistributionFlags::mandatory());
    }

    #[test]
    fn hash_consistent() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(DistributionFlags::PUBLISHED);
        set.insert(DistributionFlags::PUBLISHED);
        assert_eq!(set.len(), 1);

        set.insert(DistributionFlags::UTF8_ATOMS);
        assert_eq!(set.len(), 2);
    }
}

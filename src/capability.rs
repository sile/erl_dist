pub const LOWEST_DISTRIBUTION_PROTOCOL_VERSION: u16 = 5;
pub const HIGHEST_DISTRIBUTION_PROTOCOL_VERSION: u16 = 6;

bitflags::bitflags! {
    // capability flags
    /// Distribution flags.
    pub struct DistributionFlags: u64 {
        /// The node is to be published and part of the global namespace.
        const DFLAG_PUBLISHED = 0x01;

        /// The node implements an atom cache (obsolete).
        const DFLAG_ATOM_CACHE = 0x02;

        /// The node implements extended (3 × 32 bits) references (required).
        ///
        /// [NOTE] This flag is mandatory. If not present, the connection is refused.
        const DFLAG_EXTENDED_REFERENCES = 0x04;

        /// The node implements distributed process monitoring.
        ///
        /// [NOTE] This flag will become mandatory in OTP 25.
        const DFLAG_DIST_MONITOR = 0x08;

        /// The node uses separate tag for funs (lambdas) in the distribution protocol.
        const DFLAG_FUN_TAGS = 0x10;

        /// The node implements distributed named process monitoring.
        const DFLAG_DIST_MONITOR_NAME = 0x20;

        /// The (hidden) node implements atom cache (obsolete).
        const DFLAG_HIDDEN_ATOM_CACHE = 0x40;

        /// The node understands new fun tags.
        ///
        /// [NOTE] This flag is mandatory. If not present, the connection is refused.
        const DFLAG_NEW_FUN_TAGS = 0x80;

        /// The node can handle extended pids and ports (required).
        ///
        /// [NOTE] This flag is mandatory. If not present, the connection is refused.
        const DFLAG_EXTENDED_PIDS_PORTS = 0x100;

        /// This node understands `EXPORT_EXT` tag.
        ///
        /// [NOTE] This flag will become mandatory in OTP 25.
        const DFLAG_EXPORT_PTR_TAG = 0x200;

        /// The node understands bit binaries.
        ///
        /// [NOTE] This flag will become mandatory in OTP 25.
        const DFLAG_BIT_BINARIES = 0x400;

        /// The node understandss new float format.
        ///
        /// [NOTE] This flag will become mandatory in OTP 25.
        const DFLAG_NEW_FLOATS = 0x800;

        /// This node allows unicode characters in I/O operations.
        const DFLAG_UNICODE_IO = 0x1000;

        /// The node implements atom cache in distribution header.
        ///
        /// Note that currently `erl_dist` can not handle distribution headers.
        const DFLAG_DIST_HDR_ATOM_CACHE = 0x2000;

        /// The node understands the `SMALL_ATOM_EXT` tag.
        const DFLAG_SMALL_ATOM_TAGS = 0x4000;

        /// The node understands UTF-8 encoded atoms.
        ///
        /// [NOTE] This flag is mandatory. If not present, the connection is refused.
        const DFLAG_UTF8_ATOMS = 0x10000;

        /// The node understands maps.
        ///
        /// [NOTE] This flag will become mandatory in OTP 25.
        const DFLAG_MAP_TAGS = 0x20000;

        /// The node understands big node creation tags `NEW_PID_EXT`, `NEW_PORT_EXT` and `NEWER_REFERENCE_EXT`.
        ///
        /// [NOTE] This flag is mandatory. If not present, the connection is refused.
        const DFLAG_BIG_CREATION = 0x40000;

        /// Use the `SEND_SENDER` control message instead of the `SEND` control message and use the `SEND_SENDER_TT` control message instead of the `SEND_TT` control message.
        const DFLAG_SEND_SENDER = 0x80000;

        /// The node understands any term as the seqtrace label.
        const DFLAG_BIG_SEQTRACE_LABELS = 0x100000;

        /// Use the `PAYLOAD_EXIT`, `PAYLOAD_EXIT_TT`, `PAYLOAD_EXIT2`, `PAYLOAD_EXIT2_TT` and `PAYLOAD_MONITOR_P_EXIT` control messages instead of the non-PAYLOAD variants.
        const DFLAG_EXIT_PAYLOAD = 0x400000;

        /// Use fragmented distribution messages to send large messages.
        const DFLAG_FRAGMENTS = 0x800000;

        /// The node supports the new connection setup handshake (version 6) introduced in OTP 23.
        const DFLAG_HANDSHAKE_23 = 0x1000000;

        /// Use the new link protocol.
        ///
        /// Unless both nodes have set the `DFLAG_UNLINK_ID` flag, the old link protocol will be used as a fallback.
        ///
        /// [NOTE] This flag will become mandatory in OTP 25.
        const DFLAG_UNLINK_ID = 0x2000000;

        /// Set if the `SPAWN_REQUEST`, `SPAWN_REQUEST_TT`, `SPAWN_REPLY`, `SPAWN_REPLY_TT` control messages are supported.
        const DFLAG_SPAWN = 1 << 32;

        /// Dynamic node name.
        ///
        /// This is not a capability but rather used as a request from the connecting node
        /// to receive its node name from the accepting node as part of the handshake.
        const DFLAG_NAME_ME = 1 << 33;

        /// The node accepts a larger amount of data in pids, ports and references (node container types version 4).
        ///
        /// In the pid case full 32-bit `ID` and `Serial` fields in `NEW_PID_EXT`,
        /// in the port case a 64-bit integer in `V4_PORT_EXT`, and in the reference case up to 5 32-bit ID words are
        /// now accepted in `NEWER_REFERENCE_EXT`.
        /// Introduced in OTP 24.
        ///
        /// [NOTE] This flag will become mandatory in OTP 26.
        const DFLAG_V4_NC = 1 << 34;

        /// The node supports process alias and can by this handle the `ALIAS_SEND` and `ALIAS_SEND_TT` control messages.
        ///
        /// Introduced in OTP 24.
        const DFLAG_ALIAS = 1 << 35;
    }
}

impl Default for DistributionFlags {
    // TODO: update
    /// The default distribution flags.
    ///
    /// This is equivalent to the following code:
    ///
    /// ```no_run
    /// # use erl_dist::capability::DistributionFlags;
    /// DistributionFlags::DFLAG_EXTENDED_REFERENCES | DistributionFlags::DFLAG_EXTENDED_PIDS_PORTS |
    /// DistributionFlags::DFLAG_FUN_TAGS | DistributionFlags::DFLAG_NEW_FUN_TAGS |
    /// DistributionFlags::DFLAG_EXPORT_PTR_TAG | DistributionFlags::DFLAG_BIT_BINARIES |
    /// DistributionFlags::DFLAG_NEW_FLOATS | DistributionFlags::DFLAG_SMALL_ATOM_TAGS |
    /// DistributionFlags::DFLAG_UTF8_ATOMS | DistributionFlags::DFLAG_MAP_TAGS
    /// # ;
    /// ```
    fn default() -> Self {
        Self::DFLAG_EXTENDED_REFERENCES
            | Self::DFLAG_EXTENDED_PIDS_PORTS
            | Self::DFLAG_FUN_TAGS
            | Self::DFLAG_NEW_FUN_TAGS
            | Self::DFLAG_EXPORT_PTR_TAG
            | Self::DFLAG_BIT_BINARIES
            | Self::DFLAG_NEW_FLOATS
            | Self::DFLAG_SMALL_ATOM_TAGS
            | Self::DFLAG_UTF8_ATOMS
            | Self::DFLAG_MAP_TAGS
            | Self::DFLAG_HANDSHAKE_23
            | Self::DFLAG_BIG_CREATION
    }
}

// TODO: mandatory list

/// Minium amount of data within a single udp packet
const PROTOCOL_MINIMUM_MTU: usize = 576;
/// Maximum amount of data within a single udp packet
const PROTOCOL_MAXIMUM_MTU: usize = 4096;
/// Maximum possible commands within one udp packet
const PROTOCOL_MAXIMUM_PACKET_COMMANDS: usize = 32;
/// Minimum allowed window size
const PROTOCOL_MINIMUM_WINDOW_SIZE: usize = 4096;
/// Maximum allowed window size
const PROTOCOL_MAXIMUM_WINDOW_SIZE: usize = 65536;
/// Minimium allowed channel count
const PROTOCOL_MINIMUM_CHANNEL_COUNT: usize = 1;
/// Maximum allowed channel count
const PROTOCOL_MAXIMUM_CHANNEL_COUNT: usize = 255;
/// Maximum allowed peer id
const PROTOCOL_MAXIMUM_PEER_ID: usize = 0xFFF;
/// Maximum allowed fragmentation count
const PROTOCOL_MAXIMUM_FRAGMENT_COUNT: usize = 1024 * 1024;

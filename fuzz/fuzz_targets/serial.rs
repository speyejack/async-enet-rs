#![no_main]

use bytes::BytesMut;
use enet::{
    net::{deserializer::EnetDeserializer, serializer::EnetSerializer},
    protocol::SendUnreliableCommand,
};
use libfuzzer_sys::fuzz_target;
use serde::{de::Deserialize, ser::Serialize};

fuzz_target!(|data: SendUnreliableCommand| {
    roundtrip(data);
});

fn roundtrip(protocol: SendUnreliableCommand) -> () {
    let mut buff = BytesMut::zeroed(100);
    let mut ser = EnetSerializer {
        output: &mut buff[..],
        size: 0,
    };

    protocol.serialize(&mut ser);

    let size = ser.size;
    let buf = buff.freeze();

    let mut deser = EnetDeserializer {
        input: &buf[..size],
        consumed: 0,
    };

    let out = SendUnreliableCommand::deserialize(&mut deser).unwrap();
    assert_eq!(size, deser.consumed);
    assert_eq!(protocol, out);
}

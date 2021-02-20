use crate::buffers::*;
use crate::configs::Configs;
use crate::error::Error;
use crate::core::*;
use crate::message::Payload;

pub struct PingPayloadBuilder
{
    pub buffer: Buffer
}

impl PingPayloadBuilder
{
    pub fn new<'configs>(configs: &'configs Configs) -> Result<Self, Error> {
        let factory = configs.schemas.get(&CORE_SCHEMA_PING)?;
        let buffer = factory.new_buffer(Option::None);
        let mut buffer = Buffer::new(buffer);
        buffer.set(&path!(), "PING");
        Ok(PingPayloadBuilder {
            buffer: buffer
        })
    }

    pub fn to_payload( builder: PingPayloadBuilder) -> Payload
    {
        Payload {
            buffer: builder.buffer.read_only(),
            artifact: CORE_SCHEMA_PING.clone()
        }
    }
}

pub struct PongPayloadBuilder
{
    pub buffer: Buffer
}

impl PongPayloadBuilder
{
    pub fn new<'configs>(configs: &'configs Configs) -> Result<Self, Error> {
        let factory = configs.schemas.get(&CORE_SCHEMA_PONG)?;
        let buffer = factory.new_buffer(Option::None);
        let mut buffer = Buffer::new(buffer);
        buffer.set(&path!(),"PONG");
        Ok(PongPayloadBuilder {
            buffer: buffer
        })
    }
    pub fn to_payload( builder: PingPayloadBuilder) -> Payload
    {
        Payload {
            buffer: builder.buffer.read_only(),
            artifact: CORE_SCHEMA_PONG.clone()
        }
    }
}

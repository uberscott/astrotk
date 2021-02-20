use crate::buffers::*;
use crate::configs::Configs;
use crate::error::Error;
use crate::core::*;
use crate::message::Payload;

pub struct PingPayloadBuilder;

impl PingPayloadBuilder
{
    pub fn new<'configs>(configs: &'configs Configs) -> Result<Payload, Error> {
        let factory = configs.schemas.get(&CORE_SCHEMA_PING)?;
        let buffer = factory.new_buffer(Option::None);
        let mut buffer = Buffer::new(buffer);
        buffer.set(&path!(), "PING");

        Ok(Payload {
            buffer: buffer.read_only(),
            artifact: CORE_SCHEMA_PING.clone()
        })
    }
}

pub struct PongPayloadBuilder;
impl PongPayloadBuilder
{
    pub fn new<'configs>(configs: &'configs Configs) -> Result<Payload, Error> {
        let factory = configs.schemas.get(&CORE_SCHEMA_PONG)?;
        let buffer = factory.new_buffer(Option::None);
        let mut buffer = Buffer::new(buffer);
        buffer.set(&path!(),"PONG");

        Ok(Payload {
            buffer: buffer.read_only(),
            artifact: CORE_SCHEMA_PONG.clone()
        })
    }
}

pub struct TextPayloadBuilder;

impl TextPayloadBuilder
{
    pub fn new(text: &str, configs: &Configs) -> Result<Payload, Error> {
        let factory = configs.schemas.get(&CORE_SCHEMA_TEXT )?;
        let buffer = factory.new_buffer(Option::None);
        let mut buffer = Buffer::new(buffer);
        buffer.set(&path!(),text.to_string() )?;
        Ok(Payload {
            buffer: buffer.read_only(),
            artifact: CORE_SCHEMA_TEXT.clone()
        })
    }
}


pub struct OkPayloadBuilder;

impl OkPayloadBuilder
{
    pub fn new<'configs>(ok: bool, configs: &'configs Configs) -> Result<Payload, Error> {
        let factory = configs.schemas.get(&CORE_SCHEMA_OK)?;
        let buffer = factory.new_buffer(Option::None);
        let mut buffer = Buffer::new(buffer);
        buffer.set(&path!(), ok)?;

        Ok(Payload {
            buffer: buffer.read_only(),
            artifact: CORE_SCHEMA_PING.clone()
        })
    }
}
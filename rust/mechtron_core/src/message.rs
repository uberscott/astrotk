use std::collections::HashMap;
use std::sync::Arc;

use bytes::Bytes;
use no_proto::buffer::{NP_Buffer, NP_Finished_Buffer};
use no_proto::collection::map::NP_Map;
use no_proto::collection::struc::NP_Struct;
use no_proto::error::NP_Error;
use no_proto::memory::NP_Memory_Owned;
use no_proto::NP_Factory;
use no_proto::pointer::{NP_Scalar, NP_Value};
use no_proto::pointer::option::NP_Enum;
use uuid::Uuid;

use crate::artifact::Artifact;
use crate::buffers::{Buffer, BufferFactories, Path, ReadOnlyBuffer};
use crate::configs::Configs;
use crate::error::Error;
use crate::id::{DeliveryMomentKey, Id, IdSeq, Revision, TronKey};
use crate::util::{TextPayloadBuilder, OkPayloadBuilder};

static ID: &'static str = r#"
struct({fields: {
        seq_id: i64(),
        id: i64()
      }})
"#;

static MESSAGE_SCHEMA: &'static str = r#"
struct({fields: {
  id:struct({fields: {
        seq_id: i64(),
        id: i64()
      }}),
  kind: enum({choices: ["Create", "Update", "Content", "Request", "Response", "Reject"]}),
  from: struct({fields: {
    tron: struct({fields: {
      nucleus: struct({fields: {
        seq_id: i64(),
        id: i64()
      }}),
      tron: struct({fields: {
        seq_id: i64(),
        id: i64()
      }})
    }}),
    cycle: i64(),
    timestamp: u64()
  }}),
  to: struct({fields: {
    tron: struct({fields: {
      nucleus: struct({fields: {
        seq_id: i64(),
        id: i64()
      }}),
      tron: struct({fields: {
        seq_id: i64(),
        id: i64()
      }})
    }}),
    port: string(),
    cycle: i64(),
    phase: u8(),
    delivery: enum({choices: ["Cyclic", "Phasic", "ExtraCyclic"], default: "Cyclic"}),
    target: enum({choices: ["Shell", "Kernel"], default: "Kernel"})
  }}),
  callback: struct({fields: {
    tron: struct({fields: {
      nucleus: struct({fields: {
        seq_id: i64(),
        id: i64()
      }}),
      tron: struct({fields: {
        seq_id: i64(),
        id: i64()
      }})
    }}),
    port: string(),
    cycle: i64(),
    phase: u8(),
    delivery: enum({choices: ["Cyclic", "Phasic", "ExtraCyclic"], default: "Cyclic"}),
    target: enum({choices: ["Shell", "Kernel"], default: "Kernel"})
  }}),
  payloads: list( {of: struct({fields: {artifact: string(),bytes: bytes()} }) })

}})


"#;

static MESSAGE_BUILDERS_SCHEMA: &'static str = r#"{
"type": "string"
}"#;

lazy_static! {
    static ref MESSAGES_FACTORY: NP_Factory<'static> = NP_Factory::new(MESSAGE_SCHEMA).unwrap();
    static ref MESSAGE_BUILDERS_FACTORY: NP_Factory<'static> =
        NP_Factory::new_json(MESSAGE_BUILDERS_SCHEMA).unwrap();
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct To {
    pub tron: TronKey,
    pub port: String,
    pub cycle: Cycle,
    pub phase: u8,
    pub delivery: DeliveryMoment,
    pub target: DeliveryTarget
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct From {
    pub tron: TronKey,
    pub cycle: i64,
    pub timestamp: u64,
}

impl From {
    pub fn append(&self, path: &Path, buffer: &mut Buffer) -> Result<(), Error> {
        self.tron.append(&path.push(path!("tron")), buffer)?;
        buffer.set(&path.with(path!("cycle")), self.cycle.clone())?;
        buffer.set(&path.with(path!("timestamp")), self.timestamp.clone())?;
        Ok(())
    }

    pub fn from(path: &Path, buffer: &ReadOnlyBuffer) -> Result<Self, Error> {
        Ok(From {
            tron: TronKey::from(&path.push(path!["tron"]), buffer)?,
            cycle: buffer.get(&path.with(path!["cycle"]))?,
            timestamp: buffer.get(&path.with(path!["timestamp"]))?,
        })
    }
}

impl To {
    pub fn basic(tron: TronKey, port: String) -> Self {
        To {
            tron: tron,
            port: port,
            cycle: Cycle::Next,
            phase: 0,
            delivery: DeliveryMoment::Cyclic,
            target: DeliveryTarget::Kernel
        }
    }

    pub fn phasic(tron: TronKey, port: String, phase: u8) -> Self {
        To {
            tron: tron,
            port: port,
            cycle: Cycle::Next,
            phase: phase,
            delivery: DeliveryMoment::Cyclic,
            target: DeliveryTarget::Kernel
        }
    }

    pub fn inter_phasic(tron: TronKey, port: String, phase: u8) -> Self {
        To {
            tron: tron,
            port: port,
            cycle: Cycle::Present,
            phase: phase,
            delivery: DeliveryMoment::Phasic,
            target: DeliveryTarget::Kernel
        }
    }

    pub fn append(&self, path: &Path, buffer: &mut Buffer) -> Result<(), Error> {
        self.tron.append(&path.push(path!("tron")), buffer)?;
        buffer.set(&path.with(path!("port")), self.port.clone())?;

        match self.cycle {
            Cycle::Exact(cycle) => {
                buffer.set(&path.with(path!("cycle")), cycle.clone())?;
            }
            Cycle::Present => {}
            Cycle::Next => {}
        }

        buffer.set(&path.with(path!("phase")), self.phase.clone())?;

        match self.delivery {
            DeliveryMoment::Cyclic => {
println!("Cyclic!!!");
                buffer.set(
                    &path.with(path!("delivery")),
                    NP_Enum::Some("Cyclic".to_string()),
                )?;
            }
            DeliveryMoment::Phasic => {
                buffer.set(
                    &path.with(path!("delivery")),
                    NP_Enum::Some("Phasic".to_string()),
                )?;
            }
            DeliveryMoment::ExtraCyclic=> {
                buffer.set(
                    &path.with(path!("delivery")),
                    NP_Enum::Some("ExtraCyclic".to_string()),
                )?;
            }
        }

        match self.target{
            DeliveryTarget::Shell=> {
                buffer.set(
                    &path.with(path!("target")),
                    NP_Enum::Some("Shell".to_string()),
                )?;
            }
            DeliveryTarget::Kernel=> {
                buffer.set(
                    &path.with(path!("target")),
                    NP_Enum::Some("Kernel".to_string()),
                )?;
            }
        }

        Ok(())
    }

    pub fn from(path: &Path, buffer: &ReadOnlyBuffer) -> Result<Self, Error> {
        let tron = TronKey::from(&path.push(path!("tron")), buffer)?;
        let port = buffer.get::<String>(&path.with(path!["port"]))?;
        let cycle = match buffer.is_set::<i64>(&path.with(path!("cycle")))? {
            true => Cycle::Exact(buffer.get::<i64>(&path.with(path!("cycle")))?),
            false => Cycle::Next,
        };

        let phase = buffer.get::<u8>(&path.with(path!["phase"]))?;
        let delivery = match buffer.get::<NP_Enum>(&path.with(path!("delivery")))? {
            NP_Enum::None => return Err("unkown delivery type".into()),
            NP_Enum::Some(delivery) => match delivery.as_str() {
                "Cyclic" => DeliveryMoment::Cyclic,
                "Phasic" => DeliveryMoment::Phasic,
                "ExtraCyclic" => DeliveryMoment::ExtraCyclic,
                _ => return Err("unknown delivery type".into()),
            },
        };
        let target = match buffer.get::<NP_Enum>(&path.with(path!("target")))? {
            NP_Enum::None => return Err("unkown target type".into()),
            NP_Enum::Some(target) => match target.as_str() {
                "Shell" => DeliveryTarget::Shell,
                "Kernel" => DeliveryTarget::Kernel,
                _ => return Err("unknown delivery type".into()),
            },
        };


        Ok(To {
            tron: tron,
            port: port,
            cycle: cycle,
            phase: phase,
            delivery: delivery,
            target: target
        })
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Phase {
    Primordial,
    Pre,
    Zero,
    Custom(String),
    Post,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Cycle {
    Exact(i64),
    Present,
    Next,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum MessageKind {
    Create,
    Update,
    Content,
    Request,
    Response,
    Reject,
    Panic
}

fn message_kind_to_index(kind: &MessageKind) -> u8 {
    match kind {
        MessageKind::Create => 0,
        MessageKind::Update => 1,
        MessageKind::Content => 2,
        MessageKind::Request => 3,
        MessageKind::Response => 4,
        MessageKind::Reject => 5,
        MessageKind::Panic => 6,
    }
}

fn message_kind_to_string(kind: &MessageKind) -> &str {
    match kind {
        MessageKind::Create => "Create",
        MessageKind::Update => "Update",
        MessageKind::Content => "Content",
        MessageKind::Request => "Request",
        MessageKind::Response => "Response",
        MessageKind::Reject => "Reject",
        MessageKind::Panic=> "Panic",
    }
}

fn index_to_message_kind(index: u8) -> Result<MessageKind, Error> {
    match index {
        0 => Ok(MessageKind::Create),
        1 => Ok(MessageKind::Update),
        2 => Ok(MessageKind::Content),
        3 => Ok(MessageKind::Request),
        4 => Ok(MessageKind::Response),
        5 => Ok(MessageKind::Reject),
        6 => Ok(MessageKind::Panic),
        _ => Err(format!("invalid index {}", index).into()),
    }
}

fn string_to_message_kind(str: &str) -> Result<MessageKind, Error> {
    match str {
        "Create" => Ok(MessageKind::Create),
        "Update" => Ok(MessageKind::Update),
        "Content" => Ok(MessageKind::Content),
        "Request" => Ok(MessageKind::Request),
        "Response" => Ok(MessageKind::Response),
        "Reject" => Ok(MessageKind::Reject),
        "Panic" => Ok(MessageKind::Reject),
        _ => Err(format!("invalid index {}", str).into()),
    }
}

// meaning the "between" delivery which can either be between cycles or phases
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum DeliveryMoment {
    Cyclic,
    Phasic,
    ExtraCyclic
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum DeliveryTarget {
    Kernel,
    Shell
}

#[derive(Clone)]
pub struct MessageBuilder {
    pub kind: Option<MessageKind>,
    pub from: Option<From>,
    pub to_nucleus_lookup_name: Option<String>,
    pub to_nucleus_id: Option<Id>,
    pub to_tron_lookup_name: Option<String>,
    pub to_tron_id: Option<Id>,
    pub to_cycle_kind: Option<Cycle>,
    pub to_phase: Option<u8>,
    pub to_phase_name: Option<String>,
    pub to_port: Option<String>,
    pub to_delivery: Option<DeliveryMoment>,
    pub to_target: Option<DeliveryTarget>,
    pub payloads: Option<Vec<PayloadBuilder>>,
    pub meta: Option<HashMap<String, String>>,
    pub transaction: Option<Id>,
    pub callback: Option<To>,
}

impl MessageBuilder {
    pub fn new() -> Self {
        MessageBuilder {
            kind: None,
            from: None,
            to_nucleus_lookup_name: None,
            to_tron_lookup_name: None,
            to_nucleus_id: None,
            to_tron_id: None,
            to_cycle_kind: None,
            to_port: None,
            to_phase: None,
            to_phase_name: None,
            to_delivery: None,
            to_target: None,
            payloads: None,
            meta: None,
            transaction: None,
            callback: None
        }
    }

    pub fn validate(&self) -> Result<(), Error> {
        if self.kind.is_none() {
            return Err("message builder kind must be set".into());
        }

        if self.to_phase_name.is_some() && self.to_phase.is_some() {
            return Err("to_phase_name and to_phase cannot both be set".into());
        }

        if self.to_nucleus_lookup_name.is_some() == self.to_nucleus_id.is_some() {
            return Err("message builder to_nucleus_lookup_name OR to_nucleus_id must be set (but not both)".into());
        }

        if self.to_tron_lookup_name.is_some() == self.to_tron_id.is_some() {
            return Err(
                "message builder to_tron_lookup_name OR to_tron_id must be set (but not both)"
                    .into(),
            );
        }

        if self.to_cycle_kind.is_none() {
            return Err(
                "message builder to_cycle_kind OR to_cycle must be set (but not both)".into(),
            );
        }
        if self.payloads.is_none() {
            return Err("message builder payload must be set".into());
        }

        if self.to_port.is_none() {
            return Err("message builder to_port must be set".into());
        }

        if self.kind == Option::Some(MessageKind::Request) &&
            self.callback.is_none()
        {
            return Err("message builder callback must be set if message kind is Request".into());
        }

        Ok(())
    }

    pub fn validate_build(&self) -> Result<(), Error> {
        self.validate()?;

        if self.to_nucleus_id.is_none() {
            return Err("message builder to_nucleus_id must be set before build".into());
        }

        if self.to_tron_id.is_none() {
            return Err("message builder to_tron_id must be set before build".into());
        }

        Ok(())
    }

    pub fn build(&self, seq: &mut IdSeq) -> Result<Message, Error> {
        self.validate_build()?;
        Ok(Message {
            id: seq.next(),
            kind: self.kind.clone().unwrap(),
            from: self.from.clone().unwrap(),
            to: To {
                tron: TronKey {
                    nucleus: self.to_nucleus_id.clone().unwrap(),
                    tron: self.to_tron_id.clone().unwrap(),
                },
                port: self.to_port.clone().unwrap(),
                cycle: self.to_cycle_kind.clone().unwrap(),
                phase: self.to_phase.clone().unwrap(),
                delivery: match &self.to_delivery {
                    Some(r) => r.clone(),
                    None => DeliveryMoment::Cyclic,
                },
                target: match &self.to_target{
                    Some(r) => r.clone(),
                    None => DeliveryTarget::Kernel,
                },
            },
            payloads: vec![],
            meta: self.meta.clone(),
            transaction: self.transaction.clone(),
            callback: self.callback.clone()
        })
    }

    pub fn message_builders_to_buffer(
        builders: Vec<MessageBuilder>,
    ) -> Result<NP_Buffer<NP_Memory_Owned>, Error> {
        let mut buffer = MESSAGE_BUILDERS_FACTORY.new_buffer(Option::None);
        let mut index = 0;
        for b in builders {
            let result = b.append_to_buffer(&mut buffer, index);
            match result {
                Ok(_) => {}
                Err(e) => {
                    return Err(format!("error when append_to_buffer {}", e.to_string()).into())
                }
            };
            index = index + 1;
        }
        return Ok(buffer);
    }

    pub fn append_to_buffer(
        &self,
        buffer: &mut NP_Buffer<NP_Memory_Owned>,
        index: usize,
    ) -> Result<(), Error> {
        self.validate()?;
        let result = self.append_to_buffer_np_error(buffer, index);
        match result {
            Ok(_) => Ok(()),
            Err(e) => Err("np_error".into()),
        }
    }

    fn append_to_buffer_np_error(
        &self,
        buffer: &mut NP_Buffer<NP_Memory_Owned>,
        index: usize,
    ) -> Result<(), NP_Error> {
        let index = index.to_string();
        //        buffer.set(&[&index, &"kind"], message_kind_to_index(&self.kind.as_ref().unwrap()))?;

        Ok(())
    }
}

#[derive(Clone)]
pub struct PayloadBuilder {
    pub buffer: Buffer,
    pub artifact: Artifact,
}

impl PayloadBuilder {
    pub fn build(builder: PayloadBuilder) -> Payload {
        Payload {
            artifact: builder.artifact,
            buffer: builder.buffer.read_only(),
        }
    }
}

#[derive(Clone)]
pub struct Payload {
    pub buffer: ReadOnlyBuffer,
    pub artifact: Artifact,
}

impl Payload {
    pub fn dump(payload: Payload, path: &Path, buffer: &mut Buffer) -> Result<(), Error> {
        buffer.set(&path.with(path!["artifact"]), payload.artifact.to())?;
        buffer.set::<Vec<u8>>(
            &path.with(path!["bytes"]),
            ReadOnlyBuffer::bytes(payload.buffer),
        )?;

        Ok(())
    }

    pub fn from(
        path: &Path,
        buffer: &ReadOnlyBuffer,
        buffer_factories: &BufferFactories,
    ) -> Result<Payload, Error> {
        let artifact = Artifact::from(buffer.get(&path.with(path!["artifact"]))?)?;
        let bytes = buffer.get::<Vec<u8>>(&path.with(path!["bytes"]))?;
        let factory = buffer_factories.get(&artifact)?;
        let buffer = factory.open_buffer(bytes);
        let buffer = ReadOnlyBuffer::new(buffer);
        Ok(Payload {
            buffer: buffer,
            artifact: artifact,
        })
    }
}

#[derive(Clone)]
pub struct Message {
    pub id: Id,
    pub kind: MessageKind,
    pub from: From,
    pub to: To,
    pub callback: Option<To>,
    pub payloads: Vec<Payload>,
    pub meta: Option<HashMap<String, String>>,
    pub transaction: Option<Id>,
}

impl Message {
    pub fn single_payload(
        seq: Arc<IdSeq>,
        kind: MessageKind,
        from: From,
        to: To,
        payload: Payload,
    ) -> Self {
        Message::multi_payload(seq, kind, from, to, vec![payload])
    }

    pub fn multi_payload(
        seq: Arc<IdSeq>,
        kind: MessageKind,
        from: From,
        to: To,
        payloads: Vec<Payload>,
    ) -> Self {
        Message::longform(seq, kind, from, to, Option::None, payloads, Option::None, Option::None)
    }

    pub fn longform(
        seq: Arc<IdSeq>,
        kind: MessageKind,
        from: From,
        to: To,
        callback: Option<To>,
        payloads: Vec<Payload>,
        meta: Option<HashMap<String, String>>,
        transaction: Option<Id>,
    ) -> Self {
        Message {
            id: seq.next(),
            kind: kind,
            from: from,
            to: to,
            payloads: payloads,
            meta: meta,
            transaction: transaction,
            callback: callback
        }
    }

    pub fn calc_bytes(&self) -> usize {
        let mut size = 0;
        for payload in &self.payloads {
            size = size + payload.buffer.size();
        }
        return size;
    }

    pub fn to_bytes(message: Message) -> Result<Vec<u8>, Error> {
        let mut buffer =
            Buffer::new(MESSAGES_FACTORY.new_buffer(Option::Some(message.calc_bytes())));
        let path = Path::new(path!());
        message.id.append(&path.push(path!["id"]), &mut buffer)?;
        buffer.set(
            &path!("kind"),
            NP_Enum::new(message_kind_to_string(&message.kind)),
        )?;

        message
            .from
            .append(&path.push(path!["from"]), &mut buffer)?;

        message.to.append(&path.push(path!["to"]), &mut buffer)?;
        if message.callback.is_some()
        {
            message.to.append(&path.push(path!["callback"]), &mut buffer)?;
        }

        let mut payload_index = 0;
        for payload in message.payloads {
            let path = path.push(path!("payloads", payload_index.to_string()));
            Payload::dump(payload, &path, &mut buffer)?;
            payload_index = payload_index + 1;
        }

        if message.meta.is_some() {
            let meta = message.meta.unwrap();
            for k in meta.keys() {
                buffer.set(
                    &path!(&"meta", &k.as_str()),
                    meta.get(k).unwrap().to_string(),
                )?;
            }
        }

        if message.transaction.is_some() {
            let transaction = &message.transaction.clone().unwrap();
            transaction.append(&Path::new(path!("transaction")), &mut buffer);
        }

        // for some reason compacting is breaking!
        //        buffer.compact()?;

        Ok((Buffer::bytes(buffer)))
    }

    pub fn from_bytes(
        bytes: Vec<u8>,
        buffer_factories: &dyn BufferFactories,
    ) -> Result<Self, Error> {
        let buffer = MESSAGES_FACTORY.open_buffer(bytes);
        let buffer = Buffer::new(buffer);
        let buffer = buffer.read_only();
        let id = Id::from(&Path::new(path!("id")), &buffer)?;
        let kind = buffer.get::<NP_Enum>(&path!["kind"])?;
        let kind = string_to_message_kind(kind.to_str())?;

        let from = From::from(&Path::new(path!["from"]), &buffer)?;
        let to = To::from(&Path::new(path!["to"]), &buffer)?;
        let callback = match buffer.is_set::<String>(&path!["callback","port"]) {
            Ok(value) => match value {
                true => Option::Some(To::from(&Path::new(path!["callback"]), &buffer)?),
                false => Option::None
            }
            Err(_) => Option::None
        };

        let payload_count = buffer.get_length(&path!("payloads"))?;
        let mut payloads = vec![];
        for payload_index in 0..payload_count {
            let path = Path::new(path!["payloads", payload_index.to_string()]);
            payloads.push(Payload::from(&path, &buffer, buffer_factories)?);
        }

        Ok(Message {
            id: id,
            kind: kind,
            from: from,
            to: to,
            callback: callback,
            payloads: payloads,
            meta: Option::None,
            transaction: Option::None,
        })
    }

    pub fn reject(&self, from: From,reason: &str,  seq: Arc<IdSeq>,configs: &Configs, ) -> Result<Message, Error>
    {
        let rtn = Message::single_payload(seq,
                                           MessageKind::Reject,
                                          from,
                                          match &self.callback
                                          {
                                              None => {
                                                  To {
                                                      tron: self.from.tron.clone(),
                                                      port: "reject".to_string(),
                                                      cycle: Cycle::Next,
                                                      phase: 0,
                                                      delivery: DeliveryMoment::Cyclic,
                                                      target: DeliveryTarget::Kernel,
                                                  }
                                              }
                                              Some(callback) => callback.clone(),
                                          },
                                   TextPayloadBuilder::new(reason, configs)?,
        );

        Ok(rtn)
    }

    pub fn respond(&self, from: From, payloads: Vec<Payload>, seq: Arc<IdSeq>) -> Message
    {
        Message::longform(seq,
                                    MessageKind::Response,
                                    from,
                                    match &self.callback
                                    {
                                        None => {
                                            To {
                                                tron: self.from.tron.clone(),
                                                port: self.to.port.clone(),
                                                cycle: Cycle::Next,
                                                phase: 0,
                                                delivery: DeliveryMoment::Cyclic,
                                                target: DeliveryTarget::Kernel,
                                            }
                                        }
                                        Some(callback) => callback.clone(),
                                    },
                                    Option::None,
                                    payloads,
                                    Option::None,
                                    Option::Some(self.id)
        )

    }


    pub fn ok(&self, from: From,  ok:bool, seq: Arc<IdSeq>,configs: &Configs) -> Result<Message, Error>
    {
        let payload = OkPayloadBuilder::new(ok,configs)?;

        let rtn = Message::single_payload(seq,
                                           MessageKind::Reject,
                                          from,
                                          match &self.callback
                                          {
                                              None => {
                                                  To {
                                                      tron: self.from.tron.clone(),
                                                      port: "reject".to_string(),
                                                      cycle: Cycle::Next,
                                                      phase: 0,
                                                      delivery: DeliveryMoment::Cyclic,
                                                      target: DeliveryTarget::Kernel,
                                                  }
                                              }
                                              Some(callback) => callback.clone(),
                                          },payload
        );


        Ok(rtn)
    }
}

fn cat(path: &[&str]) -> String {
    let mut rtn = String::new();
    for segment in path {
        rtn.push_str(segment);
        rtn.push('/');
    }
    return rtn;
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use no_proto::buffer::NP_Buffer;
    use no_proto::memory::NP_Memory_Ref;
    use no_proto::NP_Factory;

    use crate::artifact::Artifact;
    use crate::buffers::{Buffer, BufferFactories, Path};
    use crate::error::Error;
    use crate::id::{Id, IdSeq, TronKey};
    use crate::message;
    use crate::message::{Cycle, DeliveryMoment, DeliveryTarget, From, ID, Message, MESSAGE_BUILDERS_SCHEMA, MESSAGE_SCHEMA, MessageKind, Payload, PayloadBuilder, To};

    static TEST_SCHEMA: &'static str = r#"list({of: string()})"#;

    struct BufferFactoriesImpl {}

    impl BufferFactories<'_> for BufferFactoriesImpl {
        fn get(&self, artifact: &Artifact) -> Result<Arc<NP_Factory<'static>>, Error> {
            Ok(Arc::new(NP_Factory::new(TEST_SCHEMA).unwrap()))
        }
    }

    #[test]
    fn check_message_schema() {
        NP_Factory::new(MESSAGE_SCHEMA).unwrap();
    }

    #[test]
    fn check_ro_buffer() {
        let schema = r#"struct({fields: {
                         userId: string(),
                         password: string(),
                         email: string(),
                         age: u8()
}})"#;
        let np_factory = NP_Factory::new(schema).unwrap();
        let mut np_buffer = np_factory.new_buffer(Option::None);
        let mut buffer = Buffer::new(np_buffer);

        buffer.set(&path!("userId"), "hello").unwrap();
    }
    #[test]
    fn test_serialize_id() {
        let np_factory = NP_Factory::new(ID).unwrap();
        let mut np_buffer = np_factory.new_buffer(Option::None);

        let mut buffer = Buffer::new(np_buffer);

        let mut seq = IdSeq::new(0);
        let id = seq.next();

        let path = Path::new(path![]);
        id.append(&path, &mut buffer).unwrap();

        let buffer = buffer.read_only();

        let ser_id = Id::from(&path, &buffer).unwrap();

        assert_eq!(id, ser_id);
    }

    #[test]
    fn test_serialize_from() {
        let np_factory = NP_Factory::new(MESSAGE_SCHEMA).unwrap();
        let np_buffer = np_factory.new_buffer(Option::None);
        let mut buffer = Buffer::new(np_buffer);

        let mut seq = IdSeq::new(0);
        let from = message::From {
            tron: TronKey {
                nucleus: seq.next(),
                tron: seq.next(),
            },
            cycle: 0,
            timestamp: 0,
        };

        let path = Path::new(path!["from"]);
        from.append(&path, &mut buffer).unwrap();

        let buffer = buffer.read_only();

        let ser_from = message::From::from(&path, &buffer).unwrap();

        assert_eq!(from, ser_from);
    }

    #[test]
    fn test_serialize_to()
    {
        let np_factory = NP_Factory::new(MESSAGE_SCHEMA).unwrap();
        let np_buffer = np_factory.new_buffer(Option::None);
        let mut buffer = Buffer::new(np_buffer);

        let mut seq = IdSeq::new(0);

        let key = TronKey {
            nucleus: seq.next(),
            tron: seq.next(),
        };
        let to = To::basic(key,"someport".to_string());

        let path = Path::new(path!["to"]);
        to.append(&path, &mut buffer).unwrap();

        let buffer = buffer.read_only();

        let ser_to = To::from(&path, &buffer).unwrap();

        assert_eq!(to, ser_to);

    }

    #[test]
    fn test_serialize_message() {
        let artifact = Artifact::from("mechtron.io:core:1.0.0:schema/empty.schema").unwrap();
        let factories = BufferFactoriesImpl {};
        let np_factory = factories.get(&artifact).unwrap();
        let np_buffer = np_factory.new_buffer(Option::None);
        let mut buffer = Buffer::new(np_buffer);
        buffer.set(&path!("0"), "hello");
        let payload = Payload {
            buffer: buffer.read_only(),
            artifact: artifact.clone(),
        };
        let seq = Arc::new(IdSeq::new(0));

        let from = message::From {
            tron: TronKey {
                nucleus: seq.next(),
                tron: seq.next(),
            },
            cycle: 0,
            timestamp: 0,
        };

        let to = To {
            tron: TronKey {
                nucleus: seq.next(),
                tron: seq.next(),
            },
            port: "someport".to_string(),
            cycle: Cycle::Exact(32),
            phase: 0,
            delivery: DeliveryMoment::Cyclic,
            target: DeliveryTarget::Kernel
        };

        let mut message = Message::single_payload(
            seq.clone(),
            MessageKind::Create,
            from.clone(),
            to.clone(),
            payload.clone(),
        );

        let bytes = Message::to_bytes(message.clone()).unwrap();

        let new_message = Message::from_bytes(bytes, &factories).unwrap();

        assert_eq!(message.to, new_message.to);
        assert_eq!(message.from, new_message.from);
        assert_eq!(
            message.payloads[0].artifact,
            new_message.payloads[0].artifact
        );
        assert_eq!(
            message.payloads[0].buffer.read_bytes(),
            new_message.payloads[0].buffer.read_bytes()
        );

        //assert_eq!(message,new_message);
    }
}

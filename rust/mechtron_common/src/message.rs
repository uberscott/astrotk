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

use crate::artifact::Artifact;
use crate::buffers::{Buffer, BufferFactories, Path, ReadOnlyBuffer};
use crate::configs::Configs;
use crate::error::Error;
use crate::id::{DeliveryMomentKey, Id, IdSeq, Revision, MechtronKey};
use crate::util::{OkPayloadBuilder, TextPayloadBuilder};
use crate::core::*;
use std::cell::{Cell, RefCell};


#[derive(PartialEq, Eq, Clone, Debug)]
pub struct To {
    pub tron: MechtronKey,
    pub port: String,
    pub cycle: Cycle,
    pub phase: String,
    pub delivery: DeliveryMoment,
    pub layer: MechtronLayer
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct From {
    pub tron: MechtronKey,
    pub cycle: i64,
    pub timestamp: u64,
    pub layer: MechtronLayer
}

impl From {
    pub fn append(&self, path: &Path, buffer: &mut Buffer) -> Result<(), Error> {
        self.tron.append(&path.push(path!("tron")), buffer)?;
        buffer.set(&path.with(path!("cycle")), self.cycle.clone())?;
        buffer.set(&path.with(path!("timestamp")), self.timestamp.clone())?;

        match self.layer {
            MechtronLayer::Shell=> {
                buffer.set(
                    &path.with(path!("layer")),
                    NP_Enum::Some("Shell".to_string()),
                )?;
            }
            MechtronLayer::Kernel=> {
                buffer.set(
                    &path.with(path!("layer")),
                    NP_Enum::Some("Kernel".to_string()),
                )?;
            }
        }

        Ok(())
    }

    pub fn from(path: &Path, buffer: &ReadOnlyBuffer) -> Result<Self, Error> {

        let layer = match buffer.get::<NP_Enum>(&path.with(path!("layer")))? {
            NP_Enum::None => return Err("unkown layer type".into()),
            NP_Enum::Some(layer) => match layer.as_str() {
                "Shell" => MechtronLayer::Shell,
                "Kernel" => MechtronLayer::Kernel,
                _ => return Err("unknown delivery type".into()),
            },
        };

        Ok(From {
            tron: MechtronKey::from(&path.push(path!["tron"]), buffer)?,
            cycle: buffer.get(&path.with(path!["cycle"]))?,
            timestamp: buffer.get(&path.with(path!["timestamp"]))?,
            layer: layer
        })
    }
}

impl To {
    pub fn basic(tron: MechtronKey, port: String) -> Self {
        To {
            tron: tron,
            port: port,
            cycle: Cycle::Next,
            phase: "default".to_string(),
            delivery: DeliveryMoment::Cyclic,
            layer: MechtronLayer::Kernel
        }
    }

    pub fn phasic(tron: MechtronKey, port: String, phase: String) -> Self {
        To {
            tron: tron,
            port: port,
            cycle: Cycle::Next,
            phase: phase,
            delivery: DeliveryMoment::Cyclic,
            layer: MechtronLayer::Kernel
        }
    }

    pub fn inter_phasic(tron: MechtronKey, port: String, phase: String) -> Self {
        To {
            tron: tron,
            port: port,
            cycle: Cycle::Present,
            phase: phase,
            delivery: DeliveryMoment::Phasic,
            layer: MechtronLayer::Kernel
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

        match self.layer {
            MechtronLayer::Shell=> {
                buffer.set(
                    &path.with(path!("layer")),
                    NP_Enum::Some("Shell".to_string()),
                )?;
            }
            MechtronLayer::Kernel=> {
                buffer.set(
                    &path.with(path!("layer")),
                    NP_Enum::Some("Kernel".to_string()),
                )?;
            }
        }

        Ok(())
    }

    pub fn from(path: &Path, buffer: &ReadOnlyBuffer) -> Result<Self, Error> {
        let tron = MechtronKey::from(&path.push(path!("tron")), buffer)?;
        let port = buffer.get::<String>(&path.with(path!["port"]))?;
        let cycle = match buffer.is_set::<i64>(&path.with(path!("cycle")))? {
            true => Cycle::Exact(buffer.get::<i64>(&path.with(path!("cycle")))?),
            false => Cycle::Next,
        };

        let phase = buffer.get::<String>(&path.with(path!["phase"]))?;
        let delivery = match buffer.get::<NP_Enum>(&path.with(path!("delivery")))? {
            NP_Enum::None => return Err("unkown delivery type".into()),
            NP_Enum::Some(delivery) => match delivery.as_str() {
                "Cyclic" => DeliveryMoment::Cyclic,
                "Phasic" => DeliveryMoment::Phasic,
                "ExtraCyclic" => DeliveryMoment::ExtraCyclic,
                _ => return Err("unknown delivery type".into()),
            },
        };
        let layer = match buffer.get::<NP_Enum>(&path.with(path!("layer")))? {
            NP_Enum::None => return Err("unkown layer type".into()),
            NP_Enum::Some(layer) => match layer.as_str() {
                "Shell" => MechtronLayer::Shell,
                "Kernel" => MechtronLayer::Kernel,
                _ => return Err("unknown delivery type".into()),
            },
        };


        Ok(To {
            tron: tron,
            port: port,
            cycle: cycle,
            phase: phase,
            delivery: delivery,
            layer: layer
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
pub enum NucleusKind
{
    Source,
    Replica
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum MessageKind {
    Create,
    Update,
    State,
    Request,
    Response,
    Command,
    Reject,
    Panic,
    Api
}

fn message_kind_to_index(kind: &MessageKind) -> u8 {
    match kind {
        MessageKind::Create => 0,
        MessageKind::Update => 1,
        MessageKind::State => 2,
        MessageKind::Request => 3,
        MessageKind::Response => 4,
        MessageKind::Command=> 5,
        MessageKind::Reject => 6,
        MessageKind::Panic => 7,
        MessageKind::Api=> 8,
    }
}

fn message_kind_to_string(kind: &MessageKind) -> &str {
    match kind {
        MessageKind::Create => "Create",
        MessageKind::Update => "Update",
        MessageKind::State => "Content",
        MessageKind::Request => "Request",
        MessageKind::Response => "Response",
        MessageKind::Command=> "Command",
        MessageKind::Reject => "Reject",
        MessageKind::Panic=> "Panic",
        MessageKind::Api=> "Api",
    }
}

fn index_to_message_kind(index: u8) -> Result<MessageKind, Error> {
    match index {
        0 => Ok(MessageKind::Create),
        1 => Ok(MessageKind::Update),
        2 => Ok(MessageKind::State),
        3 => Ok(MessageKind::Request),
        4 => Ok(MessageKind::Response),
        5 => Ok(MessageKind::Command),
        6 => Ok(MessageKind::Reject),
        7 => Ok(MessageKind::Panic),
        8 => Ok(MessageKind::Api),
        _ => Err(format!("invalid index {}", index).into()),
    }
}

fn string_to_message_kind(str: &str) -> Result<MessageKind, Error> {
    match str {
        "Create" => Ok(MessageKind::Create),
        "Update" => Ok(MessageKind::Update),
        "Content" => Ok(MessageKind::State),
        "Request" => Ok(MessageKind::Request),
        "Response" => Ok(MessageKind::Response),
        "Reject" => Ok(MessageKind::Reject),
        "Panic" => Ok(MessageKind::Panic),
        "Api" => Ok(MessageKind::Api),
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
pub enum MechtronLayer {
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
    pub to_phase: Option<String>,
    pub to_phase_name: Option<String>,
    pub to_port: Option<String>,
    pub to_delivery: Option<DeliveryMoment>,
    pub to_layer: Option<MechtronLayer>,
    pub payloads: RefCell<Option<Vec<Payload>>>,
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
            to_layer: None,
            payloads: RefCell::new(None),
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
        if self.payloads.borrow().is_none() {
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

    pub fn build(&self, seq: Arc<IdSeq>) -> Result<Message, Error> {
        self.validate_build()?;
        let payloads = self.payloads.replace(Option::None);

        Ok(Message {
            id: seq.next(),
            kind: self.kind.clone().unwrap(),
            from: self.from.clone().unwrap(),
            to: To {
                tron: MechtronKey {
                    nucleus: self.to_nucleus_id.clone().unwrap(),
                    mechtron: self.to_tron_id.clone().unwrap(),
                },
                port: self.to_port.clone().unwrap(),
                cycle: self.to_cycle_kind.clone().unwrap(),
                phase: self.to_phase.clone().unwrap(),
                delivery: match &self.to_delivery {
                    Some(r) => r.clone(),
                    None => DeliveryMoment::Cyclic,
                },
                layer: match &self.to_layer {
                    Some(r) => r.clone(),
                    None => MechtronLayer::Kernel,
                },
            },
            payloads: payloads.unwrap_or(vec!()),
            meta: self.meta.clone(),
            transaction: self.transaction.clone(),
            callback: self.callback.clone()
        })
    }

    pub fn to_buffer(
        builders: Vec<MessageBuilder>,
        configs: &Configs
    ) -> Result<Buffer, Error> {
        let mut buffer = {
            let factory = configs.schemas.get(&CORE_SCHEMA_MESSAGE_BUILDERS)?;
            let buffer = factory.new_buffer(Option::Some(builders.len() * 128));
            let buffer = Buffer::new(buffer);
            buffer
        };

        for index in 0..builders.len() {
            let builder = &builders[index];
            builder.append_to_buffer(Path::new(path![index.to_string()]), &mut buffer)?;
        }

        return Ok(buffer);
    }

    pub fn append_to_buffer(
        &self,
        path: Path,
        buffer: &mut Buffer,
    ) -> Result<(), Error> {
        self.validate()?;

        buffer.set(&path.with(path!("kind")), NP_Enum::Some(message_kind_to_string(&self.kind.as_ref().unwrap().clone()).to_string()))?;
        if self.from.is_some()
        {
            self.from.as_ref().unwrap().append(&path.push(path!("from")), buffer)?
        }
        {
            let path = path.push(path!["to"]);
            if self.to_nucleus_lookup_name.is_some()
            {
                buffer.set(&path.with(path!["nucleus_lookup_name"]), self.to_nucleus_lookup_name.as_ref().unwrap().clone())?;
            }
            if self.to_tron_lookup_name.is_some()
            {
                buffer.set(&path.with(path!["tron_lookup_name"]), self.to_tron_lookup_name.as_ref().unwrap().clone())?;
            }
            if self.to_tron_id.is_some()
            {
                self.to_tron_id.unwrap().append(&path.push(path!["tron_id"]), buffer)?
            }
            if self.to_nucleus_id.is_some()
            {
                self.to_nucleus_id.unwrap().append(&path.push(path!["nucleus_id"]), buffer)?
            }
            buffer.set(&path.with(path!["port"]), self.to_port.as_ref().unwrap().clone())?;
            buffer.set(&path.with(path!["phase"]), self.to_phase.as_ref().unwrap().clone())?;
            buffer.set(&path.with(path!["cycle_kind"]), NP_Enum::Some(match self.to_cycle_kind.as_ref().unwrap().clone() {
                Cycle::Exact(_) => "Exact".to_string(),
                Cycle::Present => "Present".to_string(),
                Cycle::Next => "Next".to_string()
            }));

            match self.to_cycle_kind.as_ref().unwrap()
            {
                Cycle::Exact(cycle) => {
                    buffer.set(&path.with(path!["cycle"]), cycle.clone() )?;
                }
                Cycle::Present => {}
                Cycle::Next => {}
            }

            buffer.set(&path.with(path!["delivery"]), NP_Enum::Some(match self.to_delivery.as_ref().unwrap().clone() {
                DeliveryMoment::Cyclic => "Cyclic".to_string(),
                DeliveryMoment::Phasic => "Phasic".to_string(),
                DeliveryMoment::ExtraCyclic => "ExtraCyclic".to_string()
            }));


            buffer.set(&path.with(path!["layer"]), NP_Enum::Some(match self.to_layer.as_ref().unwrap().clone() {
                MechtronLayer::Kernel => "Kernel".to_string(),
                MechtronLayer::Shell => "Shell".to_string()
            }));
        }

        {
            let path = path.push(path!("payloads"));
            let payloads = self.payloads.replace(Option::None).unwrap();
            let mut payload_index = 0;
            for payload in payloads
            {
                payload.append(&path.push(path![payload_index.to_string()]), buffer)?;
                payload_index = payload_index + 1;
            }
        }

        if self.meta.is_some()
        {
            let path = path.push(path!("meta"));
            for key in self.meta.as_ref().unwrap().clone().keys()
            {
                buffer.set(&path.with(path![key]), self.meta.as_ref().unwrap().get(key).clone().unwrap().clone())?;
            }
        }

        Ok(())
    }

    pub fn from_buffer(buffer: Vec<u8>, configs: &Configs) -> Result<Vec<MessageBuilder>, Error>
    {
        let buffer = {
            let factory = configs.schemas.get(&CORE_SCHEMA_MESSAGE_BUILDERS)?;
            let buffer = factory.open_buffer(buffer);
            let buffer = Buffer::new(buffer);
            buffer.read_only()
        };

        let mut builders = vec!();
        for index in 0..buffer.get_length(&path![])?
        {
            let path = Path::new(path![index.to_string()]);
            let mut builder = MessageBuilder::new();
            builder.kind = Option::Some(string_to_message_kind(buffer.get::<NP_Enum>(&path.with(path!["kind"]))?.to_str())?);

            if buffer.is_set::<i64>(&path.with(path!["from", "cycle"]))?
            {
                let from = From::from(&path.push(path!["from"]), &buffer)?;
                builder.from.replace(from);
            }

            {
                let path = path.push(path!["to"]);
                builder.to_nucleus_lookup_name = buffer.opt_get(&path.with(path!["nucleus_lookup_name"]));
                builder.to_tron_lookup_name = buffer.opt_get(&path.with(path!["tron_lookup_name"]));
                if buffer.is_set::<i64>(&path.with(path!["tron_id","id"]))?
                {
                    builder.to_tron_id = Option::Some(Id::from( &path.push( path!["tron_id"]), &buffer )?);
                }

                if buffer.is_set::<i64>(&path.with(path!["nucleus_id","id"]))?
                {
                    builder.to_nucleus_id= Option::Some(Id::from( &path.push( path!["nucleus_id"]), &buffer )?);
                }

                builder.to_port = buffer.opt_get(&path.with(path!["port"]));
                builder.to_cycle_kind = match buffer.get::<NP_Enum>(&path.with(path!["cycle_kind"]))? {
                    NP_Enum::None => { return Err("cycle_kind".into()) }
                    NP_Enum::Some(cycle_kind) => Option::Some(match cycle_kind.as_str() {
                        "Exact" => Cycle::Exact(buffer.get(&path.with(path!["cycle"]))?),
                        "Present" => Cycle::Present,
                        "Next"=> Cycle::Next,
                        _=> Cycle::Next,
                    })
                };
                builder.to_delivery = match buffer.get::<NP_Enum>(&path.with(path!["delivery"]))? {
                    NP_Enum::None => { return Err("delivery_kind".into()) }
                    NP_Enum::Some(delivery) => Option::Some(match delivery.as_str() {
                        "Cyclic" => DeliveryMoment::Cyclic,
                        "Phasic" => DeliveryMoment::Phasic,
                        "ExtraCyclic" => DeliveryMoment::ExtraCyclic,
                        _ => DeliveryMoment::Cyclic
                    })
                };
                builder.to_layer = match buffer.get::<NP_Enum>(&path.with(path!["layer"]))? {
                    NP_Enum::None => { return Err("layer".into()) }
                    NP_Enum::Some(delivery) => Option::Some(match delivery.as_str() {
                        "Shell" => MechtronLayer::Shell,
                        "Kernel" => MechtronLayer::Kernel,
                        _ => MechtronLayer::Kernel
                    })
                };
            }

            {
                let path = path.push(path!["payloads"]);
                let mut payloads = vec![];
                for index in 0..buffer.get_length(&path.with(path![]))?
                {
                    payloads.push(Payload::from(&path.push(path![index.to_string()]), &buffer, configs)?);
                }
                builder.payloads.replace(Option::Some(payloads));
            }

            if buffer.get_keys(&path.with(path!["meta"]))?.is_some()
            {
                let path = path.push(path!["meta"]);
                let mut meta = HashMap::new();
                for key in buffer.get_keys(&path.with(path![]))?.unwrap()
                {
                    let value = buffer.get::<String>(&path.with(path![key]))?;
                    meta.insert(key, value);
                }
                builder.meta = Option::Some(meta);
            }

            builders.push(builder);
        }

        Ok(builders)
    }
}



/*
#[derive(Clone)]
pub struct PayloadBuilder {
    pub buffer: Buffer,
    pub schema: Artifact,
}

impl PayloadBuilder {
    pub fn build(builder: PayloadBuilder) -> Payload {
        Payload {
            schema: builder.schema,
            buffer: builder.buffer.read_only(),
        }
    }
}
 */



#[derive(Clone)]
pub struct Payload {
    pub buffer: ReadOnlyBuffer,
    pub schema: Artifact,
}

impl Payload {
    pub fn append(&self, path: &Path, buffer: &mut Buffer) -> Result<(), Error> {
        buffer.set(&path.with(path!["artifact"]), self.schema.to())?;
        buffer.set::<Vec<u8>>(
            &path.with(path!["bytes"]),
            ReadOnlyBuffer::bytes(self.buffer.clone()),
        )?;

        Ok(())
    }

    pub fn from(
        path: &Path,
        buffer: &ReadOnlyBuffer,
        configs: &Configs,
    ) -> Result<Payload, Error> {
        let artifact = Artifact::from(buffer.get(&path.with(path!["artifact"]))?)?;
        let bytes = buffer.get::<Vec<u8>>(&path.with(path!["bytes"]))?;
        let factory = configs.schemas.get(&artifact)?;
        let buffer = factory.open_buffer(bytes);
        let buffer = ReadOnlyBuffer::new(buffer);
        Ok(Payload {
            buffer: buffer,
            schema: artifact,
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


    pub fn copy_to_bytes(&self, configs: &Configs) -> Result<Vec<u8>, Error> {
        Message::to_bytes(self, configs)
    }

    pub fn to_payload(message:&Message, configs: &Configs) -> Result<Payload, Error> {
        Ok(Payload{
            schema: CORE_SCHEMA_MESSAGE.clone(),
            buffer: Message::to_buffer(message,configs)?
        })
    }

    pub fn to_buffer(&self, configs: &Configs) -> Result<ReadOnlyBuffer, Error> {
        let bytes = self.to_bytes(configs)?;
        let factory = configs.schemas.get( &CORE_SCHEMA_MESSAGE )?;
        let buffer = factory.open_buffer(bytes);
        let buffer = ReadOnlyBuffer::new(buffer );
        Ok(buffer)
    }

    pub fn to_bytes(&self, configs: &Configs) -> Result<Vec<u8>, Error> {
        let factory = configs.schemas.get(&CORE_SCHEMA_MESSAGE)?;
        let mut buffer =
            Buffer::new(factory.new_buffer(Option::Some(self.calc_bytes())));
        let path = Path::new(path!());
        self.id.append(&path.push(path!["id"]), &mut buffer)?;

        buffer.set(
            &path!("kind"),
            NP_Enum::Some(message_kind_to_string(&self.kind).to_string()),
        )?;

       self
            .from
            .append(&path.push(path!["from"]), &mut buffer)?;

        self.to.append(&path.push(path!["to"]), &mut buffer)?;
        if self.callback.is_some()
        {
            self.to.append(&path.push(path!["callback"]), &mut buffer)?;
        }

        let mut payload_index = 0;
        for payload in &self.payloads {
            let path = path.push(path!("payloads", payload_index.to_string()));
            payload.append(&path, &mut buffer);
            payload_index = payload_index + 1;
        }

        if self.meta.is_some() {
            let meta = self.meta.as_ref().unwrap();
            for k in meta.keys() {
                buffer.set(
                    &path!(&"meta", &k.as_str()),
                    meta.get(k).unwrap().to_string(),
                )?;
            }
        }

        if self.transaction.is_some() {
            let transaction = &self.transaction.clone().unwrap();
            transaction.append(&Path::new(path!("transaction")), &mut buffer);
        }

        // for some reason compacting is breaking!
        //        buffer.compact()?;

        Ok((Buffer::bytes(buffer)))
    }

    pub fn from_bytes(
        bytes: Vec<u8>,
        configs: &Configs,
    ) -> Result<Self, Error> {
        let factory = configs.schemas.get(&CORE_SCHEMA_MESSAGE)?;
        let buffer = factory.open_buffer(bytes);
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
            payloads.push(Payload::from(&path, &buffer, configs )?);
        }

        let meta= match buffer.get_keys(&path!["meta"]) {
            Ok(option) => match option {
                None => Option::None,
                Some(keys) => {
                    let path = Path::new(path!["meta"]);
                    let mut rtn = HashMap::new();

                    for key in keys{
                        rtn.insert(key.clone(), buffer.get(&path.with(path![&key.as_str()] ))?);
                    }

                    Option::Some(rtn)
                }
            }
            Err(_) => Option::None
        };


        Ok(Message {
            id: id,
            kind: kind,
            from: from,
            to: to,
            callback: callback,
            payloads: payloads,
            meta: meta,
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
                                                      phase: "default".to_string(),
                                                      delivery: DeliveryMoment::Cyclic,
                                                      layer: self.from.layer.clone(),
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
                                                phase: "default".to_string(),
                                                delivery: DeliveryMoment::Cyclic,
                                                layer: self.from.layer.clone(),
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
                                                      phase: "default".to_string(),
                                                      delivery: DeliveryMoment::Cyclic,
                                                      layer: self.from.layer.clone(),
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
pub mod tests {
    use std::sync::{Arc, RwLock};

    use no_proto::buffer::NP_Buffer;
    use no_proto::memory::NP_Memory_Ref;
    use no_proto::NP_Factory;

    use crate::artifact::{Artifact, ArtifactBundle, ArtifactRepository, ArtifactCache};
    use crate::buffers::{Buffer, BufferFactories, Path};
    use crate::error::Error;
    use crate::id::{Id, IdSeq, MechtronKey};
    use crate::core::*;
    use crate::message;
    use std::collections::{HashSet, HashMap};
    use std::fs::File;
    use std::io::Read;
    use crate::configs::Configs;
    use crate::message::{To, Message, MessageKind, Cycle, DeliveryMoment, MechtronLayer, Payload, MessageBuilder};

    static TEST_SCHEMA: &'static str = r#"list({of: string()})"#;

    lazy_static!
    {
        static ref CONFIGS: Configs<'static>  = Configs::new(Arc::new(TestArtifactRepository::new("../../repo")));
    }


    #[test]
    fn check_message_schema() {
        CONFIGS.schemas.get(&CORE_SCHEMA_MESSAGE );
    }

    #[test]
    fn check_message_builder_schema() {
        CONFIGS.schemas.get(&CORE_SCHEMA_MESSAGE_BUILDERS );
    }
    #[test]
    fn test_message_builder() {
        let mut builder = MessageBuilder::new();
        builder.kind = Option::Some(MessageKind::Create);
//        builder.to_nucleus_id = Option::Some(Id::new(1,2));
        builder.to_nucleus_lookup_name = Option::Some("some nucleus".to_string());
        builder.to_tron_lookup_name= Option::Some("sometron".to_string());
 //       builder.to_tron_id = Option::Some(Id::new( 3, 4 ));

        let payload = {
            let factory = CONFIGS.schemas.get( &CORE_SCHEMA_TEXT ).unwrap();
            let mut buffer = factory.new_buffer(Option::None);
            let mut buffer = Buffer::new(buffer);
            buffer.set( &path!["text"], "Some TExt");
            let buffer = buffer.read_only();
            Payload{
                buffer: buffer,
                schema: CORE_SCHEMA_TEXT.clone()
            }
        };

        builder.payloads.replace( Option::Some( vec![payload] ));
        builder.from.replace(crate::message::From{
            tron: MechtronKey::new(Id::new(5,3), Id::new( 335,66)),
            cycle: 32,
            timestamp: 3242343,
            layer: MechtronLayer::Kernel
        });

        builder.to_delivery.replace(DeliveryMoment::ExtraCyclic);
        builder.to_port.replace( "HelloPOrt".to_string() );
        builder.to_cycle_kind.replace(Cycle::Exact(34));
        builder.to_phase.replace("Default".to_string());
        builder.to_layer.replace(MechtronLayer::Kernel);

        let mut meta = HashMap::new();
        meta.insert("One".to_string(), "Two ".to_string());
        meta.insert("Three".to_string(), "Four ".to_string());
        builder.meta.replace(meta.clone());

        let copy = builder.clone();

        let buffer = MessageBuilder::to_buffer(vec![builder], &CONFIGS ).unwrap();

        let restore = MessageBuilder::from_buffer(Buffer::bytes(buffer),&CONFIGS).unwrap();

        assert_eq!( copy.kind, restore[0].kind);
        assert_eq!( copy.to_nucleus_lookup_name, restore[0].to_nucleus_lookup_name);
        assert_eq!( copy.to_tron_lookup_name, restore[0].to_tron_lookup_name);
        assert_eq!( copy.to_tron_id, restore[0].to_tron_id);
        assert_eq!( copy.to_port, restore[0].to_port );
        assert_eq!( copy.from, restore[0].from );

        assert_eq!( copy.meta, restore[0].meta );

        let b1 = copy.payloads.into_inner();
        let b1 = &b1.unwrap()[0];
        let b1 =  b1.buffer.read_bytes();
        let b2 = &restore[0];
        let b2 = &b2.payloads.borrow();
        let b2 = &b2.as_ref().unwrap()[0];
        let b2 = b2.buffer.read_bytes();
        assert_eq!( b1, b2 )



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
    /*
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

     */

    #[test]
    fn test_serialize_from() {
        let np_factory = CONFIGS.schemas.get( &CORE_SCHEMA_MESSAGE ).unwrap();
        let np_buffer = np_factory.new_buffer(Option::None);
        let mut buffer = Buffer::new(np_buffer);

        let mut seq = IdSeq::new(0);
        let from = message::From {
            tron: MechtronKey {
                nucleus: seq.next(),
                mechtron: seq.next(),
            },
            cycle: 0,
            timestamp: 0,
            layer: MechtronLayer::Kernel
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
        let np_factory = CONFIGS.schemas.get( &CORE_SCHEMA_MESSAGE ).unwrap();
        let np_buffer = np_factory.new_buffer(Option::None);
        let mut buffer = Buffer::new(np_buffer);

        let mut seq = IdSeq::new(0);

        let key = MechtronKey {
            nucleus: seq.next(),
            mechtron: seq.next(),
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

        let np_factory = CONFIGS.schemas.get( &CORE_SCHEMA_MESSAGE ).unwrap();
        let artifact = CORE_SCHEMA_EMPTY.clone();
        let np_buffer = np_factory.new_buffer(Option::None);
        let mut buffer = Buffer::new(np_buffer);
        buffer.set(&path!("0"), "hello");
        let payload = Payload {
            buffer: buffer.read_only(),
            schema: artifact.clone(),
        };
        let seq = Arc::new(IdSeq::new(0));

        let from = message::From {
            tron: MechtronKey {
                nucleus: seq.next(),
                mechtron: seq.next(),
            },
            cycle: 0,
            timestamp: 0,
            layer: MechtronLayer::Kernel
        };

        let to = To {
            tron: MechtronKey {
                nucleus: seq.next(),
                mechtron: seq.next(),
            },
            port: "someport".to_string(),
            cycle: Cycle::Exact(32),
            phase: "default".to_string(),
            delivery: DeliveryMoment::Cyclic,
            layer: MechtronLayer::Kernel
        };

        let mut message = Message::single_payload(
            seq.clone(),
            MessageKind::Create,
            from.clone(),
            to.clone(),
            payload.clone(),
        );

        let bytes = message.to_bytes(&CONFIGS).unwrap();

        let new_message = Message::from_bytes(bytes, &CONFIGS).unwrap();

        assert_eq!(message.to, new_message.to);
        assert_eq!(message.from, new_message.from);
        assert_eq!(
            message.payloads[0].schema,
            new_message.payloads[0].schema
        );
        assert_eq!(
            message.payloads[0].buffer.read_bytes(),
            new_message.payloads[0].buffer.read_bytes()
        );

        //assert_eq!(message,new_message);
    }


    pub struct TestArtifactRepository {
        repo_path: String,
        fetches: RwLock<HashSet<ArtifactBundle>>,
    }

    impl TestArtifactRepository {
        pub fn new(repo_path: &str ) -> Self {
            return TestArtifactRepository {
                repo_path: repo_path.to_string(),
                fetches: RwLock::new(HashSet::new()),
            };
        }
    }

    impl ArtifactRepository for TestArtifactRepository {
        fn fetch(&self, bundle: &ArtifactBundle) -> Result<(), Error> {
            {
                let lock = self.fetches.read()?;
                if lock.contains(bundle) {
                    return Ok(());
                }
            }

            let mut lock = self.fetches.write()?;
            lock.insert(bundle.clone());

            // at this time we don't do anything
            // later we will pull a zip file from a public repository and
            // extract the files to 'repo_path'
            return Ok(());
        }
    }

    impl ArtifactCache for TestArtifactRepository {
        fn cache(&self, artifact: &Artifact) -> Result<(), Error > {


            self.fetch( &artifact.bundle )?;
            /*

            let mut cache = self.cache.write()?;
            if cache.contains_key(artifact) {
                return Ok(());
            }
             */
            //        let mut cache = cell.borrow_mut();
//        let string = String::from_utf8(self.load(artifact)?)?;
//        cache.insert(artifact.clone(), Arc::new(string));
            return Ok(());
        }

        fn load(&self, artifact: &Artifact) -> Result<Vec<u8>, Error > {
            {
                let lock = self.fetches.read()?;
                if !lock.contains(&artifact.bundle) {
                    return Err(format!(
                        "fetch must be called on bundle: {} before artifact can be loaded: {}",
                        artifact.bundle.to(),
                        artifact.to()
                    )
                        .into());
                }
            }

            let mut path = String::new();
            path.push_str(self.repo_path.as_str());
            if !self.repo_path.ends_with("/") {
                path.push_str("/");
            }
            path.push_str(artifact.bundle.group.as_str());
            path.push_str("/");
            path.push_str(artifact.bundle.id.as_str());
            path.push_str("/");
            path.push_str(artifact.bundle.version.to_string().as_str());
            path.push_str("/");
            path.push_str(artifact.path.as_str());

            let mut file = File::open(path)?;
            let mut data = Vec::new();
            file.read_to_end(&mut data)?;
            return Ok(data);
        }

        /*
        fn get(&self, artifact: &Artifact) -> Result<Arc<String>, Error > {
            let cache = self.cache.read()?;
            let option = cache.get(artifact);

            match option {
                None => Err(format!("artifact is not cached: {}", artifact.to()).into()),
                Some(rtn) => Ok(rtn.clone()),
            }
        }
         */
    }
}

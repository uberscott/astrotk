
use crate::artifact::Artifact;
use std::collections::HashMap;
use no_proto::buffer::{NP_Buffer, NP_Finished_Buffer};
use no_proto::NP_Factory;
use no_proto::error::NP_Error;
use crate::buffers::{BufferFactories, Buffer, RO_Buffer, Path};
use no_proto::pointer::{NP_Scalar, NP_Value};
use bytes::Bytes;
use std::error::Error;
use uuid::Uuid;
use std::sync::Arc;
use crate::id::{Id, IdSeq, TronKey, Revision, DeliveryMomentKey};
use crate::configs::Configs;
use no_proto::memory::NP_Memory_Owned;


static MESSAGE_SCHEMA: &'static str = r#"{
    "type":"list",
    "of":
    {"type": "table",
    "columns": [
        ["id",   {"type": "table", "columns":[["seq_id":{"type":"i64"}],["id":{"type":"i64"}]]}],
        ["kind",   {"type": "u8"}],
        ["from",    {"type": "table", "columns":[["nucleus_id",   {"type": "table", "columns":[["seq_id":{"type":"i64"}],["id":{"type":"i64"}]]}],["tron_id",   {"type": "table", "columns":[["seq_id":{"type":"i64"}],["id":{"type":"i64"}]]}],["cycle",{"type":"i64"}],["timestamp",{"type":"i64"}]]}],
        ["to",      {"type": "table", "columns":[["nucleus_id",   {"type": "table", "columns":[["seq_id":{"type":"i64"}],["id":{"type":"i64"}]]}],["tron_id",   {"type": "table", "columns":[["seq_id":{"type":"i64"}],["id":{"type":"i64"}]]}],["cycle",{"type":"i64"}],["phase",{"type":"u8"}]]},["inter_delivery_type", {"type": "enum", "choices": ["cyclic", "phasic"], "default": "cyclic"}],["port",   {"type": "string"}]],

        ["payloads",   {"type": "list", "of":{ "type":"table", "columns": [ ["buffer", {"type","bytes"}], ["artifact", {"type","string"}] ]  }}],

        ["meta",   {"type": "map","value": { "type": "string" } }],
        ["transaction",   {"type": "table", "columns":[["seq_id":{"type":"i64"}],["id":{"type":"i64"}]]}]
        ]
}"#;


static MESSAGE_BUILDERS_SCHEMA: &'static str = r#"{
    "type":"list",
    "of":
    {"type": "table",
    "columns": [
        ["kind",   {"type": "i32"}],

        ["to_tron",  {"type": "table", "columns":[["nucleus_id",   {"type": "table", "columns":[["seq_id":{"type":"i64"}],["id":{"type":"i64"}]]}],["tron_id",   {"type": "table", "columns":[["seq_id":{"type":"i64"}],["id":{"type":"i64"}]]}]]}],

        ["to_nucleus_lookup_name",      {"type": "string"}],
        ["to_tron_lookup_name",      {"type": "string"}],
        ["to_cycle_kind",      {"type": "i64"}],
        ["to_cycle",      {"type": "i64"}],
        ["to_phase_name",      {"type": "string"}],
        ["to_phase",      {"type": "u8"}],
        ["to_inter_delivery_type", {"type": "enum", "choices": ["cyclic", "phasic"], "default": "cyclic"}],
        ["to_port",   {"type": "string"}],

        ["payloads",   {"type": "list", "of":{ "type":"table", "columns": [ ["buffer", {"type","bytes"}], ["artifact", {"type","string"}] ]  }}],
        ["meta",   {"type": "map","value": { "type": "string" } }],
        ["transaction",   {"type": "table", "columns":[["seq_id":{"type":"i64"}],["id":{"type":"i64"}]]}]
        ]
}"#;

lazy_static! {
static ref MESSAGES_FACTORY : NP_Factory<'static> = NP_Factory::new_json(MESSAGE_SCHEMA).unwrap();
static ref MESSAGE_BUILDERS_FACTORY : NP_Factory<'static> = NP_Factory::new_json(MESSAGE_BUILDERS_SCHEMA).unwrap();
}


#[derive(Clone)]
pub struct To {
    pub tron: TronKey,
    pub port: String,
    pub cycle: Cycle,
    pub phase: u8,
    pub inter_delivery_type: InterDeliveryType,
}

#[derive(Clone)]
pub struct From {
    pub tron: TronKey,
    pub cycle: i64,
    pub timestamp: i64
}

impl From
{
    pub fn append( &self, path: &Path, buffer: &mut Buffer )->Result<(),Box<dyn Error>>
    {
        Ok(())
    }

    pub fn from( path: &Path, buffer: &RO_Buffer)->Result<Self,Box<dyn Error>>
    {
        unimplemented!()
    }

}



impl To
{
    pub fn basic( tron: TronKey, port: String ) -> Self
    {
        To{
            tron:tron,
            port: port,
            cycle: Cycle::Next,
            phase: 0,
            inter_delivery_type: InterDeliveryType::Cyclic
        }
    }

    pub fn phasic( tron: TronKey, port: String, phase: u8 ) -> Self
    {
        To{
            tron:tron,
            port: port,
            cycle: Cycle::Next,
            phase: phase,
            inter_delivery_type: InterDeliveryType::Cyclic
        }
    }

    pub fn inter_phasic( tron: TronKey, port: String, phase: u8 ) -> Self
    {
        To{
            tron:tron,
            port: port,
            cycle: Cycle::Present,
            phase: phase,
            inter_delivery_type: InterDeliveryType::Phasic
        }
    }

    pub fn append( &self, path: &Path, buffer: &mut Buffer )->Result<(),Box<dyn Error>>
    {
        self.tron.append(&path.push(path!("tron")), buffer )?;
        buffer.set(&path.with(path!("port")), self.port.clone() )?;
        //buffer.set(vec!["port".to_string()], self.port.clone() )?;

        match self.cycle{
            Cycle::Exact(cycle) => {
                buffer.set(&path.with(path!("cycle")), cycle.clone() )?;
            }
            Cycle::Present => {}
            Cycle::Next => {}
        }

        buffer.set(&path.with(path!("phase")), self.phase.clone() )?;

        match self.inter_delivery_type{
            InterDeliveryType::Cyclic =>
                {
                    buffer.set(&path.with(path!("delivery_moment")), "cyclic" )?;
                }
            InterDeliveryType::Phasic => {
                buffer.set(&path.with(path!("delivery_moment")), "phasic" )?;
            }
        }

        Ok(())
    }

    pub fn from( path: &Path, buffer: &RO_Buffer)->Result<Self,Box<dyn Error>>
    {
        let tron = TronKey::from( &path.push(path!("tron")),buffer )?;
        let port = buffer.get::<String>( &path.with(path!["port"]))?;
        let cycle = match buffer.is_set::<i64>( &path.with(path!("cycle")))?
        {
            true => {
                Cycle::Exact(buffer.get::<i64>( &path.with(path!("cycle")))?)
            }
            false => {
                Cycle::Next
            }
        };

        let phase = buffer.get::<u8>(&path.with(path!["phase"]))?;
        let delivery_moment = match buffer.get::<&str>(&path.with(path!("delivery_moment")))?
        {
            "cyclic" => InterDeliveryType::Cyclic,
            "phasic" => InterDeliveryType::Phasic,
            _ => return Err("encountered unknown delivery_moment".into())
        };

        Ok(To{
            tron: tron,
            port: port,
            cycle: cycle,
            phase: phase,
            inter_delivery_type: delivery_moment
        })
    }

}


#[derive(Clone)]
pub enum Cycle{
    Exact(i64),
    Present,
    Next
}

#[derive(Clone)]
pub enum MessageKind{
    Create,
    Update,
    Content,
    Request,
    Response,
    Reject
}

fn message_kind_to_index(kind: &MessageKind ) -> u8
{
    match kind {
        MessageKind::Create=>0,
        MessageKind::Update=>1,
        MessageKind::Content =>2,
        MessageKind::Request =>3,
        MessageKind::Response =>4,
        MessageKind::Reject=>5
    }
}

fn index_to_message_kind( index: u8 ) -> Result<MessageKind,Box<dyn Error>>
{
    match index {
        0 => Ok(MessageKind::Create),
        1 => Ok(MessageKind::Update),
        2 => Ok(MessageKind::Content),
        3 => Ok(MessageKind::Request),
        4 => Ok(MessageKind::Response),
        5 => Ok(MessageKind::Reject),
        _ => Err(format!("invalid index {}",index).into())
    }
}


// meaning the "between" delivery which can either be between cycles or phases
#[derive(Clone)]
pub enum InterDeliveryType
{
    Cyclic,
    Phasic
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
    pub to_inter_delivery_type: Option<InterDeliveryType>,
    pub payloads: Option<Vec<Payload>>,
    pub meta: Option<HashMap<String,String>>,
    pub transaction: Option<Id>
}

impl  MessageBuilder {
    pub fn new( )->Self
    {
        MessageBuilder{
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
            to_inter_delivery_type: None,
            payloads: None,
            meta: None,
            transaction: None,
        }
    }

    pub fn validate(&self) ->Result<(),Box<dyn Error>>
    {
        if self.kind.is_none()
        {
            return Err("message builder kind must be set".into());
        }

        if self.to_phase_name.is_some() && self.to_phase.is_some()
        {
            return Err("to_phase_name and to_phase cannot both be set".into());
        }

        if self.to_nucleus_lookup_name.is_some() != self.to_nucleus_id.is_some()
        {
            return Err("message builder to_nucleus_lookup_name OR to_nucleus_id must be set (but not both)".into());
        }

        if self.to_tron_lookup_name.is_some() != self.to_tron_id.is_some()
        {
            return Err("message builder to_tron_lookup_name OR to_tron_id must be set (but not both)".into());
        }

        if self.to_cycle_kind.is_none()
        {
            return Err("message builder to_cycle_kind OR to_cycle must be set (but not both)".into());
        }
        if self.payloads.is_none()
        {
            return Err("message builder payload must be set".into());
        }

        Ok(())
    }

    pub fn validate_build(&self) -> Result<(),Box<dyn Error>>
    {
        self.validate()?;

        if self.to_nucleus_id.is_some()
        {
            return Err("message builder to_nucleus_id must be set before build".into());
        }

        if self.to_tron_id.is_some()
        {
            return Err("message builder to_tron_id must be set before build".into());
        }

        Ok(())
    }

    pub fn build(&self, seq: &mut IdSeq) -> Result<Message,Box<dyn Error>>
    {
        self.validate_build()?;
        Ok(Message{
            id: seq.next(),
            kind: self.kind.clone().unwrap(),
            from: self.from.clone().unwrap(),
            to: To {
                tron: TronKey { nucleus: self.to_nucleus_id.clone().unwrap(),
                                tron: self.to_tron_id.clone().unwrap() },
                port: self.to_port.clone().unwrap(),
                cycle: self.to_cycle_kind.clone().unwrap(),
                phase: self.to_phase.clone().unwrap(),
                inter_delivery_type: match &self.to_inter_delivery_type {
                    Some(r)=>r.clone(),
                    None=>InterDeliveryType::Cyclic
                }
            },
            payloads: vec![],
            meta: self.meta.clone(),
            transaction: self.transaction.clone()
        })
    }

    pub fn message_builders_to_buffer( builders: Vec<MessageBuilder> )->Result<NP_Buffer<NP_Memory_Owned> ,Box<dyn Error>>
    {
        let mut buffer= MESSAGE_BUILDERS_FACTORY.new_buffer(Option::None);
        let mut index = 0;
        for b in builders
        {
            let result = b.append_to_buffer(&mut buffer,index);
            match result{
                Ok(_)=>{},
                Err(e)=>return Err(format!("error when append_to_buffer {}",e.to_string()).into())
            };
            index=index+1;
        }
        return Ok(buffer);
    }


    pub fn append_to_buffer(&self, buffer: &mut NP_Buffer<NP_Memory_Owned>, index: usize ) -> Result<(),Box<dyn Error>>
    {
        self.validate()?;
        let result = self.append_to_buffer_np_error(buffer,index);
        match result{
            Ok(_) => Ok(()),
            Err(e) => Err("np_error".into())
        }
    }

    fn append_to_buffer_np_error( &self, buffer: &mut NP_Buffer<NP_Memory_Owned>, index: usize) ->Result<(),NP_Error>
    {

        let index = index.to_string();
//        buffer.set(&[&index, &"kind"], message_kind_to_index(&self.kind.as_ref().unwrap()))?;


        Ok(())
    }
}


#[derive(Clone)]
pub struct PayloadBuilder {
    pub buffer: Buffer,
    pub artifact: Artifact
}

impl PayloadBuilder
{
    pub fn build(builder:PayloadBuilder)->Payload
    {
        Payload{
            artifact: builder.artifact,
            buffer: Buffer::read_only(builder.buffer)
        }
    }
}



#[derive(Clone)]
pub struct Payload {
    pub buffer: RO_Buffer,
    pub artifact: Artifact
}

#[derive(Clone)]
pub struct Message {
    pub id: Id,
    pub kind: MessageKind,
    pub from: From,
    pub to: To,
    pub payloads: Vec<Payload>,
    pub meta: Option<HashMap<String,String>>,
    pub transaction: Option<Id>,
}


impl Message {

    pub fn single_payload(seq: & mut IdSeq,
                 kind: MessageKind,
                 from: From,
                 to: To,
                 payload: Payload ) -> Self
    {
        Message::multi_payload( seq, kind, from, to, vec!(payload) )
    }

    pub fn multi_payload(seq: & mut IdSeq,
                 kind: MessageKind,
                 from: From,
                 to: To,
                 payloads: Vec<Payload> ) -> Self
    {
        Message::longform( seq, kind, from, to, payloads, Option::None, Option::None )
    }



    pub fn longform(seq: & mut IdSeq,
               kind: MessageKind,
               from: From,
               to: To,
               payloads: Vec<Payload>,
               meta: Option<HashMap<String,String>>,
               transaction: Option<Id> ) -> Self
    {
        Message{
            id: seq.next(),
            kind: kind,
            from: from,
            to: to,
            payloads: payloads,
            meta: meta,
            transaction: transaction
        }
    }

    pub fn calc_bytes(&self)->usize
    {
        let mut size= 0;
        for payload in &self.payloads
        {
            size = size+ payload.buffer.size();
        }
        return size;
    }

    pub fn to_bytes(&mut self) -> Result<Vec<u8>,Box<dyn Error>>
    {
        let mut buffer= Buffer::new(MESSAGES_FACTORY.new_buffer(Option::Some(self.calc_bytes())));
        buffer.set(&path!("kind"), message_kind_to_index(&self.kind) )?;


        let path = Path::new(path!("from"));
        self.from.append(&path,&mut buffer)?;

        let path = Path::new(path!("to"));
        self.to.append(&path,&mut buffer)?;

        match &self.to.cycle
        {
            Cycle::Exact(c)=>buffer.set(&path!(&"to", &"cycle"), c.clone())?,
            _ => false
        };

        let mut payload_index = 0;
        for payload in &self.payloads
        {
            buffer.set(&path!("payloads", payload_index.to_string(), "buffer" ), payload.buffer.read_bytes() )?;
            buffer.set(&path!(&"payloads", &payload_index.to_string(), "artifact"), payload.artifact.to() )?;
            payload_index = payload_index+1;
        }

        if self.meta.is_some() {
            let meta = self.meta.clone().unwrap();
            for k in meta.keys()
            {
                buffer.set(&path!(&"meta",&k.as_str()), meta.get(k).unwrap().to_string())?;
            }
        }

        if self.transaction.is_some()
        {
            let transaction = &self.transaction.clone().unwrap();
            transaction.append(&Path::new(path!("transaction")),&mut buffer);
        }

        buffer.compact()?;

        Ok((Buffer::bytes(buffer)))
    }

    /*
    pub fn from( configs: &Configs, bytes: Vec<u8> ) -> Result<Self,Box<dyn Error>>
    {


        return Ok(message);
    }

     */


}

fn cat( path: &[&str])->String
{
    let mut rtn = String::new();
    for segment in path{
        rtn.push_str(segment);
        rtn.push('/');
    }
    return rtn;
}


/*
#[cfg(test)]
mod tests {
    use crate::artifact_config::ArtifactFile;
    use main::data;
    use crate::buffers::BufferFactories;
    use main::data::AstroTK;
    use no_proto::NP_Factory;
    use crate::message::{MESSAGE_SCHEMA, MessageKind, Message, Address};

    #[test]
    fn check_schema() {
        NP_Factory::new( MESSAGE_SCHEMA ).unwrap();
    }

    #[test]
    fn create_message() {
        let mut astroTK = AstroTK::new();
        let artifact_file = ArtifactFile::from("main:actor:1.0.0-alpha:content.json").unwrap();
        astroTK.load_buffer_factory(&artifact_file).unwrap();

        let mut payload = astroTK.create_buffer(&artifact_file).unwrap();

        payload.set(&[&"name"], "Fred Jarvis");

        let message = Message::new(MessageKind::Request,
                                   Address { actor: 0 },
                                   Address { actor: 1 },
                                   "some-port".to_string(),
                                   payload,
                                   artifact_file.clone());

        let messages_buffer = Message::messages_to_buffer( &[&message] ).unwrap();

        let rtn_messages = Message::messages_from_buffer(&astroTK, &messages_buffer ).unwrap();

        assert_eq!(  1, rtn_messages.len() );



    }
}

 */




use crate::artifact::Artifact;
use std::collections::HashMap;
use no_proto::buffer::{NP_Buffer, NP_Finished_Buffer};
use no_proto::NP_Factory;
use no_proto::error::NP_Error;
use crate::buffers::BufferFactories;
use no_proto::pointer::{NP_Scalar, NP_Value};
use bytes::Bytes;
use std::error::Error;
use uuid::Uuid;
use std::sync::Arc;
use no_proto::memory::{NP_Memory_Owned, NP_Memory, NP_Mem_New};
use std::sync::Mutex;
use crate::id::{Id, IdSeq, TronKey, Revision};


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
static ref MESSAGES_FACTORY : NP_Factory<'static> = NP_Factory::new(MESSAGE_SCHEMA).unwrap();
static ref MESSAGE_BUILDERS_FACTORY : NP_Factory<'static> = NP_Factory::new(MESSAGE_BUILDERS_SCHEMA).unwrap();
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

        if self.to_cycle_kind.is_some() != self.to_cycle.is_some()
        {
            return Err("message builder to_cycle_kind OR to_cycle must be set (but not both)".into());
        }
        if self.payload.is_none()
        {
            return Err("message builder payload must be set".into());
        }
        if self.payload_artifact.is_none()
        {
            return Err("message builder payload_artifact must be set".into());
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
            kind: self.kind.unwrap(),
            from: self.from.unwrap(),
            to: To {
                tron: TronKey { nucleus_id: self.to_nucleus_id.unwrap().clone(),
                                tron_id: self.to_tron_id.unwrap().clone() },
                port: self.to_port.unwrap().clone(),
                cycle: self.to_cycle_kind.unwrap().clone(),
                phase: self.to_phase.unwrap(),
                inter_delivery_type: match &self.to_inter_delivery_type {
                    Some(r)=>r.clone(),
                    None=>Option::None
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
        buffer.set(&[&index, &"kind"], message_kind_to_index(&self.kind.as_ref().unwrap()))?;

        if self.to_nucleus_lookup_name.is_some() {
          buffer.set(&[&index, &"to_nucleus_lookup_name"], self.to_nucleus_lookup_name.as_ref().unwrap().as_str())?;
        }

        if self.to_tron_lookup_name.is_some() {
            buffer.set(&[&index, &"to_tron_lookup_name"], self.to_tron_lookup_name.as_ref().unwrap().as_str())?;
        }

        if self.to_nucleus_id.is_some() {
            buffer.set(&[&index, &"to_nucleus_id", &"seq_id"], self.to_nucleus_id.unwrap().seq_id)?;
            buffer.set(&[&index, &"to_nucleus_id", &"id"], self.to_nucleus_id.unwrap().id)?;
        }

        if self.to_tron_id.is_some() {
            buffer.set(&[&index, &"to_tron_id", &"seq_id"], self.to_tron_id.unwrap().seq_id)?;
            buffer.set(&[&index, &"to_tron_id", &"id"], self.to_tron_id.unwrap().id)?;
        }

        buffer.set(&[&index, &"to_port"], self.to_port.unwrap())?;


        if self.to_inter_delivery_type.is_some()
        {
            buffer.set(&[&index, &"to_inter_delivery_type"], match &self.to_inter_delivery_type.unwrap(){
                InterDeliveryType::Cyclic=>"cyclic",
                InterDeliveryType::Phasic=>"phasic",
            } )?;
        }

        buffer.set( &[&index,&"payload"], self.payload.as_ref().unwrap().read_bytes() )?;
        buffer.set( &[&index,&"payload_artifact"], self.payload_artifact.as_ref().unwrap().to() )?;

        if self.meta.is_some()
        {
            for k in self.meta.as_ref().unwrap().keys()
            {
                buffer.set(&[&index, &"meta", k], self.meta.as_ref().unwrap().get(k).as_ref().unwrap().to_string())?;
            }
        }

        if self.transaction.is_some()
        {
            let transaction = self.transaction.as_ref().unwrap();
            buffer.set(&[&index,&"transaction", &"seq_id"], transaction.seq_id )?;
            buffer.set(&[&index,&"transaction", &"id"], transaction.id)?;
        }

        Ok(())
    }
}


#[derive(Clone)]
pub struct PayloadBuilder {
    pub buffer: Arc<NP_Buffer<NP_Memory_Owned>>,
    pub artifact: Artifact
}

impl PayloadBuilder
{
    pub fn build(&self)->Result<Payload,Box<dyn Error>>
    {
        Ok(Payload {
            buffer: Arc::new(self.buffer.finish()),
            artifact: self.artifact.clone()
        })
    }
}


#[derive(Clone)]
pub struct Payload {
    pub buffer: Arc<NP_Finished_Buffer<NP_Memory_Owned>>,
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

    pub fn messages_to_buffer<'message,'buffer> ( messages: &[&'message Message] )->Result<NP_Buffer<NP_Memory_Owned> ,Box<dyn Error>>
    {
        let mut buffer= MESSAGES_FACTORY.new_buffer(Option::None);
        let mut index = 0;
        for m in messages
        {
            let result = m.append_to_buffer(&mut buffer,index);
            match result{
                Ok(_)=>{},
                Err(_)=>return Err("error when append_to_buffer".into())
            }
            index=index+1;
        }
        return Ok(buffer);
    }

    pub fn append_to_buffer<M: NP_Memory + Clone + NP_Mem_New>(&self, buffer: &mut NP_Buffer<M>, index: usize ) -> Result<(),Box<NP_Error>>
    {
        let index = index.to_string();
        buffer.set( &[&index,&"kind"], message_kind_to_index(&self.kind) )?;
        buffer.set(&[&index, &"from", &"nucleus_id",&"seq_id"],&self.from.tron.nucleus_id.seq_id)?;
        buffer.set(&[&index, &"from", &"nucleus_id",&"id"],&self.from.tron.nucleus_id.id)?;
        buffer.set(&[&index, &"from", &"tron_id",&"seq_id"],&self.from.tron.tron_id.seq_id)?;
        buffer.set(&[&index, &"from", &"tron_id",&"id"],&self.from.tron.tron_id.id)?;
        buffer.set(&[&index, &"from", &"cycle"], self.from.cycle.clone())?;
        buffer.set(&[&index, &"from", &"timestamp"], self.from.timestamp.clone())?;

        buffer.set(&[&index, &"to", &"nucleus_id",&"seq_id"],&self.to.tron.nucleus_id.seq_id)?;
        buffer.set(&[&index, &"to", &"nucleus_id",&"id"],&self.to.tron.nucleus_id.id)?;
        buffer.set(&[&index, &"to", &"tron_id",&"seq_id"],&self.to.tron.tron_id.seq_id)?;
        buffer.set(&[&index, &"to", &"tron_id",&"id"],&self.to.tron.tron_id.id)?;
        buffer.set(&[&index, &"to", &"phase"], self.to.phase)?;
        buffer.set(&[&index, &"to", &"port"], self.to.port.clone() )?;
        buffer.set(&[&index, &"to", &"inter_delivery_type"], match &self.inter_delivery_type {
            InterDeliveryType::Cyclic=>"cyclic",
            InterDeliveryType::Phasic=>"phasic"
        } )?;

        match &self.to.cycle
        {
            Cycle::Exact(c)=>buffer.set(&[&index, &"to", &"cycle"], c.clone())?,
            Cycle::Next=> false
        };



        buffer.set( &[&index,&"payload"], self.payload.read_bytes() )?;
        buffer.set( &[&index,&"payload_artifact"], self.payload_artifact.to() )?;

        for k in self.meta.keys()
        {
            buffer.set( &[&index,&"meta",k], self.meta.get(k).unwrap().to_string())?;
        }

        if self.transaction.is_some()
        {
            let transaction = self.transaction.as_ref().unwrap();
            buffer.set(&[&index,&"transaction"], transaction.clone() )?;
        }

        Ok(())
    }

    pub fn from_buffer<M: NP_Memory + Clone + NP_Mem_New>(buffer_factories: & dyn BufferFactories, buffer: &NP_Buffer<M>, index: usize ) -> Result<Self,Box<dyn Error>>
    {
        let index = index.to_string();
        let payload_artifact = Message::get::<String,M>(&buffer, &[&index,&"payload_artifact"])?;
        let payload_artifact = Artifact::from(&payload_artifact)?;
        let payload = Message::get::<Vec<u8>,M>(&buffer, &[&index,&"payload"])?;

        let mut meta: HashMap<String,String> = HashMap::new();
        if buffer.get_collection( &[&index,&"meta"]).unwrap().is_some()
        {
            for item in buffer.get_collection(&[&index, &"meta"]).unwrap().unwrap()
            {
                meta.insert(item.key.to_string(), item.get::<String>().unwrap().unwrap());
            }
        }

        let meta= meta;

        let message = Message {
            id: Id{
                seq_id: Message::get::<i64,M>(&buffer, &[&index,&"id",&"seq_id"])?,
                id: Message::get::<i64,M>(&buffer, &[&index,&"id",&"id"])?
            },
            kind: index_to_message_kind(Message::get::<u8,M>(&buffer, &[&index, &"kind"])?)?,
            from:
                    From{ tron: TronKey{ nucleus_id : Id{ seq_id: Message::get::<i64,M>(&buffer, &[&index,&"from",&"tron",&"nucleus_id",&"seq_id"])?,
                                                          id: Message::get::<i64,M>(&buffer, &[&index,&"from",&"tron",&"nucleus_id",&"id"])? },
                                             tron_id: Id{ seq_id: Message::get::<i64,M>(&buffer, &[&index,&"from",&"tron",&"tron_id",&"seq_id"])?,
                                                          id: Message::get::<i64,M>(&buffer, &[&index,&"from",&"tron",&"tron_id",&"id"])? }},
                    timestamp: Message::get::<i64,M>(&buffer, &[&index,&"from",&"timestamp"])?,
                    cycle: Message::get::<i64,M>(&buffer, &[&index,&"from",&"cycle"])?

                    } ,


                to:To{ tron: TronKey{ nucleus_id : Id{ seq_id: Message::get::<i64,M>(&buffer, &[&index,&"to",&"tron",&"nucleus_id",&"seq_id"])?,
                    id: Message::get::<i64,M>(&buffer, &[&index,&"to",&"tron",&"nucleus_id",&"id"])? },
                    tron_id: Id{ seq_id: Message::get::<i64,M>(&buffer, &[&index,&"to",&"tron",&"tron_id",&"seq_id"])?,
                        id: Message::get::<i64,M>(&buffer, &[&index,&"to",&"tron",&"tron_id",&"id"])? }},
                    cycle: match buffer.get::<i64>(&[&index,&"to",&"cycle"]).unwrap() {
                        Some(cycle) => Cycle::Exact(cycle),
                        None => Cycle::Next
                    },
                    phase:Message::get::<u8,M>(&buffer, &[&index,&"to",&"phase"])?,
                    port:Message::get::<String,M>(&buffer, &[&index,&"to",&"port"])?,
                    inter_delivery_type: match Message::get::<String,M>(&buffer, &[&index,&"to",&"inter_delivery_type"])?{
                       &"cyclic"=>InterDeliveryType::Cyclic,
                        &"phasic"=>InterDeliveryType::Phasic,
                        _ => {}
                    }
                },

            payloads: { let length = buffer.get_length(&[] )?.unwrap();
                        let mut rtn = vec!();
                        for payload_index in 0..length {
                            let artifact = Artifact::from(Message::get:: <String, M>( & buffer, &[ & index, &"payloads", &payload_index, &"artifact"])?.as_str())?;
                            let bytes = Message::get:: <Vec<u8>, M>( & buffer, &[ & index, &"payloads", &payload_index, &"buffer"])?;
                            let payload = Payload { buffer: Arc::new(buffer_factories.create_buffer_from(&artifact, bytes)?), artifact: artifact };
                            rtn.push(payload);
                        }
                        rtn},


            meta: Option::None,
            transaction: match buffer.get::<i64>( &[&index,&"transaction", &"seq_id"] )?{
                None=>Option::None,
                Some(seq_id)=>Some( Id{ seq_id:seq_id,
                                              id: buffer.get::<i64>( &[&index,&"transaction", &"seq_id"] )?.unwrap()
                 })
            }
        };
        return Ok(message);
    }

    pub fn messages_from_bytes(  seq: &mut IdSeq,buffer_factories: & dyn BufferFactories, bytes: &Bytes) -> Result<Vec<Self>,Box<dyn Error>>
    {
        let buffer = MESSAGES_FACTORY.open_buffer( bytes.to_vec() );
        return Ok( Message::messages_from_buffer( buffer_factories, &buffer)? );
    }

    pub fn messages_from_buffer<M: NP_Memory + Clone + NP_Mem_New>( buffer_factories: & dyn BufferFactories, buffer: &NP_Buffer<M> ) -> Result<Vec<Self>,Box<dyn Error>>
    {
        let length = buffer.get_length(&[] )?.unwrap();

        let mut rtn = vec![];
        for index in 0..length
        {
            rtn.push(Message::from_buffer( buffer_factories, buffer, index )?);
        }

        return Ok(rtn);
    }

    pub fn get<'get, X: 'get,M: NP_Memory + Clone + NP_Mem_New>(buffer:&'get NP_Buffer<M>, path: &[&str]) -> Result<X, Box<dyn Error>> where X: NP_Value<'get> + NP_Scalar<'get> {
       match buffer.get::<X>(path)
       {
          Ok(option)=>{
              match option{
                  Some(rtn)=>Ok(rtn),
                  None=>Err(format!("expected a value for {}", path[path.len()-1] ).into())
              }
          },
          Err(e)=>Err(format!("could not get {}",cat(path)).into())
       }
    }


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


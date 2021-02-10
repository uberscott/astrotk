
use crate::artifact::Artifact;
use std::collections::HashMap;
use no_proto::buffer::NP_Buffer;
use no_proto::NP_Factory;
use no_proto::error::NP_Error;
use crate::buffers::BufferFactories;
use no_proto::pointer::{NP_Scalar, NP_Value};
use bytes::Bytes;
use std::error::Error;
use uuid::Uuid;
use std::sync::Arc;


static MESSAGE_SCHEMA: &'static str = r#"{
    "type":"list",
    "of":
    {"type": "table",
    "columns": [
        ["uuid",   {"type": "string"}],
        ["kind",   {"type": "i32"}],
        ["from",    {"type": "table", "columns":[["nucleus",{"type":"i64"}],["tron",{"type":"i64"}],["cycle",{"type":"i64"}]]}],
        ["to",      {"type": "table", "columns":[["nucleus",{"type":"i64"}],["tron",{"type":"i64"}],["cycle",{"type":"i64"}]]}],
        ["port",   {"type": "string"}],
        ["payload",   {"type": "bytes"}],
        ["payload_artifact",   {"type": "string"}],
        ["meta",   {"type": "map","value": { "type": "string" } }],
        ["transaction",   {"type": "string"}]
        ]
}"#;


static MESSAGE_BUILDERS_SCHEMA: &'static str = r#"{
    "type":"list",
    "of":
    {"type": "table",
    "columns": [
        ["kind",   {"type": "i32"}],

        ["to_nucleus_lookup_name",      {"type": "string"}],
        ["to_tron_lookup_name",      {"type": "string"}],
        ["to_nucleus_id",      {"type": "i64"}],
        ["to_tron_id",      {"type": "i64"}],
        ["to_cycle_kind",      {"type": "i64"}],
        ["to_cycle",      {"type": "i64"}],

        ["port",   {"type": "string"}],
        ["payload",   {"type": "bytes"}],
        ["payload_artifact",   {"type": "string"}],
        ["meta",   {"type": "map","value": { "type": "string" } }],
        ["transaction",   {"type": "string"}]
        ]
}"#;

lazy_static! {
static ref MESSAGES_FACTORY : NP_Factory<'static> = NP_Factory::new(MESSAGE_SCHEMA).unwrap();
static ref MESSAGE_BUILDERS_FACTORY : NP_Factory<'static> = NP_Factory::new(MESSAGE_BUILDERS_SCHEMA).unwrap();
}


#[derive(Clone)]
pub struct Address {
    pub nucleus: i64,
    pub tron: i64,
    pub cycle: Cycle
}


#[derive(Clone)]
pub enum Cycle{
    Some(i64),
    Next
}


#[derive(Clone)]
pub enum MessageKind{
    Info,
    Content,
    Request,
    Response,
    Reject
}

fn message_kind_to_index(kind: &MessageKind ) -> i32
{
    match kind {
        MessageKind::Info =>0,
        MessageKind::Content =>1,
        MessageKind::Request =>2,
        MessageKind::Response =>3,
        MessageKind::Reject=>4
    }
}

fn index_to_message_kind( index: i32 ) -> Result<MessageKind,Box<dyn Error>>
{
    match index {
        0 => Ok(MessageKind::Info),
        1 => Ok(MessageKind::Content),
        2 => Ok(MessageKind::Request),
        3 => Ok(MessageKind::Response),
        4 => Ok(MessageKind::Reject),
        _ => Err(format!("invalid index {}",index).into())
    }
}


#[derive(Clone)]
pub struct MessageBuilder<'a> {
    pub kind: Option<MessageKind>,
    pub to_nucleus_lookup_name: Option<String>,
    pub to_nucleus_id: Option<i64>,
    pub to_tron_lookup_name: Option<String>,
    pub to_tron_id: Option<i64>,
    pub to_cycle_kind: Option<Cycle>,
    pub to_cycle: Option<i64>,
    pub port: Option<String>,
    pub payload: Option<NP_Buffer<'a>>,
    pub payload_artifact: Option<Artifact>,
    pub meta: Option<HashMap<String,String>>,
    pub transaction: Option<String>
}

impl <'a> MessageBuilder<'a> {
    pub fn new( )->Self
    {
        MessageBuilder{
            kind: None,
            to_nucleus_lookup_name: None,
            to_tron_lookup_name: None,
            to_nucleus_id: None,
            to_tron_id: None,
            to_cycle_kind: None,
            to_cycle: None,
            port: None,
            payload: None,
            payload_artifact: None,
            meta: None,
            transaction: None
        }
    }

    pub fn validate(&self) ->Result<(),Box<dyn Error>>
    {
        if self.kind.is_none()
        {
            return Err("message builder kind must be set".into());
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
        if self.port.is_none()
        {
            return Err("message builder port must be set".into());
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

    pub fn message_builders_to_buffer<'buffer,'builder> ( builders: Vec<MessageBuilder<'builder>> )->Result<NP_Buffer<'buffer> ,Box<dyn Error>>
    {
        let mut buffer:NP_Buffer = MESSAGE_BUILDERS_FACTORY.empty_buffer(Option::None);
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


    pub fn append_to_buffer(&self, buffer: &mut NP_Buffer, index: usize ) -> Result<(),Box<dyn Error>>
    {
        self.validate()?;
        let result = self.append_to_buffer_np_error(buffer,index);
        match result{
            Ok(_) => Ok(()),
            Err(e) => Err(e.message.into())
        }
    }

    fn append_to_buffer_np_error( &self, buffer: &mut NP_Buffer, index: usize) ->Result<(),NP_Error>
    {
        let index = index.to_string();
        buffer.set(&[&index, &"kind"], message_kind_to_index(&self.kind.as_ref().unwrap()))?;

        if self.to_nucleus_lookup_name.is_some() {
          buffer.set(&[&index, &"to_nucleus_lookup_name"], self.to_nucleus_lookup_name.as_ref().unwrap().as_str())?;
        }

        if self.to_nucleus_id.is_some() {
            buffer.set(&[&index, &"to_nucleus_id"], self.to_nucleus_id.unwrap())?;
        }
        if self.to_tron_lookup_name.is_some() {
            buffer.set(&[&index, &"to_tron_lookup_name"], self.to_tron_lookup_name.as_ref().unwrap().as_str())?;
        }

        if self.to_tron_id.is_some() {
            buffer.set(&[&index, &"to_tron_id"], self.to_tron_id.unwrap())?;
        }

        buffer.set( &[&index,&"port"], self.port.as_ref().unwrap().as_str() )?;
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
            buffer.set(&[&index,&"transaction"], transaction.clone() )?;
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct Message<'b> {
    pub uuid: String,
    pub kind: MessageKind,
    pub from: Option<Address>,
    pub to: Address,
    pub port: String,
    pub payload: Arc<NP_Buffer<'b>>,
    pub payload_artifact: Artifact,
    pub meta: HashMap<String,String>,
    pub transaction: Option<String>
}


impl <'b> Message <'b> {
    pub fn new(kind: MessageKind,
               to: Address,
               from: Option<Address>,
               port: String,
               payload: NP_Buffer<'b>,
               payload_artifact: Artifact,
                ) -> Self
    {
        Message{
            uuid: Uuid::new_v4().to_string(),
            kind: kind,
            from: from,
            to: to,
            port: port,
            payload: Arc::new(payload),
            payload_artifact: payload_artifact,
            meta: HashMap::new(),
            transaction: Option::None
        }
    }

    pub fn messages_to_buffer<'message,'buffer> ( messages: &[&'message Message] )->Result<NP_Buffer<'buffer> ,Box<dyn Error>>
    {
        let mut buffer:NP_Buffer = MESSAGES_FACTORY.empty_buffer(Option::None);
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

    pub fn append_to_buffer(&self, buffer: &mut NP_Buffer, index: usize ) -> Result<(),Box<NP_Error>>
    {
        let index = index.to_string();
        buffer.set( &[&index,&"kind"], message_kind_to_index(&self.kind) )?;
        if self.from.is_some() {
            let from = (self.from).as_ref().unwrap();
            buffer.set(&[&index, &"from", &"tron"], from.tron)?;
            buffer.set(&[&index, &"from", &"nucleus"], from.nucleus)?;
        }
        buffer.set( &[&index,&"to",&"tron"], self.to.tron )?;
        buffer.set( &[&index,&"to",&"nucleus"], self.to.nucleus)?;

        match self.to.cycle{
            Cycle::Some(cycle) => {
                buffer.set( &[&index,&"to",&"cycle"], cycle)?;
            }
            Cycle::Next => {}
        }

        buffer.set( &[&index,&"port"], self.port.clone() )?;
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

    pub fn from_buffer(buffer_factories: &'b dyn BufferFactories, buffer: &NP_Buffer, index: usize ) -> Result<Self,Box<dyn Error>>
    {
        let index = index.to_string();
        let payload_artifact = Message::get::<String>(&buffer, &[&index,&"payload_artifact"])?;
        let payload_artifact = Artifact::from(&payload_artifact)?;
        let payload = Message::get::<Vec<u8>>(&buffer, &[&index,&"payload"])?;

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
            uuid: Uuid::new_v4().to_string(),
            kind: index_to_message_kind(Message::get::<i32>(&buffer, &[&index, &"kind"])?)?,
            from: match buffer.get::<i64>(&[&index,&"from",&"tron"]).unwrap() {
                None => Option::None,
                Some(_) => Option::Some(
                    Address{ nucleus: Message::get::<i64>(&buffer, &[&index,&"from",&"nucleus"])?,
                        tron: Message::get::<i64>(&buffer, &[&index,&"from",&"tron"])?,
                        cycle: match buffer.get::<i64>(&[&index,&"to",&"cycle"]).unwrap() {
                            Some(cycle) => Cycle::Some(cycle),
                            None => Cycle::Next
                        }
                    }
                )
            },
            to: Address{ nucleus: Message::get::<i64>(&buffer, &[&index,&"to",&"nucleus"])?,
                tron: Message::get::<i64>(&buffer, &[&index,&"to",&"tron"])?,
                cycle: match buffer.get::<i64>(&[&index,&"to",&"cycle"]).unwrap() {
                    Some(cycle) => Cycle::Some(cycle),
                    None => Cycle::Next
                }
            },
            port: Message::get::<String>(&buffer,&[&index,&"port"])?,
            payload: Arc::new(buffer_factories.create_buffer_from( &payload_artifact, payload )?),
            payload_artifact: payload_artifact,
            meta: meta,
            transaction: buffer.get::<String>(&[&index,&"transaction"] ).unwrap()
        };
        return Ok(message);
    }

    pub fn messages_from_bytes(  buffer_factories: &'b dyn BufferFactories, bytes: &Bytes) -> Result<Vec<Self>,Box<dyn Error>>
    {
        let buffer = MESSAGES_FACTORY.open_buffer( bytes.to_vec() );
        return Ok( Message::messages_from_buffer( buffer_factories, &buffer)? );
    }

    pub fn messages_from_buffer( buffer_factories: &'b dyn BufferFactories, buffer: &NP_Buffer ) -> Result<Vec<Self>,Box<dyn Error>>
    {
        let length = match buffer.length(&[] )
        {
            Ok(l)=>l,
            Err(_)=>{
                return Err("could not determine messages length".into());
            }
        }.unwrap();

        let mut rtn = vec![];
        for index in 0..length
        {
            rtn.push(Message::from_buffer(buffer_factories, buffer, index )?);
        }

        return Ok(rtn);
    }

    pub fn get<'get, X: 'get>(buffer:&'get NP_Buffer, path: &[&str]) -> Result<X, Box<dyn Error>> where X: NP_Value<'get> + NP_Scalar<'get> {
       match buffer.get::<X>(path)
       {
          Ok(option)=>{
              match option{
                  Some(rtn)=>Ok(rtn),
                  None=>Err(format!("expected a value for {}", path[path.len()-1] ).into())
              }
          },
          Err(e)=>Err(e.message.into())
       }
    }

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


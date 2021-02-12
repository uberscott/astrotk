use no_proto::buffer::NP_Buffer;
use mechtron_common::message::{Message, MessageBuilder, Payload};
use std::error::Error;
use mechtron_common::artifact::Artifact;
use no_proto::memory::NP_Memory_Owned;
use mechtron_common::id::{TronKey, Revision, NucleusKey, Id, ContentKey};
use mechtron_common::configs::{TronConfig, Configs, MessagesConfig, CreateMessageConfig};
use std::sync::Arc;
use crate::nucleus::NeuTron;
use crate::app::{SYS, Local};
use mechtron_common::content::{Content, ReadOnlyContent};
use crate::content::ContentRetrieval;
use no_proto::error::NP_Error;
use mechtron_common::buffers;
use mechtron_common::buffers::get;

pub trait Tron
{
    fn init (context: Context)->Result<Box<Self>,Box<dyn Error>>  where Self: Sized;

    fn create (&self,
               context: &Context,
               content: &mut Content,
               create: &Message) -> Result<(Option<Vec<MessageBuilder>>), Box<dyn Error>>;

    fn update( &self, phase: &str ) -> Result<fn ( context: &Context, content: &Content) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>>,Box<dyn Error>>;

    fn port( &self, port: &str ) -> Result<fn ( context: &Context, content: &Content, message: &Message ) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>>,Box<dyn Error>>;

    fn update_phases(&self)-> UpdatePhases;
}

pub enum UpdatePhases
{
    All,
    Some(Vec<String>),
    None
}

pub struct MessagePort
{
    pub receive: fn ( context: &Context, content: &Content, message: &Message ) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>>
}

pub struct Context
{
    pub sim_id: Id,
    pub id : TronKey,
    pub revision: Revision,
    pub tron_config: Arc<TronConfig>,
    pub timestamp: i64,
    pub phase: u8
}

impl Context {

    pub fn configs(&self) ->&mut Configs
    {
        return &mut SYS.local.configs;
    }

    pub fn get_content( &self, key:&ContentKey)->Result<ReadOnlyContent,Box<dyn Error>>
    {
        let source = SYS.local.sources.get(&self.sim_id)?;
        if key.revision.cycle >= self.revision.cycle
        {
            return Err(format!("tron {:?} attempted to read the content of tron {:?} in a present or future cycle, which is not allowed",self.id,key.content_id).into());
        }
        let content = source.content.read_only(key)?;
        Ok(content)
    }

    fn lookup_nucleus( &self, context: &Context, name: &str )->Result<Id,Box<dyn Error>>
    {
        let neutron_key = TronKey{ nucleus_id: context.id.nucleus_id.clone(), tron_id: Id::new(context.id.nucleus_id.seq_id,0) };
        let content_key = ContentKey{ tron_id: neutron_key, revision: Revision {cycle:context.revision.cycle-1} };
        let neutron_content= context.get_content(&content_key)?;
        let simulation_nucleus_id= Id::new( neutron_content.data.get::<i64>( &[&"simulation_nucleus_id",&"seq_id"] )?.unwrap(),
                                            neutron_content.data.get::<i64>( &[&"simulation_nucleus_id",&"id"] )?.unwrap() );

        let simtron_key = TronKey{
            nucleus_id: simulation_nucleus_id.clone(),
            tron_id: Id::new(simulation_nucleus_id.seq_id, 1)
        };

        let content_key = ContentKey{ tron_id: simtron_key, revision: Revision {cycle:context.revision.cycle-1} };
        let simtron_content = context.get_content(&content_key)?;

        let nucleus_id = Id::new( simtron_content.data.get::<i64>( &[&"nucleus_names",name,&"seq_id"] )?.unwrap(),
                                  simtron_content.data.get::<i64>( &[&"nucleus_names",name,&"id"] )?.unwrap() );


        Ok(nucleus_id)
    }

    fn lookup_tron( &self, context: &Context, nucleus_id: &Id, name: &str )->Result<TronKey,Box<dyn Error>>
    {
        let neutron_key = TronKey{ nucleus_id: nucleus_id.clone(), tron_id: Id::new(nucleus_id.seq_id,0) };
        let content_key = ContentKey{ tron_id: neutron_key, revision: Revision {cycle:context.revision.cycle-1} };
        let neutron_content= context.get_content(&content_key)?;
        let tron_id = Id::new( neutron_content.data.get::<i64>( &[&"tron_names",name,&"seq_id"] )?.unwrap(),
                               neutron_content.data.get::<i64>( &[&"tron_names",name,&"id"] )?.unwrap() );

        let tron_key = TronKey{
            nucleus_id: nucleus_id.clone(),
            tron_id: tron_id
        };

        Ok(tron_key)
    }
}

pub struct TronShell
{
    pub tron: Box<dyn Tron>
}

impl TronShell
{
    pub fn new( tron: Box<dyn Tron>)->Self{
        TronShell{
            tron: tron
        }
    }

    fn from( &self, context: Context )->mechtron_common::message::From
    {
        mechtron_common::message::From {
            tron: context.id.clone(),
            cycle: context.revision.cycle.clone(),
            timestamp: context.timestamp.clone()
        }
    }




    fn builders_to_messages( &self, context: &Context, builders: Option<Vec<MessageBuilder>> )->Result<Option<Vec<Message>>,Box<dyn Error>>
    {
        if builders.is_none()
        {
            return Ok(Option::None);
        }

        let mut builders = builders.unwrap();

        let messages = builders.iter().map( |builder:&mut MessageBuilder| {
            builder.from = Option::Some( from(context) );

            if builder.to_nucleus_lookup_name.is_some()
            {
                builder.to_nucleus_id = Option::Some(self.lookup_nucleus(context,builder.to_nucleus_lookup_name.unwrap().as_str())?);
            }

            if builder.to_tron_lookup_name.is_some()
            {
                builder.to_tron_id = Option::Some(self.lookup_tron(context, &builder.to_nucleus_id.unwrap(), builder.to_tron_lookup_name.unwrap().as_str() )?.tron_id);
            }


            builder.build(&mut SYS.net.id_seq)
        }).collect();

        return Ok(Option::Some(messages))
    }

    pub fn create(&self, context: &Context,
                     content: &mut Content,
                     create: &Message) -> Result<Option<Vec<Message>>, Box<dyn Error>> {
        let mut builders= self.tron.create( context, content, create )?;
        match builders {
            None => Ok(Option::None),
            Some(builders) =>
                Option::Some(builders.iter().map( |builder:&mut MessageBuilder| {
                    builder.from = Option::Some( from(context) );
                }).collect())
        }
    }

    pub fn update(&self, context: &Context, content: &mut Content, inbound_messages: Vec<&Message>) -> Result<Option<Vec<Message>>, Box<dyn Error>> {
        unimplemented!()
    }

}


pub struct SimTron
{

}

impl Tron for SimTron
{
    fn init(context: Context) -> Result<Box<Self>, Box<dyn Error>> where Self: Sized {
        unimplemented!()
    }

    fn create(&self, context: &Context, content: &mut Content, create: &Message) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>> {
        unimplemented!()
    }

    fn update(&self, phase: &str) -> Result<fn(&Context, &Content) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>>, Box<dyn Error>> {
        unimplemented!()
    }

    fn port(&self, port: &str) -> Result<fn(&Context, &Content, &Message) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>>, Box<dyn Error>> {
        unimplemented!()
    }

    fn update_phases(&self) -> UpdatePhases {
        unimplemented!()
    }
}

pub struct Neutron
{

}

pub struct NeutronContentInterface
{

}

impl NeutronContentInterface
{
    fn add_tron_np_error( &self, content: &mut Content, tron: &TronKey, kind: u8 )->Result<(),NP_Error>
    {
        let index = content.data.get_length(&[&"trons"] )?.unwrap();
        content.data.set(&[&"trons",&index.to_string(), &"seq_id"], tron.tron_id.seq_id )?;
        content.data.set(&[&"trons",&index.to_string(), &"id"], tron.tron_id.id )?;
        content.data.set(&[&"trons",&index.to_string(), &"kind"], kind )?;

        Ok(())
    }

    fn set_tron_name_np_error( &self, content: &mut Content, name: &str, tron: &TronKey )->Result<(),NP_Error>
    {
        content.set(&[&"tron_names",name,&"seq_id"], context.id.tron_id.seq_id )?;
        content.set(&[&"tron_names",name,&"id"], context.id.tron_id.id)?;

        Ok(())
    }

    pub fn add_tron( &self, content: &mut Content, tron: &TronKey, kind: u8 )->Result<(),Box<dyn Error>>
    {
        match self.add_tron_np_error(content,tron,kind)
        {
            Ok(_)=>Ok(()),
            Err(_)=>Err("encountered error when adding tron key to neutron context".into())
        }
    }

    pub fn set_tron_name( &self, content: &mut Content, name: &str, tron: &TronKey )->Result<(),Box<dyn Error>>
    {
        match self.set_tron_name_np_error(content,name, tron)
        {
            Ok(_)=>Ok(()),
            Err(_)=>Err("encountered error when adding tron key to neutron context".into())
        }
    }
}

impl Neutron {

    pub fn create_tron(&self, context: &Context, create: &Message) -> Result<Option<Vec<Message>>, Box<dyn Error>>
    {
        let tron_key= TronKey::new(nucleus_id: context.id.nucleus_id, SYS.net.id_seq.next() );
        let interface = NeutronContentInterface{};
        interface.add_tron(content,&tron_key, 0)?;

        let create_meta = &message.payloads[0].buffer;
        if create_meta.get(&[&"lookup_name"] ).unwrap().is_some()
        {
        let name = create_meta.get::<String>(&[&"lookup_name"] ).unwrap().unwrap();
        interface.set_tron_name(contet, name.as_str(), &tron_key );
        }

        let tron_config = create_meta.get::<String>(&[&"artifact"]).unwrap().unwrap();
        let tron_config = Artifact::from(&tron_config)?;
        let tron_config = context.configs().tron_config_keeper.get(&tron_config)?;

        let tron_content_artifact = match tron_config.messages
        {
        None => context.configs().core_artifact("schema/empty"),
        Some(_) => match tron_config.messages.unwrap().create{
        None => context.configs().core_artifact("schema/empty"),
        Some(_) =>  tron_config.messages.unwrap().create.unwrap().artifact
        }
        }?;

        let mut content = Content::new(context.configs(),tron_content_artifact.clone());

        content.meta.set( &[&"artifact"], tron_config.source.to() );
        content.meta.set( &[&"creation_timestamp"], context.timestamp );
        content.meta.set( &[&"creation_cycle"], context.revision.cycle );

        let tron = init_tron(&tron_config,context)?;
        let tron = TronShell::new(tron);
        let tron_context = Context{
        sim_id: context.sim_id.clone(),
        id: tron_key.clone(),
        revision: context.revision.clone(),
        tron_config:tron_config.clone(),
        timestamp: context.timestamp,
        phase: context.phase
        };

        tron.create(&tron_context, &mut content, &message )?;

        Ok(Option::None)
    }
}

impl Tron for Neutron {
    fn init(context: Context) -> Result<Box<Self>, Box<dyn Error>> where Self: Sized {
        Ok(Box::new(Neutron {}))
    }

    fn create(&self, context: &Context, content: &mut Content, create: &Message) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>> {

        let interface = NeutronContentInterface{};
        interface.add_tron(content,&context.id, 0)?;
        interface.set_tron_name(content, "neutron", &context.id )?;
        Ok(Option::None)
    }

    fn update(&self, phase: &str) -> Result<fn(&Context, &Content) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>>, Box<dyn Error>> {
        Option::None;
    }

    fn port(&self, port: &str) -> Result<fn(&Context, &mut Content, &Message) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>>, Box<dyn Error>> {

        /*
        match port {
            "create"=>Ok(| context, content, message | ),
            _=>Err(format!("could not find port {}",port).into())
        }
         */

    }

    fn update_phases(&self) -> UpdatePhases {
        UpdatePhases::None
    }
}





pub struct CreatePayloadsBuilder
{
    pub constructor_artifact : Artifact,
    pub meta: NP_Buffer<NP_Memory_Owned>,
    pub constructor: NP_Buffer<NP_Memory_Owned>
}

impl CreatePayloadsBuilder
{

    pub fn new( configs: &Configs, tron_config: &TronConfig )->Result<Self,Box<dyn Error>>
    {
        let meta_factory = configs.core_buffer_factory("schema/create/meta" )?;
        let meta = meta_factory.new_buffer(Option::None);
        let (constructor_artifact,constructor)= CreatePayloadsBuilder::constructor(configs,tron_config)?;
        Ok(CreatePayloadsBuilder {
            meta: meta,
            constructor_artifact: constructor_artifact,
            constructor: constructor,
        })
    }

    fn constructor( configs: &Configs, tron_config: &TronConfig ) -> Result<(Artifact,NP_Buffer<NP_Memory_Owned>),Box<dyn Error>>
    {
        if tron_config.messages.is_some()  && tron_config.messages.unwrap().create.is_some(){
            let constructor_artifact= tron_config.messages.unwrap().create.unwrap().artifact.clone();
            let factory = configs.buffer_factory_keeper.get(&constructon_artifact)?;
            let constructor = factory.new_buffer(Option::None);
            Ok((constructor_artifact, constructor))
        }
        else {
            let constructor_artifact = configs.core_artifact("schema/empty")?.clone();
            let factory = configs.core_buffer_factory("schema/empty")?;
            let constructor = factory.new_buffer(Option::None);
            Ok((constructor_artifact, constructor))
        }

    }

    pub fn payloads(configs: &Configs, builder: CreatePayloadsBuilder )->Vec<Payload>
    {
        let meta_artifact= configs.core_artifact("schema/create/meta" )?;
        vec![
            Payload{
                artifact: meta_artifact,
                buffer: Arc::new(builder.meta)
            },
            Payload{
                artifact: builder.constructor_artifact,
                buffer:  Arc::new(builder.constructor )
            }
        ]
    }

}

struct StdOut
{

}

impl Tron for StdOut
{
    fn init(context: Context) -> Result<Box<Self>, Box<dyn Error>> where Self: Sized {
        Ok(Box::new(StdOut{}))
    }

    fn create(&self, context: &Context, content: &mut Content, create: &Message) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>> {
        Ok(Option::None)
    }

    fn update(&self, phase: &str) -> Result<fn(&Context, &Content) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>>, Box<dyn Error>> {
        Ok(Option::None)
    }

    fn port(&self, port: &str) -> Result<fn(&Context, &Content, &Message) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>>, Box<dyn Error>> {
        match port {
            "println"=>Ok(| context, content, message | {
                let line = get::<String, NP_Memory_Owned>(&message.payloads[0].buffer, &[])?;
                println!(line);
                Ok(Option::None)
            } ),
            _=>Err(format!("could not find port {}",port).into())
        }
    }

    fn update_phases(&self) -> UpdatePhases {
        UpdatePhases::None
    }
}

pub fn init_tron(config: &TronConfig, context: &Context ) ->Result<Box<dyn Tron>, Box<dyn Error>>
{
    let rtn = match config.kind.as_str(){
        "sim"=>SimTron.init(context),
        "neutron"=>NeuTron.init(context),
        _ =>return Err(format!("we don't have a tron of kind {}",kind).into())
    };

    Ok(rtn)
}
use no_proto::buffer::NP_Buffer;
use mechtron_common::message::{Message, MessageBuilder};
use std::error::Error;
use mechtron_common::artifact::Artifact;
use no_proto::memory::NP_Memory_Owned;
use mechtron_common::id::{TronKey, Revision, NucleusKey, Id, ContentKey};
use mechtron_common::configs::TronConfig;
use std::sync::Arc;
use crate::nucleus::NeuTron;
use crate::app::SYS;
use mechtron_common::content::Content;
use crate::content::ContentRetrieval;

pub trait Tron
{
    fn init (context: Context)->Result<Box<Self>,Box<dyn Error>>  where Self: Sized;

    fn create (&self,
               context: &Context,
               content: &mut Content,
               create: &Message) -> Result<(Option<Vec<MessageBuilder>>), Box<dyn Error>>;

    fn update( &self,
               context: &Context,
               content: &mut Content,
               inbound_messages: &Vec<&Message>) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>>;

    fn port( &self, port: &str ) -> Result<Box<dyn MessagePort>,Box<dyn Error>>;

    fn phase_updates(&self)->PhaseUpdates;
}

pub enum PhaseUpdates
{
    All,
    Some(Vec<String>),
    None
}

pub trait MessagePort
{
    fn receive( &mut self, context: &Context, content: &Content, message: &Message ) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>>;
}

pub struct Context
{
    pub id : TronKey,
    pub sim_id: Id,
    pub revision: Revision,
    pub config: Arc<TronConfig>,
    pub timestamp: i64
}

impl Context {

    pub fn get_content( &self, key:&ContentKey)->Result<Content,Box<dyn Error>>
    {
        let source = SYS.local.sources.get(&self.sim_id)?;
        if key.revision.cycle >= self.revision.cycle
        {
            return Err(format!("tron {:?} attempted to read the content of tron {:?} in a present or future cycle, which is not allowed",self.id,key.content_id).into());
        }
        let content = source.content.get_read_only(key)?;
        Ok(content)
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

    fn lookup_nucleus( &self, context: &Context, name: &str )->Result<Id,Box<dyn Error>>
    {
    }

    fn lookup_tron( &self, context: &Context, nucleus_id: &Id, name: &str )->Result<TronKey,Box<dyn Error>>
    {
    }


    fn builders_to_messages( &self, context: &Context, builders: Option<Vec<MessageBuilder>> )->Result<Option<Vec<Message>>,Box<dyn Error>>
    {
        if builers.is_none()
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

    fn create(&self, context: &Context,
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

    fn update(&self, context: &Context, content: &mut Content, inbound_messages: Vec<&Message>) -> Result<Option<Vec<Message>>, Box<dyn Error>> {
        unimplemented!()
    }

}


pub struct SimTron
{

}

impl Tron for SimTron
{
    fn init(context: Context) -> Result<Box<Self>, Box<dyn Error>> where Self: Sized {
        Ok(Box::from(SimTron {}))
    }

    fn create(&self, context: &Context, content: &mut NP_Buffer<NP_Memory_Owned>, create_message: &Message) -> Result<Option<Vec<Message>>, Box<dyn Error>> {
        Ok(Option::None)
    }

    fn update(&self, context: &Context, content: &mut NP_Buffer<NP_Memory_Owned>, messages: Vec<&Message>) -> Result<Option<Vec<Message>>, Box<dyn Error>> {
        Ok(Option::None)
    }


}

pub struct Neutron
{

}


/*
content.list_push(&[&"tron_ids"], context.id() );
content.list_push(&[&"tron_names",&"neutron"], context.id() );
Ok(Option::None)

 */

impl Tron for Neutron {
    fn init(context: Context) -> Result<Box<Self>, Box<dyn Error>> where Self: Sized {
        Ok(Box::new(Neutron {}))
    }

    fn create(&self, context: &Context, content: &mut Content, create: &Message) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>> {
        content.list_push(&[&"tron_ids"], context.id() );
        content.list_push(&[&"tron_names",&"neutron"], context.id() );
        Ok(Option::None)
    }

    fn update(&self, context: &Context, content: &mut Content, inbound_messages: &Vec<&Message>) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>> {
        Ok(Option::None)
    }

    fn port(&self, port: &str) -> Result<Box<dyn MessagePort>, Box<dyn Error>> {
        Err("no ports".into())
    }
}

pub struct StdOut
{

}

impl Tron for StdOut
{
    fn init(context: Context) -> Result<Box<Self>, Box<dyn Error>> where Self: Sized {
        unimplemented!()
    }

    fn create(&self, context: &Context, content: &mut Content, create: &Message) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>> {
        unimplemented!()
    }

    fn update(&self, context: &Context, content: &mut Content, inbound_messages: &Vec<&Message>) -> Result<Option<Vec<MessageBuilder>>, Box<dyn Error>> {
        unimplemented!()
    }

    fn port(&self, port: &str) -> Result<Box<_>, Box<dyn Error>> {
        unimplemented!()
    }
}

pub fn init_tron_of_kind(kind: &str, context: &Context ) ->Result<Box<dyn Tron>, Box<dyn Error>>
{
    let rtn = match kind{
        "sim"=>SimTron.init(context),
        "neutron"=>NeuTron.init(context),
        "stdout"=>StdOut.init(context),
        _ =>return Err(format!("we don't have a tron of kind {}",kind).into())
    };

    Ok(rtn)
}

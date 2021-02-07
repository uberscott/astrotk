use no_proto::buffer::NP_Buffer;
use mechtron_common::message::Message;
use crate::app::App;
use std::error::Error;
use mechtron_common::artifact::Artifact;

pub trait Tron
{
    fn id(&self)->i64;

    fn init (context: InitContext)->Result<Box<Self>,Box<dyn Error>>  where Self: Sized;

    fn create<'a,'buffer> (&self, context: &dyn CreateContext, create_message: &Message<'a>) -> Result<(NP_Buffer<'buffer>, Vec<Message<'a>>), Box<dyn Error>>;

    fn update<'a,'buffer>( &self,
                           context: &dyn UpdateContext,
                           content: &NP_Buffer<'buffer>,
                           messages: Vec<&Message<'a>>) -> Result<(NP_Buffer<'buffer>, Vec<Message<'a>>), Box<dyn Error>>;
}

pub trait UpdateContext
{
    fn id(&self)->i64;
    fn cycle(&self) -> i64;
    fn simulation(&self) -> i64;
    fn nucleus(&self) -> i64;
    fn vertex(&self) -> &'static App<'static>;
}


pub trait CreateContext: UpdateContext {
}

pub struct InitContext<'a>
{
    pub id: i64,
    pub app: &'a App<'a>,
    config_artifact: Artifact
}

impl <'a> InitContext<'a>
{
    pub fn new(id:i64, app: &'a App, config_artifact: Artifact )->Self
    {
        InitContext{
            id: id,
            app: app,
            config_artifact:config_artifact
        }
    }

    pub fn config_artifact(&self) -> &Artifact
    {
        &self.config_artifact
    }
}

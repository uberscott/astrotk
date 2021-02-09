use no_proto::buffer::NP_Buffer;
use mechtron_common::message::Message;
use std::error::Error;
use mechtron_common::artifact::Artifact;

pub trait Tron
{
    fn id(&self)->i64;

    fn init (context: InitContext)->Result<Box<Self>,Box<dyn Error>>  where Self: Sized;

    fn create (&self, context: &dyn CreateContext, create_message: &Message) -> Result<(NP_Buffer, Vec<Message>), Box<dyn Error>>;

    fn update( &self,
                           context: &dyn UpdateContext,
                           content: &NP_Buffer,
                           messages: Vec<&Message>) -> Result<(NP_Buffer, Vec<Message>), Box<dyn Error>>;
}

pub trait UpdateContext
{
    fn id(&self)->i64;
    fn cycle(&self) -> i64;
    fn simulation(&self) -> i64;
    fn nucleus(&self) -> i64;
}


pub trait CreateContext: UpdateContext {
}

pub struct InitContext<'a>
{
    pub id: i64,
    config_artifact: Artifact
}

impl <'a> InitContext<'a>
{
    pub fn new(id:i64, config_artifact: Artifact )->Self
    {
        InitContext{
            id: id,
            config_artifact:config_artifact
        }
    }

    pub fn config_artifact(&self) -> &Artifact
    {
        &self.config_artifact
    }
}

use no_proto::buffer::NP_Buffer;
use no_proto::buffer_ro::NP_Buffer_RO;

use astrotk_config::message::Message;

struct Actor
{
    id: i64,
    engine: Box<dyn ActorEngine>
}

impl Actor
{

    fn create( &mut self, create_message: &Message) -> Result<(NP_Buffer,Vec<Message>),Box<std::error::Error>>
    {
        return Ok(self.engine.create(create_message)?);
    }

    fn update( &mut self, content: &NP_Buffer, messages: Vec<&Message> )-> Result<(NP_Buffer,Vec<Message>),Box<std::error::Error>>
    {
        return Ok(self.engine.update(content, messages )?);
    }

}


pub trait ActorEngine
{
    fn create( &mut self, create_message: &Message ) -> Result<(NP_Buffer,Vec<Message>),Box<std::error::Error>>;

    fn update( &mut self, content: &NP_Buffer, messages: Vec<&Message> )-> Result<(NP_Buffer,Vec<Message>),Box<std::error::Error>>;
}


use no_proto::buffer::NP_Buffer;
use no_proto::buffer_ro::NP_Buffer_RO;

struct Actor
{
    id: i64,
    engine: Box<dyn ActorEngine>
}

impl Actor
{

    fn create( &mut self, create_message: &NP_Buffer ) -> Result<NP_Buffer,Box<std::error::Error>>
    {
        return Ok(self.engine.create(create_message)?);
    }

    fn update( &mut self, content: &NP_Buffer, messages: Box<[&NP_Buffer]> )-> Result<(NP_Buffer,Box<[NP_Buffer]>),Box<std::error::Error>>
    {
        return Ok(self.engine.update(content, messages )?);
    }

}


pub trait ActorEngine
{
    fn create( &mut self, create_message: &NP_Buffer ) -> Result<NP_Buffer,Box<std::error::Error>>;

    fn update( &mut self, content: &NP_Buffer, messages: Box<[&NP_Buffer]> )-> Result<(NP_Buffer,Box<[NP_Buffer]>),Box<std::error::Error>>;
}


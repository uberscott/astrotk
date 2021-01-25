use wasm_bindgen::prelude::*;
use wasm_bindgen::__rt::std::collections::HashMap;
use bytes::{BytesMut, BufMut, Buf, Bytes};
use std::sync::Mutex;
use std::sync::atomic::{{AtomicUsize,Ordering}};
use astrotk_config::actor_config::{ActorConfigYaml, ActorConfig};
use wasm_bindgen::__rt::std::error::Error;
use astrotk_config::artifact_config::ArtifactFile;
use no_proto::NP_Factory;
use no_proto::buffer::NP_Buffer;
use no_proto::buffer_ro::NP_Buffer_RO;
use astrotk_config::buffers::BufferFactories;

#[macro_use]
extern crate lazy_static;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

extern "C"
{
    fn host_alloc_buffer( len: i32 ) -> i32;
    fn host_append_to_buffer( id: i32, value: i32 );
    fn host_dealloc_buffer( id: i32 );
    fn host_log( buffer_id: i32 );
    fn host_content_update( buffer_id: i32 );
    fn host_messages( buffer_id: i32 );
    fn actor_create( ctx: &ActorContext, create_message: &NP_Buffer_RO, content: &mut NP_Buffer ) -> Result<(),Box<std::error::Error>>;
}



lazy_static! {
    static ref buffers: Mutex<HashMap<i32, BytesMut>> = {
        let m = HashMap::new();
        Mutex::new(m)
    };

    static ref buffer_index : AtomicUsize = { AtomicUsize::new(0) };

    static ref context : Mutex<ActorContext> = {
        let a = ActorContext::new();
        Mutex::new(a)
    };

    static ref message_buffer_factories: Mutex<Box<BufferFactoriesCache<'static>>> = {
        let bfc = BufferFactoriesCache::new();
        Mutex::new(bfc)
    };
}

struct BufferFactoriesCache<'a>
{
    cache: HashMap<ArtifactFile,NP_Factory<'a>>
}

impl <'a> BufferFactoriesCache<'a>
{
    fn new()->Box<Self>
    {
        return Box::new(BufferFactoriesCache{cache:HashMap::new()});
    }

    fn add_factory(&mut self, artifact_file_buffer_id: i32, factory_schema_buffer_id: i32 ) ->Result<(),Box<std::error::Error>>
    {
        let artifact_file = ArtifactFile::from( consume_string(artifact_file_buffer_id)?.as_str() )?;
        let result = NP_Factory::new(consume_string(factory_schema_buffer_id)?);
        match result{
            Ok(factory)=>{
                self.cache.insert( artifact_file.clone(), factory );
                Ok(())
            },
            Err(_)=> Err("could not parse factory config".into())
        }
    }
}

impl <'a> BufferFactories for BufferFactoriesCache<'a>
{
    fn create_buffer(&self, artifact_file: &ArtifactFile) -> Result<NP_Buffer, Box<dyn Error>> {
        return match self.get_buffer_factory(artifact_file)
        {
            Some(factory)=>Ok(factory.empty_buffer(Option::None)),
            None=>Err(format!("could not find factory: {}", artifact_file.to()).into())
        }
    }

    fn create_buffer_from(&self, artifact_file: &ArtifactFile, array: Vec<u8>) -> Result<NP_Buffer, Box<dyn Error>> {
        return match self.get_buffer_factory(artifact_file)
        {
            Some(factory)=>Ok(factory.open_buffer(array)),
            None=>Err(format!("could not find factory: {}", artifact_file.to()).into())
        }
    }

    fn get_buffer_factory(&self, artifact_file: &ArtifactFile) -> Option<&NP_Factory> {
        return self.cache.get(artifact_file);
    }
}

pub fn host_write_buffer( buffer: &NP_Buffer ) -> i32
{
    unsafe{
        let buffer_id = host_alloc_buffer(buffer.calc_bytes().unwrap().current_buffer as _ );
        for b in buffer.read_bytes()
        {
            host_append_to_buffer(buffer_id,*b as _);
        }
        return buffer_id;
    }
}

pub fn log( str: &str )
{
    unsafe
        {
            let buffer_id = host_alloc_buffer(str.len() as _);
            for b in str.as_bytes()
            {
                host_append_to_buffer(buffer_id,*b as i32);
            }
            host_log(buffer_id);
            host_dealloc_buffer(buffer_id);
        }
}

#[wasm_bindgen]
pub fn alloc_buffer( len: i32 ) -> i32 {
    let mut buffer = BytesMut::with_capacity(len as usize );

    let mut buffer_id = buffer_index.fetch_add(1,Ordering::Relaxed) as i32;
    buffers.lock().unwrap().insert(buffer_id,buffer);

    return buffer_id;
}

#[wasm_bindgen]
pub fn append_to_buffer( id: i32, value: i32 )
{
    let b= buffers.lock();
    let mut unwrapped = b.unwrap();
    let option = unwrapped.get_mut(&id);
    let buffer = option.unwrap();
    buffer.put_u8(value as u8);
}

#[wasm_bindgen]
pub fn dealloc_buffer( id: i32 )
{
    buffers.lock().unwrap().remove(&id);
}


#[wasm_bindgen]
pub fn bind_message_artifact( artifact_file_buffer_id: i32, artifact_file_content_buffer_id: i32 )
{
    let mut factories = message_buffer_factories.lock().unwrap();
    factories.add_factory(artifact_file_buffer_id,artifact_file_content_buffer_id);
}

fn consume_buffer( buffer_id: i32 ) -> Result<Box<Bytes>,Box<std::error::Error>>
{
    let option = buffers.lock().unwrap().remove(&buffer_id);
    match option {
        None => Err("could not find buffer".into() ),
        Some(buffer) => {
            return Ok(Box::new(buffer.freeze()));
        }
    }
}

fn consume_string(  buffer_id: i32 ) -> Result<String,Box<Error>>
{
    let buffer = consume_buffer(buffer_id)?;
    return Ok(String::from_utf8(buffer.to_vec() )?);
}

#[wasm_bindgen]
pub fn actor_init(actor_config_buffer_id:i32, actor_config_artifact_buffer_id: i32) -> i32
{
    log( &"hello from WebAssembly actor_init" );
    let result = init_context(actor_config_buffer_id,actor_config_artifact_buffer_id);

    if result.is_err()
    {
       println!("returning ERR");
       return 1;
    }

    println!( "actor name: {}", context.lock().unwrap().actor_config.as_ref().unwrap().name );

    return 0;
}

fn get_artifact_file( artifact_buffer_id:i32)->Result<ArtifactFile,Box<std::error::Error>>
{
    let artifact_file_str= consume_string(artifact_buffer_id)?;
    return Ok(ArtifactFile::from(&artifact_file_str )?);
}

fn init_context( actor_config_buffer_id:i32, actor_config_artifact_buffer_id:i32) -> Result<(),Box<std::error::Error>>
{

    let actor_config_str = consume_string(actor_config_buffer_id)?;
    let actor_config_yaml = ActorConfigYaml::from_yaml(&actor_config_str)?;
    let actor_config = actor_config_yaml.to_actor_config( &get_artifact_file(actor_config_artifact_buffer_id)?)?;
    log( &format!("Actor Name is: {}",actor_config.name) );
    context.lock().unwrap().actor_config = Option::Some(actor_config);
    return Ok(());
}


#[wasm_bindgen]
pub fn astrotk_actor_create( create_message_buffer_id: i32 ) -> i32
{

    log("astrotk_actor_create");
    let factories = message_buffer_factories.lock().unwrap();
    log("got factories");
    let ctx = context.lock().unwrap();

    let actor_config = &ctx.actor_config.as_ref().unwrap();
    log("got actor_config");

    let create_factory = factories.get_buffer_factory( &actor_config.create_message ).unwrap();
    let content_factory = factories.get_buffer_factory( &actor_config.content ).unwrap();

    log("got create factor and content_factory");
    let raw_create_message = consume_buffer(create_message_buffer_id).unwrap();
    log(format!("raw create message size: {}",raw_create_message.len()).as_str() );
    log("raw_create_messagE");

    let create_message = create_factory.open_buffer_ro(raw_create_message.as_ref());
    let mut content = content_factory.empty_buffer(Option::None );
    log("content");

    unsafe {
        match actor_create(&ctx, &create_message, &mut content ) {
            Ok(_) => {
                log( "wrign host buffeR" );
                let content_buffer_id = host_write_buffer(&content);
                log( "astrotk_actor_create: host_content_update()");
                host_content_update(content_buffer_id);
                log( "astrotk_actor_create: host_content_update() DONE");
                return content_buffer_id;
            },
            Err(e) => {
                log("error during create...");
                return 1
            }
        }
    }
}


pub struct ActorContext
{
    actor_config: Option<ActorConfig>
}

impl ActorContext
{
    fn new( ) -> Self
    {
        return ActorContext{actor_config:Option::None};
    }
}

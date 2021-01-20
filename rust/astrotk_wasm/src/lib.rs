use wasm_bindgen::prelude::*;
use wasm_bindgen::__rt::std::collections::HashMap;
use bytes::{BytesMut, BufMut, Buf, Bytes};
use std::sync::Mutex;
use std::sync::atomic::{{AtomicUsize,Ordering}};
use astrotk_config::actor_config::{ActorConfigYaml, ActorConfig};
use wasm_bindgen::__rt::std::error::Error;
use astrotk_config::artifact_config::ArtifactFile;

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
}

extern "C"
{
    fn actor_create( ctx: &ActorContext ) -> Result<(),Box<std::error::Error>>;
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

pub fn consume_buffer( buffer_id: i32 ) -> Result<Box<Bytes>,Box<std::error::Error>>
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
pub fn actor_init(actor_config_buffer_id:i32, actor_config_artifact_buffer_id: i32, actor_context_buffer_id: i32) -> i32
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
pub fn astrotk_actor_create( content_buffer_id:i32, create_message_buffer_id: i32 ) -> i32
{

    unsafe {
        match actor_create(&context.lock().unwrap() ) {
            Ok(_) => return 0,
            Err(_) => return 1
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

#[macro_use]
extern crate lazy_static;

use std::sync::atomic::{{AtomicUsize, Ordering}};
use std::sync::{Mutex, Arc};

use bytes::{Buf, BufMut, Bytes, BytesMut};
use no_proto::buffer::NP_Buffer;
use no_proto::buffer_ro::NP_Buffer_RO;
use no_proto::NP_Factory;
use wasm_bindgen::__rt::std::collections::HashMap;
use wasm_bindgen::__rt::std::error::Error;
use wasm_bindgen::prelude::*;

use mechtron_config::artifact_config::Artifact;
use mechtron_config::buffers::BufferFactories;
use mechtron_config::mechtron_config::{ActorConfigYaml, MechtronConfig};
use std::ops::Deref;
use std::borrow::{Borrow, BorrowMut};
use std::cell::RefCell;


// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator
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
    fn host_messages(buffer_id: i32);
    fn host_write_artifact_as_string(artifact_name_buffer_id: i32) -> i32;
    fn mechtron_create(ctx: &MechtronContext, create_message: &NP_Buffer_RO, content: &mut NP_Buffer) -> Result<(), Box<std::error::Error>>;
}



lazy_static! {
    static ref buffers: Mutex<HashMap<i32, BytesMut>> = {
        let m = HashMap::new();
        Mutex::new(m)
    };

    static ref buffer_index : AtomicUsize = { AtomicUsize::new(0) };

    static ref context : Mutex<MechtronContext> = {
        let a = MechtronContext::new();
        Mutex::new(a)
    };

    static ref message_buffer_factories: Mutex<Box<BufferFactoriesCache<'static>>> = {
        let bfc = BufferFactoriesCache::new();
        Mutex::new(bfc)
    };
}

struct BufferFactoriesCache<'a>
{
    cache: HashMap<Artifact,NP_Factory<'a>>
}

impl <'a> BufferFactoriesCache<'a>
{
    fn new()->Box<Self>
    {
        return Box::new(BufferFactoriesCache{cache:HashMap::new()});
    }

    fn add_factory(&mut self, artifact_file_buffer_id: i32, factory_schema_buffer_id: i32 ) ->Result<(),Box<dyn Error>>
    {
        let artifact_file = Artifact::from( consume_string(artifact_file_buffer_id)?.as_str() )?;
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
    fn create_buffer(&self, artifact_file: &Artifact) -> Result<NP_Buffer, Box<dyn Error>> {
        return match self.get_buffer_factory(artifact_file)
        {
            Some(factory)=>Ok(factory.empty_buffer(Option::None)),
            None=>Err(format!("could not find factory: {}", artifact_file.to()).into())
        }
    }

    fn create_buffer_from(&self, artifact_file: &Artifact, array: Vec<u8>) -> Result<NP_Buffer, Box<dyn Error>> {
        return match self.get_buffer_factory(artifact_file)
        {
            Some(factory)=>Ok(factory.open_buffer(array)),
            None=>Err(format!("could not find factory: {}", artifact_file.to()).into())
        }
    }

    fn get_buffer_factory(&self, artifact_file: &Artifact) -> Option<&NP_Factory> {
        return self.cache.get(artifact_file);
    }
}

pub fn host_write_buffer( buffer: &NP_Buffer ) -> i32
{
    unsafe {
        let buffer_id = host_alloc_buffer(buffer.calc_bytes().unwrap().current_buffer as _);
        for b in buffer.read_bytes()
        {
            host_append_to_buffer(buffer_id, *b as _);
        }
        return buffer_id;
    }
}

pub fn host_write_string(str: &str) -> i32
{
    unsafe {
        let buffer_id = host_alloc_buffer(str.len() as _);
        for b in str.bytes()
        {
            host_append_to_buffer(buffer_id, b as _);
        }
        return buffer_id;
    }
}

pub fn host_get_artifact_as_str(artifact: &str) -> Result<String, Box<dyn Error>>
{
    let artifact_name_buffer_id = host_write_string(artifact);
    unsafe {
        let artifact_as_string_buffer_id =
            host_write_artifact_as_string(artifact_name_buffer_id);
        let rtn = consume_string(artifact_as_string_buffer_id)?;
        Ok(rtn)
    }
}

pub fn log(str: &str)
{
    unsafe
        {
            let buffer_id = host_alloc_buffer(str.len() as _);
            for b in str.as_bytes()
            {
                host_append_to_buffer(buffer_id, *b as i32);
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

fn consume_string(buffer_id: i32) -> Result<String, Box<dyn Error>>
{
    let buffer = consume_buffer(buffer_id)?;
    return Ok(String::from_utf8(buffer.to_vec())?);
}

#[wasm_bindgen]
pub fn mechtron_init(mechtron_config_buffer_id:i32, mechtron_config_artifact_buffer_id: i32) -> i32
{
    log( &"hello from WebAssembly actor_init" );
    let result = init_context(mechtron_config_buffer_id, mechtron_config_artifact_buffer_id);

    if result.is_err()
    {
       println!("returning ERR");
       return 1;
    }

    println!( "actor name: {}", context.lock().unwrap().actor_config.as_ref().unwrap().name );

    return 0;
}

fn get_artifact_file( artifact_buffer_id:i32)->Result<Artifact,Box<std::error::Error>>
{
    let artifact_file_str= consume_string(artifact_buffer_id)?;
    return Ok(Artifact::from(&artifact_file_str )?);
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
pub fn mechtron_actor_create( create_message_buffer_id: i32 ) -> i32
{

    log("mechtron_actor_create");
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
        match mechtron_create(&ctx, &create_message, &mut content ) {
            Ok(_) => {
                log( "wrign host buffeR" );
                let content_buffer_id = host_write_buffer(&content);
                log( "mechtron_actor_create: host_content_update()");
                host_content_update(content_buffer_id);
                log( "mechtron_actor_create: host_content_update() DONE");
                return content_buffer_id;
            },
            Err(e) => {
                log("error during create...");
                return 1
            }
        }
    }
}


pub struct MechtronContext
{
    actor_config: Option<MechtronConfig>
}

impl MechtronContext
{
    fn new() -> Self
    {
        return MechtronContext { actor_config: Option::None };
    }
}


pub struct MechtronArtifactCache
{
    cache: Mutex<RefCell<HashMap<Artifact, Arc<String>>>>,
}


impl<'a> MechtronArtifactCache
{
    pub fn new(repo_path: String) -> Self
    {
        return MechtronArtifactCache {
            cache: Mutex::new(RefCell::new(HashMap::new())),
        };
    }
}

impl<'a> ArtifactRepository for MechtronArtifactCache
{
    fn fetch(&self, bundle: &ArtifactBundle) -> Result<(), Box<dyn Error + '_>>
    {
        let lock = self.fetches.read()?;
        if lock.contains(bundle)
        {
            return Ok(())
        }

        let mut lock = self.fetches.write()?;
        lock.insert(bundle.clone());

        // at this time we don't do anything
        // later we will pull a zip file from a public repository and
        // extract the files to 'repo_path'
        return Ok(());
    }
}

impl<'a> ArtifactCache for MechtronArtifactCache
{
    fn cache(&self, artifact: &Artifact) -> Result<(), Box<dyn Error + '_>>
    {
        let cache = self.cache.lock();
        let cell = &*cache.unwrap();
        let cache = cell.borrow();
        if cache.contains_key(artifact)
        {
            return Ok(());
        }
        let mut cache = cell.borrow_mut();
        let string = String::from_utf8(self.load(artifact)?)?;
        cache.insert(artifact.clone(), Arc::new(string));
        return Ok(());
    }

    fn load(&self, artifact: &Artifact) -> Result<Vec<u8>, Box<dyn Error + '_>>
    {
        host
        return Ok(data);
    }

    fn get(&self, artifact: &Artifact) -> Result<Arc<String>, Box<dyn Error + '_>>
    {
        let cache = self.cache.lock();
        let cache = cache.unwrap();
        let cache = cache.deref();
        let cache = cache.borrow();
        let option = cache.get(artifact);

        match option {
            None => Err(format!("artifact is not cached: {}", artifact.to()).into()),
            Some(rtn) => Ok(rtn.clone())
        }
    }
}
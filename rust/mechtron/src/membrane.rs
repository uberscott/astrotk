use wasm_bindgen::prelude::*;
use wasm_bindgen::__rt::std::alloc::{Layout,alloc,dealloc};
use wasm_bindgen::__rt::std::mem;
use mechtron_common::core::*;
use wasm_bindgen::__rt::core::slice;
use std::borrow::BorrowMut;
use std::sync::{RwLock, Mutex, MutexGuard,Arc};
use wasm_bindgen::__rt::std::collections::HashMap;
use wasm_bindgen::__rt::std::sync::atomic::{Ordering, AtomicPtr};
use wasm_bindgen::__rt::std::sync::atomic::AtomicI32;
use crate::CONFIGS;
use crate::mechtron;
use std::ops::{Deref, DerefMut};
use mechtron_common::state::{State, NeutronStateInterface, StateMeta};
use mechtron_common::error::Error;
use mechtron_common::id::{Id, MechtronKey};
use mechtron_common::message::{Message, MessageBuilder};
use mechtron_common::mechtron::Context;
use mechtron_common::buffers::Buffer;
use crate::mechtron::{MessageHandler, Response};
use std::rc::Rc;
use std::cell::{Cell, RefCell};
use mechtron_common::artifact::Artifact;

lazy_static! {
  pub static ref BUFFERS: RwLock<HashMap<i32,Vec<u8>>> = RwLock::new(HashMap::new());
  pub static ref BUFFER_INDEX: AtomicI32 = AtomicI32::new(0);
  pub static ref STATE: Mutex<HashMap<i32,State>> = Mutex::new(HashMap::new());
}

pub static EMPTY : i32 = -1;
pub static ERROR : i32 = -2;


extern "C"
{
    pub fn wasm_init();

    pub fn mechtronium_log( type_ptr: *const u8, type_len: i32, message_ptr: *const u8, message_len: i32);
    pub fn mechtronium_cache( artifact_id: i32 );
    pub fn mechtronium_load( artifact_id: i32 )->i32;
    pub fn mechtronium_panic( buffer_id: i32 );
}


pub fn log( log_type: &str, string: &str ){
    unsafe
        {
            mechtronium_log(log_type.as_ptr(), log_type.len() as _, string.as_ptr(), string.len() as _ );
        }
}

pub fn panic( message: String )
{
    let buffer_id = wasm_write_string(message);
    unsafe {
        mechtronium_panic(buffer_id);
    }
}

pub fn mechtronium_read_string(buffer_id: i32) -> Result<String, Error>
{
    let buffers = BUFFERS.read()?;
    let bytes = buffers.get(&buffer_id).unwrap().to_vec();
    Ok(String::from_utf8(bytes )?)
}

pub fn mechtronium_consume_string(buffer_id: i32) -> Result<String, Error>
{
    let mut buffers = BUFFERS.write()?;
    let bytes = buffers.remove(&buffer_id).unwrap().to_vec();
    Ok(String::from_utf8(bytes)?)
}

pub fn mechtronium_consume_buffer(buffer_id: i32) -> Result<Vec<u8>, Error>
{
    let bytes = {
      let mut buffers = BUFFERS.write()?;
      buffers.remove(&buffer_id).unwrap()
    };
    Ok(bytes)
}

fn mechtronium_consume_messsage(buffer_id: i32) -> Result<Message, Error>
{
    let bytes = mechtronium_consume_buffer(buffer_id)?;
    Ok(Message::from_bytes(bytes,&CONFIGS)?)
}

fn mechtronium_consume_context(buffer_id: i32) -> Result<Context, Error>
{
    let bytes = mechtronium_consume_buffer(buffer_id)?;
    Ok(Context::from_bytes(bytes, &CONFIGS )?)
}






#[wasm_bindgen]
pub fn wasm_get_buffer_ptr(id: i32)->*const u8
{
    let buffer_info = BUFFERS.read();
    let buffer_info = buffer_info.unwrap();
    let buffer = buffer_info.get(&id).unwrap();
    return buffer.as_ptr()
}

#[wasm_bindgen]
pub fn wasm_get_buffer_len(id: i32)->i32
{
    let buffer_info = BUFFERS.read();
    let buffer_info = buffer_info.unwrap();
    let buffer = buffer_info.get(&id).unwrap();
    buffer.len() as _

}

#[wasm_bindgen]
pub fn wasm_dealloc_buffer(id: i32)
{
    let mut buffers= BUFFERS.write().unwrap();
    buffers.remove( &id );
}

#[wasm_bindgen]
pub fn wasm_alloc_buffer(len: i32) ->i32
{
    let buffer_id = BUFFER_INDEX.fetch_add(1, Ordering::Relaxed);
    {
        let mut buffers = BUFFERS.write().unwrap();
        let mut bytes: Vec<u8> = Vec::with_capacity(len as _);
        unsafe { bytes.set_len(len as _) }
        buffers.insert(buffer_id, bytes);
    }
    buffer_id
}

pub fn wasm_write_string(mut string: String) -> i32{
    let mut buffers = BUFFERS.write().unwrap();
    let buffer_id = BUFFER_INDEX.fetch_add(1, Ordering::Relaxed );
    unsafe {
        buffers.insert(buffer_id, string.as_mut_vec().to_vec());
    }
    buffer_id
}

pub fn wasm_write_buffer(mut bytes: Vec<u8>) -> i32{
    let mut buffers = BUFFERS.write().unwrap();
    let buffer_id = BUFFER_INDEX.fetch_add(1, Ordering::Relaxed );
    buffers.insert(buffer_id, bytes );
    buffer_id
}




#[wasm_bindgen]
pub fn wasm_inject_state(state_buffer_id: i32 ) -> i32
{
    let state = mechtronium_consume_buffer(state_buffer_id).unwrap();
    let state = State::from_bytes(state, &CONFIGS );
    if state.is_err()
    {
        log("ERROR", format!("STATE {} NOT OK", state_buffer_id).as_str());
        panic(format!("STATE {} NOT OK", state_buffer_id));
        return -1;
    }
    else {
        let state = state.unwrap();
        let mut states = STATE.lock().unwrap();
        states.insert(state_buffer_id, state);
        return 0;
    }
}

#[wasm_bindgen]
pub fn wasm_move_state_to_buffers(state_buffer_id: i32 )
{
    {
        let state = checkout_state(state_buffer_id);
        let bytes = state.to_bytes(&CONFIGS).unwrap();
        let mut buffers = BUFFERS.write().unwrap();
        buffers.insert(state_buffer_id, bytes);
    }
}



#[wasm_bindgen]
pub fn wasm_test_modify_state( state:i32 )
{
    let mut state = checkout_state(state);
    let interface = NeutronStateInterface{};
    let key = MechtronKey::new(Id::new(1,2), Id::new(3,4));
    state.set_taint(true);
    interface.add_mechtron(& mut state ,&key,"BlankMechtron".to_string());
}

#[wasm_bindgen]
pub fn wasm_test_log( )
{
    log("test", "this is a TEST log file from WASM");
}

#[wasm_bindgen]
pub fn wasm_cache( buffer_id: i32 )
{
    let artifact = mechtronium_consume_string(buffer_id).unwrap();
    let artifact = Artifact::from(&artifact).unwrap();
    CONFIGS.cache(&artifact).unwrap();
}


fn checkout_state(state_id: i32 ) ->State
{
    let states = STATE.lock();
    let mut states = states.unwrap();
    let state = states.remove(&state_id);
    if( state.is_none() )
    {
        log("ERROR", format!("state is not set for this state_id {}", state_id).as_str());
        panic(format!("state is not set for this state_id {}", state_id));
    }
    state.unwrap()
}

fn return_state(state: State, state_id: i32 )
{
    let states = STATE.lock();
    let mut states = states.unwrap();
    states.insert(state_id,state);
}

#[wasm_bindgen]
pub fn mechtron_is_tainted( state: i32) -> i32
{
    let states = STATE.lock();
    let states = states.unwrap();
    let state = states.get(&state).unwrap();
    if state.is_tainted().unwrap()
    {
        1
    }
    else
    {
        0
    }
}

#[wasm_bindgen]
pub fn mechtron_cache(artifact: i32) -> i32
{
    match mechtron_cache_inner(artifact)
    {
        Ok(_) => 0,
        Err(error) => {
            wasm_write_string(format!("ERROR {}",error))
        }
    }
}



#[wasm_bindgen]
pub fn mechtron_create(context: i32, state_id: i32, message: i32 ) -> i32
{
    match mechtron_create_inner(context,state_id,message)
    {
        Ok(builder_id) => builder_id,
        Err(err) => {
            panic(format!("{}",err));
            -1
        }
    }
}

#[wasm_bindgen]
pub fn mechtron_update(context: i32, state_id: i32) -> i32
{
    let state = checkout_state(state_id);
    let config = state.config.clone();
    let state = Rc::new(RefCell::new( Option::Some( Box::new(state) )));
    let context = mechtronium_consume_context(context ).unwrap();

    let mut mechtron = unsafe{
        mechtron(config.kind.as_str(),context.clone(),state.clone())
    }.unwrap();

    let response = mechtron.update().unwrap();

    let state = *state.replace(Option::None).unwrap();
    let (state,builder) = handle_response(response,state);
    return_state(state,state_id);

    builder
}

#[wasm_bindgen]
pub fn mechtron_message(context: i32, state_id: i32, message: i32) -> i32
{

    let state = checkout_state(state_id);
    let config = state.config.clone();
    let state = Rc::new(RefCell::new( Option::Some( Box::new(state) )));
    let context = mechtronium_consume_context(context ).unwrap();
    let message = mechtronium_consume_messsage(message).unwrap();

    let mut mechtron = unsafe{
        mechtron(config.kind.as_str(),context.clone(),state.clone())
    }.unwrap();


    let handler = mechtron.message( &message.to.port ).unwrap();


    let mut state = *state.replace(Option::None).unwrap();

    let response = match handler
    {
        MessageHandler::None => Response::None,
        MessageHandler::Handler(func) => func(&context,& mut state,message).unwrap()
    };

    let (state,builder) = handle_response(response,state);

    return_state(state,state_id);

    builder
}

#[wasm_bindgen]
pub fn mechtron_extra(context: i32, state_id: i32, message: i32) -> i32
{
    let state = checkout_state(state_id);
    let config = state.config.clone();
    let state = Rc::new(RefCell::new( Option::Some( Box::new(state) )));
    let context = mechtronium_consume_context(context ).unwrap();
    let message = mechtronium_consume_messsage(message).unwrap();

    let mut mechtron = unsafe{
        mechtron(config.kind.as_str(),context.clone(),state.clone())
    }.unwrap();

    let handler = mechtron.extra( &message.to.port ).unwrap();

    let mut state = *state.replace(Option::None).unwrap();

    let response = match handler
    {
        MessageHandler::None => Response::None,
        MessageHandler::Handler(func) => func(&context,& mut state,message).unwrap()
    };

    let (state,builder) = handle_response(response,state);
    return_state(state,state_id);

    builder
}





fn handle_response( response: Response, state: State )->(State,i32)
{
log("debug","handle_response");
    let builders= match response {
        Response::None => {
            -1
        }
        Response::Messages(builders) => {
for builder in &builders
{
    match builder.validate()
    {
        Ok(_) => {}
        Err(err) => {log("ERROR", format!("{:?}",err).as_str())}
    }

    match builder.validate_build()
    {
        Ok(_) => {}
        Err(err) => {log("ERROR", format!("{:?}",err).as_str())}
    }
    builder.validate_build().unwrap();
}
            let buffer = MessageBuilder::to_buffer(builders,&CONFIGS).unwrap();


            let bytes = Buffer::bytes(buffer);
            let buffer_id = wasm_write_buffer(bytes);
            buffer_id
        }
    };

    (state,builders)
}

pub fn mechtron_cache_inner(artifact: i32) -> Result<(),Error>
{
    let artifact = mechtronium_consume_string(artifact)?;
    let artifact = Artifact::from(artifact.as_str() )?;
    CONFIGS.cache(&artifact)?;
    Ok(())
}


pub fn mechtron_create_inner(context: i32, state_id: i32, message: i32 ) -> Result<i32,Error>
{
    let state = checkout_state(state_id);
    let config = state.config.clone();
    let state = Rc::new(RefCell::new( Option::Some( Box::new(state) )));
    let context = mechtronium_consume_context(context )?;
    let message = mechtronium_consume_messsage(message)?;

    let mut mechtron = unsafe{
        mechtron(config.kind.as_str(),context.clone(),state.clone())
    }.unwrap();

log("debug", "precreate");
    let response = mechtron.create(&message)?;
log("debug","postcreate");

    let state = state.replace(Option::None);
    let state = state.unwrap();

    let (state,builder) = handle_response(response,*state);
    return_state(state,state_id);

    Ok(builder)
}
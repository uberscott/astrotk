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

lazy_static! {
  pub static ref BUFFERS: RwLock<HashMap<i32,BufferInfo>> = RwLock::new(HashMap::new());
  pub static ref BUFFER_INDEX: AtomicI32 = AtomicI32::new(0);
  pub static ref STATE: Mutex<HashMap<i32,State>> = Mutex::new(HashMap::new());
}

pub static EMPTY : i32 = -1;
pub static ERROR : i32 = -2;

pub struct BufferInfo
{
    len: usize,
    ptr: AtomicPtr<u8>
}
extern "C"
{
    pub fn wasm_init();

    pub fn mechtronium_log( type_ptr: *const u8, type_len: i32, message_ptr: *const u8, message_len: i32);
    pub fn mechtronium_cache( artifact_id: i32 );
    pub fn mechtronium_load( artifact_id: i32 )->i32;
}

pub fn log( log_type: &str, string: &str ){
    unsafe
        {
            mechtronium_log(log_type.as_ptr(), log_type.len() as _, string.as_ptr(), string.len() as _ );
        }
}

pub fn mechtronium_read_string(buffer_id: i32) -> Result<String, Error>
{
    let buffers = BUFFERS.read()?;
    let buffer = buffers.get(&buffer_id);
    if buffer.is_none()
    {
        return Err("could not find string buffer".into());
    }
    let buffer_info = buffer.unwrap();

    unsafe {
        Ok(String::from_raw_parts(buffer_info.ptr.load(Ordering::Relaxed), buffer_info.len.clone() as _, buffer_info.len.clone() as _))
    }
}

pub fn mechtronium_consume_string(buffer_id: i32) -> Result<String, Error>
{
    let mut buffers = BUFFERS.write()?;
    let buffer = buffers.remove(&buffer_id);
    if buffer.is_none()
    {
        return Err("could not find string buffer".into());
    }
    let buffer_info = buffer.unwrap();

    unsafe {
        Ok(String::from_raw_parts(buffer_info.ptr.load(Ordering::Relaxed), buffer_info.len.clone() as _, buffer_info.len.clone() as _))
    }
}

pub fn mechtronium_read_buffer(buffer_id: i32) -> Result<Vec<u8>, Error>
{
    let buffers = BUFFERS.read()?;
    let buffer = buffers.get(&buffer_id);
    if buffer.is_none()
    {
        return Err("could not find string buffer".into());
    }
    let buffer_info = buffer.unwrap();

    unsafe {

        Ok(Vec::from_raw_parts(buffer_info.ptr.load(Ordering::Relaxed), buffer_info.len.clone() as _, buffer_info.len.clone() as _))
    }
}

pub fn mechtronium_consume_buffer(buffer_id: i32) -> Result<Vec<u8>, Error>
{
    let mut buffers = BUFFERS.write()?;
    let buffer = buffers.remove(&buffer_id);
    if buffer.is_none()
    {
        log("ERROR", format!("could not consume buffer {}",buffer_id).as_str());
        return Err("could not find string buffer".into());
    }
log("debug", "unwrapping buffer");
    let buffer_info = buffer.unwrap();
log("debug", format!("ptr: {:?}",buffer_info.ptr).as_str());
log("debug", format!("len : {}",buffer_info.len.clone()).as_str());

    unsafe {
log("debug", "doing the unsafe...");
        Ok(Vec::from_raw_parts(buffer_info.ptr.load(Ordering::Relaxed), buffer_info.len.clone() as _, buffer_info.len.clone() as _))
    }
}

fn mechtronium_consume_messsage(buffer_id: i32) -> Result<Message, Error>
{
    //let bytes = mechtronium_consume_buffer(buffer_id)?;
    let bytes = mechtronium_read_buffer(buffer_id)?;
    Ok(Message::from_bytes(bytes,&CONFIGS)?)
}

fn mechtronium_consume_context(buffer_id: i32) -> Result<Context, Error>
{
    let bytes = mechtronium_consume_buffer(buffer_id)?;
    Ok(Context::from_bytes(bytes, &CONFIGS )?)
}

fn wasm_alloc(len: i32) -> *mut u8 {
    /*let rtn = unsafe {
        let align = mem::align_of::<u8>();
        let size = mem::size_of::<u8>();
        let layout = Layout::from_size_align(size * (len as usize), align).unwrap();
        alloc(layout)
    };
     */
    let mut vec = Vec::with_capacity(len as _ );
    let mut vec = mem::ManuallyDrop::new(vec);
    unsafe {
        vec.set_len(len as _);
    }
    vec.as_mut_ptr()
}

fn wasm_dealloc(ptr: *mut u8, len: i32) {
    unsafe {
        let align = mem::align_of::<u8>();
        let size = mem::size_of::<u8>();
        let layout = Layout::from_size_align(size * (len as usize), align).unwrap();
        dealloc(ptr, layout)
    };
}




#[wasm_bindgen]
pub fn wasm_get_buffer_ptr(id: i32)->*const u8
{
    let buffer_info = BUFFERS.read();
    let buffer_info = buffer_info.unwrap();
    let buffer_info = buffer_info.get(&id).unwrap();
    buffer_info.ptr.load(Ordering::Relaxed)
}

#[wasm_bindgen]
pub fn wasm_get_buffer_len(id: i32)->i32
{
    let buffer_info = BUFFERS.read();
    let buffer_info = buffer_info.unwrap();
    let buffer_info = buffer_info.get(&id).unwrap();
    buffer_info.len.clone() as _
}

#[wasm_bindgen]
pub fn wasm_dealloc_buffer(id: i32)
{
    let buffer_info = BUFFERS.read();
    let buffer_info = buffer_info.unwrap();
    match buffer_info.get(&id)
    {
        Some(buffer_info)=> {
            let ptr = buffer_info.ptr.load(Ordering::Relaxed);
            wasm_dealloc(ptr, buffer_info.len as _);
        },
        None=>{}
    }
}

#[wasm_bindgen]
pub fn wasm_alloc_buffer(len: i32) ->i32
{
log("debug", format!("allocating buffer of size: {}",len).as_str());
    let buffer_id = BUFFER_INDEX.fetch_add(1, Ordering::Relaxed );
    let buffer_ptr = wasm_alloc(len);
    {
        let mut buffers = BUFFERS.write().unwrap();
        let buffer_info = BufferInfo{
            ptr: AtomicPtr::new(buffer_ptr),
            len: len as _
        };
        buffers.insert( buffer_id, buffer_info  );
    }

    buffer_id
}


pub fn wasm_write_string(mut string: String) -> i32{
    let rtn = wasm_assign_buffer(string.as_mut_ptr(), string.len() as _ );
    mem::forget(string);
    rtn
}

pub fn wasm_write_buffer(mut bytes: Vec<u8>) -> i32{
    let rtn = wasm_assign_buffer(bytes.as_mut_ptr(), bytes.len() as _ );
    mem::forget(bytes );
    rtn
}


pub fn wasm_assign_buffer(ptr: *mut u8, len: i32) -> i32{
    let buffer_id = BUFFER_INDEX.fetch_add(1, Ordering::Relaxed );
    let buffer_info = BufferInfo{
        ptr: AtomicPtr::new(ptr),
        len: len as _
    };
    {
        let mut buffers = BUFFERS.write().unwrap();
        buffers.insert( buffer_id, buffer_info  );
    }
    buffer_id
}

#[wasm_bindgen]
pub fn wasm_inject_state(state_buffer_id: i32 )
{
    let state = mechtronium_consume_buffer(state_buffer_id).unwrap();
    let state = State::from_bytes(state, &CONFIGS ).unwrap();

    let mut states = STATE.lock().unwrap();
    states.insert(state_buffer_id, state );
}

#[wasm_bindgen]
pub fn wasm_move_state_to_buffers(state_buffer_id: i32 )
{
    let state = checkout_state(state_buffer_id);
    let mut bytes = state.to_bytes(&CONFIGS).unwrap();
    let buffer_info = BufferInfo{
        ptr: AtomicPtr::new(bytes.as_mut_ptr() ),
        len: bytes.len() as _
    };
    {
        BUFFERS.write().unwrap().insert(state_buffer_id, buffer_info);
    }
    mem::forget(bytes);
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
    log("test", "this is a log file");
}


fn checkout_state(state_id: i32 ) ->State
{
    let states = STATE.lock();
    let mut states = states.unwrap();
    let state = states.remove(&state_id).unwrap();
    state
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
pub fn mechtron_create(context: i32, state_id: i32, message: i32 ) -> i32
{
    let state = checkout_state(state_id);
    let config = state.config.clone();
    let state = Rc::new(RefCell::new( Option::Some( Box::new(state) )));
    let context = mechtronium_consume_context(context ).unwrap();
    let message = mechtronium_consume_messsage(message).unwrap();

    let mut mechtron = unsafe{
        mechtron(config.kind.as_str(),context.clone(),state.clone())
    }.unwrap();

    let response = mechtron.create(&message).unwrap();

    let state = state.replace(Option::None);
    let state = state.unwrap();

    let (state,builder) = handle_response(response,*state);
    return_state(state,state_id);

    builder
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

log("debug","mechtron_message() entry");

    let state = checkout_state(state_id);
log("debug","checkout state");
    let config = state.config.clone();
log("debug","clone config ");
    let state = Rc::new(RefCell::new( Option::Some( Box::new(state) )));
log("debug","new refcel");
    let context = mechtronium_consume_context(context ).unwrap();
log("debug","contet consumed");
    let message = mechtronium_consume_messsage(message).unwrap();
log("debug","mechtron_message() message consumed");

    let mut mechtron = unsafe{
        mechtron(config.kind.as_str(),context.clone(),state.clone())
    }.unwrap();

log("debug","created mechtron");

    let handler = mechtron.message( &message.to.port ).unwrap();

log("debug","message requested");

    let mut state = *state.replace(Option::None).unwrap();

log("debug","Ogres are people too");
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


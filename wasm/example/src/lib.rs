mod utils;

use mechtron_wasm::*;
use no_proto::buffer::NP_Buffer;
use no_proto::buffer_ro::NP_Buffer_RO;

#[no_mangle]
pub extern "C" fn mechtron_create(ctx: &MechtronContext, create_message: &NP_Buffer_RO, content: &mut NP_Buffer ) -> Result<(),Box<std::error::Error>>
{
   log("actor_create() called! XX");
   let name = create_message.get::<String>(&[&"name"]);
   log( format!("name result {:?}",name).as_str());
   let name = name.unwrap().unwrap();
   content.set(&[&"name"], name.clone());
   let age = create_message.get::<i32>(&[&"age"]).unwrap().unwrap();

   log( format!("name is: {}",name.as_str()).as_str() );

   content.set( &[&"age"], age+1 );

   return Ok(());
}




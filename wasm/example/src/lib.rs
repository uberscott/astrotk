mod utils;

use astrotk_wasm::*;

#[no_mangle]
pub extern "C" fn actor_create( ctx: &ActorContext ) -> Result<(),Box<std::error::Error>>
{
   log("actor_create() called!");
   return Ok(());
}


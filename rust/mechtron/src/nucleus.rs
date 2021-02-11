use std::sync::{RwLock, PoisonError, RwLockWriteGuard};
use std::collections::HashMap;
use std::error::Error;
use std::collections::hash_map::RandomState;
use crate::tron::{Tron, InitContext, UpdateContext, CreateContext};
use mechtron_common::message::Message;
use no_proto::buffer::NP_Buffer;
use no_proto::memory::NP_Memory_Owned;

pub struct NucleiStore
{
    // list of nuclei
    nuclei: Vec<i64>
}

// is there really a need for this?  maybe it should just be a vec in Source
impl NucleiStore
{
    pub fn new()->Self{
        NucleiStore{ nuclei: vec!() }
    }

    pub fn add( &mut self, nucleus_id: i64 )->Result<(),Box<dyn Error+'_>>
    {
        if self.nuclei.contains(&nucleus_id) {
            return Err(format!("already contains nucleus id {}",nucleus_id).into())
        }

        self.nuclie.push(nucleus_id);

        Ok(())
    }

    pub fn get( &self )->&Vec<i64>
    {
        return &self.nuclei;
    }
}


struct NueTron
{

}

impl Tron for NueTron {

    fn init(context: InitContext) -> Result<Box<Self>, Box<dyn Error>> where Self: Sized {
        Ok(Box::new(NueTron {}))
    }

    fn create(&self, context: &dyn CreateContext, content: &mut NP_Buffer<NP_Memory_Owned>, create: &Message) -> Result<Option<Vec<Message>>, Box<dyn Error>> {
        content.list_push(&[&"tron_ids"], context.id() );
        content.list_push(&[&"tron_names",&"neutron"], context.id() );
        Ok(Option::None)
    }

    fn update(&self, context: &dyn UpdateContext, content: &NP_Buffer<NP_Memory_Owned>, messages: Vec<&Message>) -> Result<Option<Vec<Message>>, Box<dyn Error>> {
        Ok(Option::None)
    }
}

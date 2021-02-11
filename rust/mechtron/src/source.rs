use crate::content::{ContentStore};
use crate::nucleus::NucleiStore;
use crate::message::{MessageStore, MessageIntake};
use std::error::Error;
use mechtron_common::revision::Revision;
use mechtron_common::id::Id;
use mechtron_common::id::TronKey;
use mechtron_common::id::Revision;

pub struct Source
{
    sim_id: Id,
    pub content: ContentStore,
    pub nuclei: Vec<Id>,
    pub messages: MessageStore
}

impl Source
{
    pub fn new(sim_id:Id)->Self{
        Source{
            sim_id: sim_id,
            content: ContentStore::new(),
            nuclei: vec!(),
            messages: MessageStore::new()
        }
    }

    pub fn id(&self)->i64
    {
        self.id
    }

    pub fn add_nucleus(&mut self, nucleus_id: Id, nuctron_id: Id) -> Result<(),Box<dyn Error>>
    {
        self.nuclei.push(nucleus_id.clone() );
        self.content.create(&TronKey::new(nucleus_id, nuctron_id ) )?;
        return Ok(())
    }

    pub fn revise( from: Revision, to: Revision )->Result<(),Box<dyn Error>>
    {
        if from.cycle != to.cycle-1
        {
            return Err("cycles must be sequential. 'from' revision cycle must be exactly 1 less than 'to' revision cycle".into());
        }



        Ok(())
    }
}
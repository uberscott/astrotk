use crate::content::{ContentStore, TronKey};
use crate::nucleus::NucleiStore;
use crate::message::{MessageStore, MessageIntake};
use std::error::Error;
use mechtron_common::revision::Revision;

pub struct Source
{
    sim_id: i64,
    pub content: ContentStore,
    pub nuclei: Vec<i64>,
//    pub messages: MessageStore<'a>
}

impl Source
{
    pub fn new(sim_id:i64)->Self{
        Source{
            sim_id: sim_id,
            content: ContentStore::new(),
            nuclei: vec!(),
//            messages: MessageStore::new()
        }
    }

    pub fn id(&self)->i64
    {
        self.id
    }

    pub fn add_nuclues(&mut self, nucleus_id: i64, nuctron_id: i64 ) -> Result<(),Box<dyn Error>>
    {
        self.nuclei.push(nucleus_id);
        self.content.create(&TronKey::new(nucleus_id, nuctron_id ) )?;
        return Ok(())
    }

    pub fn revise( from: Revision, to: Revision )
    {

    }
}
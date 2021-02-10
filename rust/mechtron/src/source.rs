use crate::content::ContentStore;
use crate::nucleus::NucleiStore;
use crate::message::{MessageStore, MessageIntake};

pub struct Source<'a>
{
    sim_id: i64,
    content: ContentStore<'a>,
    nuclei: NucleiStore,
    messages: MessageStore<'a>
}

impl <'a> Source<'a>
{
    pub fn new(sim_id:i64)->Self{
        Source{
            sim_id: sim_id,
            content: ContentStore::new(),
            nuclei: NucleiStore::new(),
            messages: MessageStore::new()
        }
    }


}
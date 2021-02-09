use std::collections::HashMap;
use std::error::Error;

use crate::message::{MessageIntakeChamber, MessageChamber};
use crate::tron::Tron;
use crate::simulation::Simulation;
use crate::app::SYS;
use mechtron_common::revision::Revision;

pub struct Nucleus<'a> {
    id: i64,
    sim_id: i64,
    trons: HashMap<i64, Box<dyn Tron>>,
    tron_lookup: HashMap<String, i64>,
    message_intake_chamber: MessageIntakeChamber<'a>
}


impl <'a> Nucleus<'a> {
    pub fn new(id:i64,sim_id:i64)->Self
    {
        Nucleus { id: id, trons: HashMap::new(), tron_lookup: HashMap::new(), sim_id: sim_id, message_intake_chamber: MessageIntakeChamber::new() }
    }

    pub fn add( &mut self, tron_id: i64, tron: Box<dyn Tron> )
    {
        self.trons.insert(tron_id, tron );
    }

    pub fn bind_lookup_name( &mut self, tron_id:i64, lookup_name: &str )->Result<(),Box<dyn Error>>
    {
        if self.tron_lookup.contains_key(lookup_name) {
            return Err(format!("tron already bound to name: {}", lookup_name).into());
        }

        self.tron_lookup.insert(lookup_name.to_string(), tron_id );
        Ok(())
    }

    pub fn id(&self)->i64
    {
        return self.id;
    }

    pub fn sim_id(&self)->i64
    {
        return self.sim_id;
    }

    pub fn revise(&mut self, ctx: &RevisionContext )
    {
        let messages = self.message_intake_chamber.messages(&ctx.revision);
    }
}



struct RevisionContext<'a>
{
   simulation: &'a Simulation,
   revision: Revision
}





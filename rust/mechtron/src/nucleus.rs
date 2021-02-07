use crate::tron::Tron;
use std::collections::HashMap;
use std::error::Error;

pub struct Nucleus {
    id: i64,
    sim_id: i64,
    trons: HashMap<i64,Box<dyn Tron>>,
    tron_lookup: HashMap<String,i64>
}


impl Nucleus {

    pub fn new(id:i64,sim_id:i64)->Self
    {
        Nucleus{id:id, trons:HashMap::new(), tron_lookup:HashMap::new(),sim_id: sim_id}
    }

    pub fn add( &mut self, tron_id: i64, tron: Box<dyn Tron> )
    {
        self.trons.insert(tron_id, tron );
    }

    pub fn bind_lookup_name( &mut self, tron_id:i64, lookup_name: &str )->Result<(),Box<dyn Error>>
    {
        if self.tron_lookup.contains_key(lookup_name) {
            return Err(format!("tron already bound to nane: {}", lookup_name).into());
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

}

use std::sync::{RwLock, PoisonError, RwLockWriteGuard};
use std::collections::HashMap;
use std::error::Error;
use std::collections::hash_map::RandomState;

pub struct NucleiStore
{
    // simulation id to list of nuclei
    nuclei: RwLock<HashMap<i64,RwLock<Vec<i64>>>>
}

impl NucleiStore
{
    pub fn new()->Self{
        NucleiStore{ nuclei: RwLock::new(HashMap::new()) }
    }

    pub fn add_to_sim( &mut self, sim_id: i64, nucleus_id: i64 )->Result<(),Box<dyn Error+'_>>
    {
        let mut result = self.nuclei.write();
        match result{
            Ok(_) => {}
            Err(_) => return Err("lock error when attempting to add to sim".into())
        }
        let mut nuclie = result.unwrap();

        if !nuclie.contains_key(&sim_id)
        {
            nuclie.insert( sim_id, RwLock::new(vec!()));
        }

        let mut nuclie = nuclie.get(&sim_id).unwrap();
        let mut result = nuclie.write();

        match result{
            Ok(_) => {}
            Err(_) => return Err("lock error when attempting to add to sim".into())
        }
        let mut nuclie = result.unwrap();

        nuclie.push(nucleus_id);

        Ok(())
    }

    pub fn get_for_sim( &self, sim_id: i64 )->Result<Vec<i64>,Box<dyn Error+'_>>
    {
        let result = self.nuclei.read();
        match result{
            Ok(_) => {}
            Err(_) => return Err("lock error when attempting to get_for_sim".into())
        }

        let nuclie = result.unwrap();
        let nuclie = nuclie.get(&sim_id);


        if nuclie.is_none(){
            return Err(format!("sim not found in this nuclues store: {}",sim_id).into());
        }

        let nuclie = nuclie.unwrap();
        let result = nuclie.read();

        match result{
            Ok(_) => {}
            Err(_) => return Err("lock error when attempting to get_for_sim".into())
        }

        let nuclie = result.unwrap();
        let nuclie = nuclie.clone();

        Ok(nuclie)
    }
}



use std::sync::RwLock;
use std::collections::HashMap;
use std::error::Error;

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

    pub fn add_to_sim( &mut self, sim_id: i64, nucleus_id: i64 )->Result<(),Box<dyn Error>>
    {
        let mut nuclie= self.nuclei.write()?;

        if !nuclie.contains_key(&sim_id)
        {
            nuclie.insert( sim_id, RwLock::new(vec!()));
        }

        let mut nuclie = nuclie.get(&sim_id).unwrap();
        let mut nuclie = nuclie.write()?;
        nuclie.push(nucleus_id);

        Ok(())
    }

    pub fn get_for_sim( &self, sim_id: i64 )->Result<Vec<i64>,Box<dyn Error>>
    {
        let nuclie= self.nuclei.read()?;
        let nuclie = nuclie.get(&sim_id);
        if nuclie.is_none(){
            return Err(format!("sim not found in this nuclues store: {}",sim_id).into());
        }

        let nuclie = nuclie.unwrap();
        let nuclie = nuclie.read()?;
        let nuclie = nuclie.clone();

        Ok(nuclie);
    }
}



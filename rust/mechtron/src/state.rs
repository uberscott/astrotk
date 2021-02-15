use no_proto::buffer::NP_Buffer;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock, PoisonError, RwLockWriteGuard, RwLockReadGuard};
use std::error::Error;
use no_proto::memory::NP_Memory_Owned;
use mechtron_common::id::{Revision, TronKey, StateKey, Id};
use mechtron_common::state::{State, ReadOnlyState};
use mechtron_common::configs::Configs;
use std::collections::hash_map::RandomState;


pub struct NucleusStateStructure {
    history: HashMap<Revision,HashMap<TronKey,ReadOnlyState>>
}


impl NucleusStateStructure
{
   pub fn new() -> Self{
       NucleusStateStructure {
           history: HashMap::new()
       }
   }

   fn unwrap<V>( &self, option: Option<V> ) -> Result<V,Box<dyn Error>>
   {
       match option{
           None => Err("option was none".into()),
           Some(value) => value
       }
   }

   pub fn get(&self, key:&StateKey) -> Result<&ReadOnlyState,Box<dyn Error>>
   {
       let history = self.unwrap(self.history.get(&key.revision ))?;
       let state = self.unwrap( history.get(&key.tron ))?;
       Ok(state)
   }

   pub fn query( &self, revision: &Revision ) -> Option<Vec<(TronKey,&ReadOnlyState)>>
   {
       let history = self.history.get(&revision );
       match history
       {
           None => Option::None,
           Some(history) => {
               let mut rtn = vec!();
               for key in history.keys()
               {
                   let state = history.get(key).unwrap();
                   rtn.push( (key,state) )
               }
               Ok(rtn)
           }
       }
   }

}

impl StateIntake for NucleusStateStructure
{
    fn intake(&mut self,state: State, key: StateKey) -> Result<(), Box<dyn Error+'_>> {
        let state = State::read_only(state)?;
        let history = self.history.entry(key.revision.clone()).or_insert(HashMap::new());
        history.insert(key.tron_id.clone(), state );
        Ok(())
    }

}


pub trait StateIntake
{
    fn intake( &mut self, state: State, key: StateKey )->Result<(),Box<dyn Error+'_>>;
}

pub trait ReadOnlyStateIntake
{
    fn intake( &mut self, state: ReadOnlyState, key: StateKey )->Result<(),Box<dyn Error+'_>>;
}

pub trait ReadOnlyStateAccess
{
    fn get( &self, key: &StateKey )->Result<&ReadOnlyState,Box<dyn Error+'_>>;
}

pub trait StateAccess
{
    fn get( &mut self, key: &StateKey )->Result<&State,Box<dyn Error+'_>>;
}

pub struct NucleusPhasicStateStructure
{
    store: HashMap<TronKey,State>
}

impl NucleusPhasicStateStructure
{
    pub fn new( ) -> Self {
        NucleusPhasicStateStructure{
            store: HashMap::new()
        }
    }

    pub fn intake( &mut self, key: TronKey, state: &ReadOnlyState) -> Result<(),Box<dyn Error>>
    {
       self.store.insert(key, state.copy()? );
       Ok(())
    }

    pub fn get(&mut self, key: &TronKey )->Result<&mut State,Box<dyn Error>>
    {
        self.store.get_mut(key).ok_or("could not find state".into() )
    }

    pub fn drain(&mut self, revision: &Revision )->Vec<(StateKey,State)>
    {
        let mut rtn = vec!();

        for key in self.store.keys(){
            let state = self.store.remove(key).unwrap();
            rtn.push( (StateKey{tron: key.clone(),revision:revision.clone()},state) );
        }

        return rtn;
    }

}

#[cfg(test)]
mod tests {

#[test]
fn test()
{

}


}
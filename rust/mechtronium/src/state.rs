use mechtron_core::configs::Configs;
use mechtron_core::id::{Id, Revision, StateKey, TronKey};
use mechtron_core::state::{ReadOnlyState, State};
use no_proto::buffer::NP_Buffer;
use no_proto::memory::NP_Memory_Owned;
use std::collections::hash_map::{Keys, RandomState};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::rc::Rc;
use std::sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

pub struct NucleusStateStructure {
    history: HashMap<Revision, HashMap<TronKey, Arc<ReadOnlyState>>>,
}

impl NucleusStateStructure {
    pub fn new() -> Self {
        NucleusStateStructure {
            history: HashMap::new(),
        }
    }

    fn unwrap<V>(&self, option: Option<V>) -> Result<V, Box<dyn Error>> {
        match option {
            None => return Err("option was none".into()),
            Some(value) => Ok(value),
        }
    }

    pub fn get(&self, key: &StateKey) -> Result<Arc<ReadOnlyState>, Box<dyn Error>> {
        let history = self.unwrap(self.history.get(&key.revision))?;
        let state = self.unwrap(history.get(&key.tron))?;
        Ok(state.clone())
    }

    pub fn read_only(&self, key: &StateKey) -> Result<Arc<ReadOnlyState>, Box<dyn Error>> {
        Ok(self.get(key)?.clone())
    }

    pub fn query(&self, revision: &Revision) -> Option<Vec<(TronKey, &ReadOnlyState)>> {
        let history = self.history.get(&revision);
        match history {
            None => Option::None,
            Some(history) => {
                let mut rtn: Vec<(TronKey, &ReadOnlyState)> = vec![];
                for key in history.keys() {
                    let state = history.get(key).unwrap();
                    rtn.push((key.clone(), state))
                }
                Option::Some(rtn)
            }
        }
    }
}

impl StateIntake for NucleusStateStructure {
    fn intake(&mut self, state: Rc<State>, key: StateKey) -> Result<(), Box<dyn Error + '_>> {
        let state = state.read_only()?;
        let history = self
            .history
            .entry(key.revision.clone())
            .or_insert(HashMap::new());
        history.insert(key.tron.clone(), Arc::new(state));
        Ok(())
    }
}

pub trait StateIntake {
    fn intake(&mut self, state: Rc<State>, key: StateKey) -> Result<(), Box<dyn Error + '_>>;
}

pub trait ReadOnlyStateIntake {
    fn intake(&mut self, state: ReadOnlyState, key: StateKey) -> Result<(), Box<dyn Error + '_>>;
}

pub trait ReadOnlyStateAccess {
    fn get(&self, key: &StateKey) -> Result<&ReadOnlyState, Box<dyn Error + '_>>;
}

pub trait StateAccess {
    fn get(&mut self, key: &StateKey) -> Result<&State, Box<dyn Error + '_>>;
    fn read_only(&mut self, key: &StateKey) -> Result<&ReadOnlyState, Box<dyn Error + '_>>;
}

pub struct NucleusPhasicStateStructure {
    store: HashMap<TronKey, Rc<State>>,
}

impl NucleusPhasicStateStructure {
    pub fn new() -> Self {
        NucleusPhasicStateStructure {
            store: HashMap::new(),
        }
    }

    pub fn intake(&mut self, key: TronKey, state: &ReadOnlyState) -> Result<(), Box<dyn Error>> {
        self.store.insert(key, Rc::new(state.copy()));
        Ok(())
    }

    pub fn get(&self, key: &TronKey) -> Result<Rc<State>, Box<dyn Error>> {
        let rtn = self.store.get(key);

        match rtn {
            None => Err("could not find state".into()),
            Some(rtn) => Ok(rtn.clone()),
        }
    }

    pub fn drain(&mut self, revision: &Revision) -> Vec<(StateKey, Rc<State>)> {
        let mut rtn = vec![];
        for (key, state) in self.store.drain() {
            let key = StateKey {
                tron: key.clone(),
                revision: revision.clone(),
            };
            rtn.push((key, state));
        }
        return rtn;
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test() {}
}

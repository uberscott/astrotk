use mechtron_core::configs::Configs;
use mechtron_core::id::{Id, Revision, StateKey, TronKey};
use mechtron_core::state::{ReadOnlyState, State};
use no_proto::buffer::NP_Buffer;
use no_proto::memory::NP_Memory_Owned;
use std::collections::hash_map::{Keys, RandomState};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard, Mutex, MutexGuard};
use crate::nucleus::NucleusError;

pub struct NucleusStateHistory {
    history: RwLock<HashMap<Revision, HashMap<TronKey, Arc<ReadOnlyState>>>>,
}

impl NucleusStateHistory {
    pub fn new() -> Self {
        NucleusStateHistory {
            history: RwLock::new(HashMap::new()),
        }
    }

    fn unwrap<V>(&self, option: Option<V>) -> Result<V, NucleusError> {
        match option {
            None => return Err("option was none".into()),
            Some(value) => Ok(value),
        }
    }

    pub fn get(&self, key: &StateKey) -> Result<Arc<ReadOnlyState>, NucleusError> {
        let history = self.history.read()?;
        let history = self.unwrap(history.get(&key.revision))?;
        let state = self.unwrap(history.get(&key.tron))?;
        Ok(state.clone())
    }

    pub fn read_only(&self, key: &StateKey) -> Result<Arc<ReadOnlyState>, NucleusError> {
        Ok(self.get(key)?.clone())
    }

    pub fn query(&self, revision: &Revision) -> Result<Option<Vec<(TronKey, Arc<ReadOnlyState>)>>,NucleusError> {
        let history = self.history.read()?;
        let history = history.get(&revision);
        match history {
            None => Ok(Option::None),
            Some(history) => {
                let mut rtn: Vec<(TronKey, Arc<ReadOnlyState>)> = vec![];
                for key in history.keys() {
                    let state = history.get(key).unwrap();
                    rtn.push((key.clone(), state.clone()))
                }
                Ok(Option::Some(rtn))
            }
        }
    }
}

impl NucleusStateHistoryIntake for NucleusStateHistory {
    fn intake(&mut self, state: Rc<State>, key: StateKey) -> Result<(), NucleusError> {
        let state = state.read_only()?;
        let mut history = self.history.write()?;
        let mut history =
            history
            .entry(key.revision.clone())
            .or_insert(HashMap::new());
        history.insert(key.tron.clone(), Arc::new(state));
        Ok(())
    }
}

pub trait NucleusStateHistoryIntake {
    fn intake(&mut self, state: Rc<State>, key: StateKey) -> Result<(), NucleusError>;
}


pub trait NucleusCycleStateIntake {
    fn intake(&mut self, state: ReadOnlyState, key: StateKey) -> Result<(), NucleusError>;
}

pub struct NucleusCycleStateStructure {
    store: RwLock<HashMap<TronKey, Arc<Mutex<State>>>>,
}

impl NucleusCycleStateStructure {
    pub fn new() -> Self {
        NucleusCycleStateStructure {
            store: RwLock::new(HashMap::new()),
        }
    }

    pub fn intake(&mut self, key: TronKey, state: &ReadOnlyState) -> Result<(), NucleusError> {
        let mut store = self.store.write()?;
        store.insert(key, Arc::new(Mutex::new(state.copy()) ));
        Ok(())
    }

    pub fn get(&mut self, key: &TronKey) -> Result<Arc<Mutex<State>>, NucleusError> {

        let store = self.store.read()?;
        let rtn = store.get(key);

        match rtn {
            None => Err("could not find state".into()),
            Some(rtn) => {
                Ok(rtn.clone())
            }
        }
    }

    pub fn drain(&mut self, revision: &Revision) -> Result<Vec<(StateKey, Arc<Mutex<State>>)>,NucleusError> {
        let mut store = self.store.write()?;
        let mut rtn = vec![];
        for (key, state) in store.drain() {
            let key = StateKey {
                tron: key.clone(),
                revision: revision.clone(),
            };
            rtn.push((key, state));
        }
        return Ok(rtn);
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test() {}
}

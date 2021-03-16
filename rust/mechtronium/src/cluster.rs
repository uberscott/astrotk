use mechtron_common::id::{IdSeq, Id};
use std::collections::{HashMap, HashSet};

pub struct Cluster
{
    pub seq: IdSeq,
    pub nucleus_to_node: HashMap<Id,Id>,
    pub simulation_supervisor_to_star: HashMap<Id,Id>,
    pub available_supervisors: Vec<Id>,
    pub available_supervisors_round_robin_index: usize
}

impl Cluster
{
    pub fn new()->Self
    {
        Cluster{
            seq: IdSeq::new(0),
            nucleus_to_node: HashMap::new(),
            simulation_supervisor_to_star: HashMap::new(),
            available_supervisors: vec!(),
            available_supervisors_round_robin_index: 0,
        }
    }


}
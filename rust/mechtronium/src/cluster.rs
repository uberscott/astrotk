use mechtron_common::id::{IdSeq, Id};
use std::collections::HashMap;

pub struct Cluster
{
    pub seq: IdSeq,
    pub nucleus_to_node: HashMap<Id,Id>
}

impl Cluster
{
    pub fn new()->Self
    {
        Cluster{
            seq: IdSeq::new(0),
            nucleus_to_node: HashMap::new()
        }
    }


}
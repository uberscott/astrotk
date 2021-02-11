use std::sync::atomic::{AtomicI64, Ordering};

#[derive(PartialEq,Eq,PartialOrd,Ord,Hash,Debug,Clone)]
pub struct Id
{
    pub seq_id: i64,
    pub id: i64
}

pub struct IdSeq
{
   seq_id: i64,
   seq: AtomicI64
}

impl IdSeq {
    pub fn new( seq_id : i64 )->Self
    {
        IdSeq{
            seq_id:seq_id,
            seq: AtomicI64::new(0)
        }
    }

    pub fn next(&mut self)->Id
    {
        Id{
            seq_id:self.seq_id,
            id: self.seq.fetch_add(1,Ordering::Relaxed )
        }
    }
}

#[derive(PartialEq,Eq,PartialOrd,Ord,Hash,Debug,Clone)]
pub struct TronKey
{
    nucleus_id: Id,
    tron_id: Id
}

impl TronKey{
    pub fn new( nucleus_id: Id, tron_id: Id )->TronKey{
        TronKey{
            nucleus_id: nucleus_id,
            tron_id: tron_id
        }
    }
}

#[derive(PartialEq,Eq,PartialOrd,Ord,Hash,Debug,Clone)]
pub struct ContentKey
{
    content_id: TronKey,
    revision: Revision
}


#[derive(PartialEq,Eq,PartialOrd,Ord, Hash,Debug,Clone)]
pub struct Revision
{
    pub cycle: i64
}

pub struct DeliveryMomentKey
{
    pub cycle: i64,
    pub phase: u8
}



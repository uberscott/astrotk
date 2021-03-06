use crate::buffers::{Buffer, Path, ReadOnlyBuffer};
use std::sync::atomic::{AtomicI64, Ordering};
use crate::error::Error;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Copy, Clone)]
pub struct Id {
    pub seq_id: i64,
    pub id: i64,
}

impl Id {
    pub fn new(seq_id: i64, id: i64) -> Self {
        Id {
            seq_id: seq_id,
            id: id,
        }
    }

    pub fn append(&self, path: &Path, buffer: &mut Buffer) -> Result<(), Error> {
        buffer.set::<i64>(&path.with(path!["seq_id"]), self.seq_id)?;
        buffer.set::<i64>(&path.with(path!["id"]), self.id)?;
        Ok(())
    }

    pub fn from(path: &Path, buffer: &ReadOnlyBuffer) -> Result<Self, Error> {
        Ok(Id {
            seq_id: buffer.get(&path.with(path!["seq_id"]))?,
            id: buffer.get(&path.with(path!["id"]))?,
        })
    }

    pub fn from_buffer(path: &Path, buffer: &Buffer ) -> Result<Self, Error> {
        Ok(Id {
            seq_id: buffer.get(&path.with(path!["seq_id"]))?,
            id: buffer.get(&path.with(path!["id"]))?,
        })
    }
}

pub struct IdSeq {
    seq_id: i64,
    seq: AtomicI64,
}

impl IdSeq {
    pub fn new(seq_id: i64) -> Self {
        IdSeq {
            seq_id: seq_id,
            seq: AtomicI64::new(0),
        }
    }

    pub fn seq_id(&self) -> i64 {
        self.seq_id
    }

    pub fn next(&self) -> Id {
        Id {
            seq_id: self.seq_id,
            id: self.seq.fetch_add(1, Ordering::Relaxed),
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
pub enum NucleusKind {
    Source,
    Replica(Id),
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
pub struct NucleusKey {
    id: Id,
    kind: NucleusKind,
}

#[derive(Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
pub struct MechtronKey {
    pub nucleus: Id,
    pub mechtron: Id,
}

impl MechtronKey {
    pub fn new(nucleus_id: Id, tron_id: Id) -> MechtronKey {
        MechtronKey {
            nucleus: nucleus_id,
            mechtron: tron_id,
        }
    }

    pub fn append(&self, path: &Path, buffer: &mut Buffer) -> Result<(), Error> {
        self.nucleus.append(&path.push(path!["nucleus"]), buffer)?;
        self.mechtron.append(&path.push(path!["tron"]), buffer)?;
        Ok(())
    }

    pub fn from(path: &Path, buffer: &ReadOnlyBuffer) -> Result<Self, Error> {
        Ok(MechtronKey {
            nucleus: Id::from(&path.push(path!["nucleus"]), buffer)?,
            mechtron: Id::from(&path.push(path!["tron"]), buffer)?,
        })
    }

}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Copy, Clone)]
pub struct StateKey {
    pub tron: MechtronKey,
    pub revision: Revision,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Copy, Clone)]
pub struct Revision {
    pub cycle: i64,
}

pub struct DeliveryMomentKey {
    pub cycle: i64,
    pub phase: u8,
}

use no_proto::buffer::NP_Buffer;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock, PoisonError, RwLockWriteGuard, RwLockReadGuard};
use std::error::Error;
use no_proto::memory::NP_Memory_Owned;
use mechtron_common::id::{RevisionKey, TronKey, Revision, ContentKey, Id};
use mechtron_common::content::{Content, ReadOnlyContent};
use mechtron_common::configs::Configs;
use std::collections::hash_map::RandomState;


pub struct NucleusContentStructure {
    history: HashMap<Revision,HashMap<TronKey,ReadOnlyContent>>
}


impl NucleusContentStructure
{
   pub fn new() -> Self{
       NucleusContentStructure {
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

   pub fn get(&self, key:&ContentKey) -> Result<&ReadOnlyContent,Box<dyn Error>>
   {
       let history = self.unwrap(self.history.get(&key.revision ))?;
       let content = self.unwrap( history.get(&key.tron_id))?;
       Ok(content)
   }

   pub fn query( &self, revision: &Revision ) -> Option<Vec<(TronKey,&ReadOnlyContent)>>
   {
       let history = self.history.get(&revision );
       match history
       {
           None => Option::None,
           Some(history) => {
               let mut rtn = vec!();
               for key in history.keys()
               {
                   let content = history.get(key).unwrap();
                   rtn.push( (key,value) )
               }
               Ok(rtn)
           }
       }
   }

}

impl ContentIntake for NucleusContentStructure
{
    fn intake(&mut self,content: Content, key: ContentKey) -> Result<(), Box<dyn Error+'_>> {
        let content = Content::read_only(content)?;
        let history = self.history.entry(key.revision.clone()).or_insert(HashMap::new());
        history.insert(key.tron_id.clone(), content );
        Ok(())
    }

}


pub trait ContentIntake
{
    fn intake( &mut self, content: Content, key: ContentKey )->Result<(),Box<dyn Error+'_>>;
}

pub trait ReadOnlyContentIntake
{
    fn intake( &mut self, content: ReadOnlyContent, key: ContentKey )->Result<(),Box<dyn Error+'_>>;
}

pub trait ReadOnlyContentAccess
{
    fn get( &self, key: &ContentKey )->Result<&ReadOnlyContent,Box<dyn Error+'_>>;
}

pub trait ContentAccess
{
    fn get( &mut self, key: &ContentKey )->Result<&Content,Box<dyn Error+'_>>;
}

pub struct NucleusPhasicContentStructure
{
    store: HashMap<TronKey,Content>
}

impl NucleusPhasicContentStructure
{
    pub fn new( ) -> Self {
        NucleusPhasicContentStructure{
            store: HashMap::new()
        }
    }

    pub fn intake( &mut self, key: TronKey, content: &ReadOnlyContent) -> Result<(),Box<dyn Error>>
    {
       self.store.insert(key, content.copy()? );
       Ok(())
    }

    pub fn get(&mut self, key: &TronKey )->Result<&mut Content,Box<dyn Error>>
    {
        self.store.get_mut(key).ok_or("could not find content".into() )
    }

}

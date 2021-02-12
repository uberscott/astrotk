use no_proto::buffer::NP_Buffer;
use std::collections::HashMap;
use std::sync::{Arc, RwLock, PoisonError, RwLockWriteGuard, RwLockReadGuard};
use std::error::Error;
use no_proto::memory::NP_Memory_Owned;
use mechtron_common::id::{RevisionKey, TronKey, Revision, ContentKey};
use mechtron_common::content::{Content, ReadOnlyContent};
use mechtron_common::configs::Configs;


pub struct ContentStructure {
    history: RwLock<HashMap<TronKey,RwLock<ContentHistory>>>
}


impl ContentStructure
{
   pub fn new() -> Self{
       ContentStructure {
           history: RwLock::new(HashMap::new())
       }
   }

   fn get(&self, key:&ContentKey) -> Result<&Content,Box<dyn Error>>
   {
       let history = self.history.read()?;
       if !history.contains_key(&revision.content_key )
       {
           return Err(format!("content history for key {:?} is not managed by this store",revision.content_key).into());
       }
       let history = history.get( &key.tron_id ).unwrap().read()?;
       return history.get(key);
   }
}

impl ContentIntake for ContentStructure
{
    fn intake(&mut self,content: Content, key: ContentKey) -> Result<(), Box<dyn Error+'_>> {
        let history = self.history.read()?;
        if !history.contains_key(&key.content_id )
        {
            return Err(format!("content history for key {:?} is not managed by this store",content.revision.content_key).into());
        }

        let history = history.get(&content.revision.content_key).unwrap();
        let result = history.write();
        match result {
            Ok(_) => {}
            Err(_) => return Err("could not acquire history lock".into())
        }
        let mut history = result.unwrap();
        let result = history.intake(content);
        match result {
            Ok(_) => Ok(()),
            Err(e) => return Err("could not intake history".into())
        }
    }
}

impl ContentRetrieval for ContentStructure {

    fn read_only(&self, key: &ContentKey, configs: &Configs ) -> Result<ReadOnlyContent, Box<dyn Error>> {
        let rtn = self.get(key)?.read_only(configs)?;
        Ok(rtn)
    }

    fn copy(&self, revision: &ContentKey) -> Result<Content, Box<dyn Error+'_>> {

        let rtn = self.get(key)?;
        Ok(rtn.clone())
    }
}

pub trait ContentIntake
{
    fn intake( &mut self, content: Content, key: ContentKey )->Result<(),Box<dyn Error+'_>>;
}

pub trait ContentAccess
{
    fn get( &self, key: &ContentKey )->Result<ReadOnlyContent,Box<dyn Error+'_>>;
}

pub trait ContentRetrieval
{
    fn read_only( &self, key: &ContentKey, configs: &Configs )->Result<ReadOnlyContent,Box<dyn Error+'_>>;
    fn copy( &self, key: &ContentKey )->Result<Content,Box<dyn Error+'_>>;
}


pub struct ContentHistory
{
    key: TronKey,
    content: HashMap<i64,Content>
}

impl ContentHistory {
    fn new(key: TronKey) ->Self{
        ContentHistory{
            key: key,
            content: HashMap::new()
        }
    }

    fn get(&self, key: &ContentKey )->Result<&Content,Box<dyn Error>>
    {
        match self.content.get(&key.revision.cycle )
        {
            None => Err(format!("could not find history for cycle {}", key.revision.cycle ).into()),
            Some(content) => Ok(content)
        }
    }
}

impl ContentIntake for ContentHistory
{
    fn intake(&mut self, content: Content,  key:ContentKey ) -> Result<(), Box<dyn Error>> {
       self.content.insert(key.revision.cycle.clone(),content );
       Ok(())
    }
}


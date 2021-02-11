use no_proto::buffer::NP_Buffer;
use std::collections::HashMap;
use std::sync::{Arc, RwLock, PoisonError, RwLockWriteGuard, RwLockReadGuard};
use std::error::Error;
use no_proto::memory::NP_Memory_Owned;
use mechtron_common::id::{RevisionKey, TronKey, Revision, ContentKey};
use mechtron_common::content::Content;




pub struct ContentStore{
    history: RwLock<HashMap<TronKey,RwLock<ContentHistory>>>
}


impl ContentStore
{
   pub fn new() -> Self{
       ContentStore{
           history: RwLock::new(HashMap::new())
       }
   }

   pub fn create(&mut self, key: &TronKey) ->Result<(),Box<dyn Error+'_>>
   {
       let mut map = self.history.write()?;
       if map.contains_key(key )
       {
           return Err(format!("content history for key {:?} has already been created for this store",key).into());
       }

       let history = RwLock::new(ContentHistory::new(key.clone() ) );
       map.insert(key.clone(), history );

       Ok(())
   }
}

impl ContentIntake for ContentStore
{
    fn intake(&mut self,content: Content) -> Result<(), Box<dyn Error+'_>> {
        let history = self.history.read()?;
        if !history.contains_key(&content.revision.content_key )
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

impl ContentRetrieval for ContentStore{

    fn retrieve(&self, revision: &ContentKey) -> Result<Content, Box<dyn Error+'_>> {
        let history = self.history.read()?;
        if !history.contains_key(&revision.content_key )
        {
            return Err(format!("content history for key {:?} is not managed by this store",revision.content_key).into());
        }

        let history = history.get(&revision.content_key).unwrap();
        let result = history.read();
        match result{
            Ok(_) => {}
            Err(_) => return Err("could not acquire read lock for ContentRetrieval".into())
        }
        let guard = result.unwrap();

        let result = guard.retrieve(revision);
        match result{
            Ok(_) => {}
            Err(_) => return Err(format!("could not acquire retrieve revision: {:?}",revision).into())
        }
        let content = result.unwrap();
        Ok(content)
    }
}

pub trait ContentIntake
{
    fn intake( &mut self, content: Content )->Result<(),Box<dyn Error+'_>>;
}

pub trait ContentRetrieval
{
    fn retrieve( &self, revision: &RevisionKey  )->Result<Content,Box<dyn Error+'_>>;
}


pub struct ContentHistory
{
    key: TronKey,
    content: HashMap<ContentKey,Content>,
}

impl ContentHistory {
    fn new(key: TronKey) ->Self{
        ContentHistory{
            key: key,
            content: HashMap::new()
        }
    }
}

impl ContentIntake for ContentHistory
{
    fn intake(&mut self, content: Content) -> Result<(), Box<dyn Error>> {

        if self.buffers.contains_key(&content.revision)
        {
            return Err(format!("history content for revision {:?} already exists.", content.revision).into());
        }

        self.buffers.insert( content.revision, content.data);

        Ok(())
    }
}

impl  ContentRetrieval for ContentHistory {
    fn retrieve(&self, revision: &RevisionKey) -> Result<Content, Box<dyn Error>> {
        if !self.buffers.contains_key(revision)
        {
            return Err(format!("history does not have content for revision {:?}.", revision).into());
        }

        let buffer= self.buffers.get( revision ).unwrap();

        let content= Content::from_arc(revision.clone(), buffer.clone());
        Ok(content)
    }
}
use no_proto::buffer::NP_Buffer;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::error::Error;

struct ContentStore<'buffer>{
    history: RwLock<HashMap<ContentKey,RwLock<ContentHistory<'buffer>>>>
}

impl ContentStore
{
   fn create_tron( &mut self, key: &ContentKey )->Result<(),Box<dyn Error>>
   {
       let mut map = self.history.write()?;
       if map.contains_key(key )
       {
           return Err(format!("content history for key {:?} has already been created for this store",content.revision.content_key).into());
       }

       let history = RwLock::new(ContentHistory::new(key.clone() ) );
       map.insert(key.clone(), history );

       Ok(())
   }
}

impl <'buffer> ContentIntake for ContentStore<'buffer>
{
    fn intake(&mut self,content: Content<'buffer>) -> Result<(), Box<dyn Error>> {
        let history = self.history.read()?;
        if !history.contains_key(&content.revision.content_key )
        {
            return Err(format!("content history for key {:?} is not managed by this store",content.revision.content_key).into());
        }

        let history = history.get(&content.revision.content_key).unwrap();
        let mut history = history.write()?;
        history.intake(content)?;

        Ok(())
    }
}

impl ContentRetrieval for ContentStore{

    fn retrieve(&self, revision: &RevisionKey) -> Result<Content<'_>, Box<dyn Error>> {
        let history = self.history.read()?;
        if !history.contains_key(&content.revision.content_key )
        {
            return Err(format!("content history for key {:?} is not managed by this store",content.revision.content_key).into());
        }

        let history = history.get(&content.revision.content_key).unwrap();
        let history = history.read()?;
        let content = history.retrieve(revision)?;
        Ok(content)
    }
}

pub trait ContentIntake<'buffer>
{
    fn intake( &mut self, content: Content<'buffer> )->Result<(),Box<dyn Error>>;
}

pub trait ContentRetrieval<'buffer>
{
    fn retrieve( &self, revision: &RevisionKey  )->Result<Content,Box<dyn Error>>;
}

#[derive(PartialEq,Eq,PartialOrd,Ord,Hash,Debug,Clone)]
pub struct ContentKey
{
    nucleus_id: i64,
    tron_id: i64
}

#[derive(PartialEq,Eq,PartialOrd,Ord,Hash,Debug,Clone)]
pub struct RevisionKey
{
    content_key: ContentKey,
    cycle: i64
}

pub struct Content<'buffer>
{
    revision: RevisionKey,
    buffer: Arc<NP_Buffer<'buffer>>
}

impl <'buffer> Content<'buffer>
{
    fn new( revision: RevisionKey, buffer: NP_Buffer<'buffer> )->Self
    {
       Content{
           revision:revision,
           buffer:Arc::new(buffer)
       }
    }

    fn from_arc( revision: RevisionKey, buffer: Arc<NP_Buffer<'buffer>> )->Self
    {
        Content{
            revision:revision,
            buffer:buffer
        }
    }
}

pub struct ContentHistory<'buffer>
{
    key: ContentKey,
    buffers: HashMap<RevisionKey,Arc<NP_Buffer<'buffer>>>,
}

impl ContentHistory {
    fn new( key: ContentKey )->Self{
        ContentHistory{
            key: key,
            buffers: HashMap::new()
        }
    }
}

impl <'buffer> ContentIntake for ContentHistory<'buffer>
{
    fn intake(&mut self, content: Content<'buffer>) -> Result<(), Box<dyn Error>> {

        if self.buffers.contains_key(&content.revision)
        {
            return Err(format!("history content for revision {:?} already exists.", content.revision).into());
        }

        self.buffers.insert( content.revision, content.buffer );

        Ok(())
    }
}

impl <'buffer> ContentRetrieval for ContentHistory<'buffer> {
    fn retrieve(&self, revision: &RevisionKey) -> Result<Content, Box<dyn Error>> {
        if !self.buffers.contains_key(revision)
        {
            return Err(format!("history does not have content for revision {:?}.", revision).into());
        }

        let buffer = self.buffers.get( revision ).unwrap();

        Ok(Content::from_arc(revision.clone(), buffer.clone()))
    }
}
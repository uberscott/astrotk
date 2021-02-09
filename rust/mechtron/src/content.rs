use no_proto::buffer::NP_Buffer;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::error::Error;

pub struct ContentStore<'buffer>{
    history: RwLock<HashMap<TronKey,RwLock<ContentHistory<'buffer>>>>
}

impl <'buffer> ContentStore<'buffer>
{
   pub fn new() -> Self{
       ContentStore{
           history: RwLock::new(HashMap::new())
       }
   }

   pub fn create(&mut self, key: &TronKey) ->Result<(),Box<dyn Error>>
   {
       let mut map = self.history.write()?;
       if map.contains_key(key )
       {
           return Err(format!("content history for key {:?} has already been created for this store",key.revision.content_key).into());
       }

       let history = RwLock::new(ContentHistory::new(key.clone() ) );
       map.insert(key.clone(), history );

       Ok(())
   }
}

impl <'buffer> ContentIntake<'buffer> for ContentStore<'buffer>
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

impl <'buffer> ContentRetrieval<'buffer> for ContentStore<'buffer>{

    fn retrieve(&self, revision: &RevisionKey) -> Result<Content<'_>, Box<dyn Error>> {
        let history = self.history.read()?;
        if !history.contains_key(&revision.content_key )
        {
            return Err(format!("content history for key {:?} is not managed by this store",revision.content_key).into());
        }

        let history = history.get(&revision.content_key).unwrap();
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
pub struct TronKey
{
    nucleus_id: i64,
    tron_id: i64
}

impl TronKey
{
    pub fn new( nucleus_id: i64, tron_id : i64 ) -> Self {
        TronKey{ nucleus_id: nucleus_id,
                 tron_id: tron_id }
    }
}

#[derive(PartialEq,Eq,PartialOrd,Ord,Hash,Debug,Clone)]
pub struct RevisionKey
{
    content_key: TronKey,
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
    key: TronKey,
    buffers: HashMap<RevisionKey,Arc<NP_Buffer<'buffer>>>,
}

impl <'buffer> ContentHistory<'buffer> {
    fn new(key: TronKey) ->Self{
        ContentHistory{
            key: key,
            buffers: HashMap::new()
        }
    }
}

impl <'buffer> ContentIntake<'buffer> for ContentHistory<'buffer>
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

impl <'buffer> ContentRetrieval<'buffer> for ContentHistory<'buffer> {
    fn retrieve(&self, revision: &RevisionKey) -> Result<Content, Box<dyn Error>> {
        if !self.buffers.contains_key(revision)
        {
            return Err(format!("history does not have content for revision {:?}.", revision).into());
        }

        let buffer = self.buffers.get( revision ).unwrap();

        Ok(Content::from_arc(revision.clone(), buffer.clone()))
    }
}
use no_proto::buffer::NP_Buffer;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock, PoisonError, RwLockWriteGuard, RwLockReadGuard};
use std::error::Error;
use no_proto::memory::NP_Memory_Owned;
use mechtron_common::id::{RevisionKey, TronKey, Revision, ContentKey, Id};
use mechtron_common::content::{Content, ReadOnlyContent};
use mechtron_common::configs::Configs;


pub struct InterCyclicContentStructure {
    history: RwLock<HashMap<Id,HashMap<TronKey,RwLock<ContentHistory>>>>
}


impl InterCyclicContentStructure
{
   pub fn new() -> Self{
       InterCyclicContentStructure {
           history: RwLock::new(HashMap::new())
       }
   }

   fn get(&self, key:&ContentKey) -> Result<&ReadOnlyContent,Box<dyn Error>>
   {
       let history = self.history.read()?;
       if !history.contains_key(&key.tron_id.nucleus_id )
       {
           return Err(format!("content history for key {:?} is not managed by this store",revision.content_key).into());
       }
       let history = history.get( &key.tron_id.nucleus_id ).unwrap();

       if !history.contains_key(&key.tron_id )
       {
           return Err(format!("content history for key {:?} is not managed by this store",revision.content_key).into());
       }

       let history = history.get(&key.tron_id ).unwrap();
       let history = history.read()?;

       return history.get(&key.revision);
   }

   fn on_each_history( &self, func: fn( history: &ContentHistory ))->Result<(),Box<dyn Error>>
   {
       let history = self.history.read()?;
       for nucleus_id in history.keys() {
           let history = history.get(&nucleus_id ).unwrap();

           for key in history.keys()
           {
               let history = history.get(key).unwrap().read()?;
               func(&history)?;
           }
       }

       Ok(())
   }

   // query for all nucleus in given revision
   pub fn query_nuclei(&self, revision: &Revision ) ->Result<HashSet<Id>,Box<dyn Error>>
   {
       let mut rtn = HashSet::new();
       let history = self.history.read()?;
       for nucleus_id in history.keys() {
           let history = history.get(&nucleus_id ).unwrap();

           for key in history.keys()
           {
               let history = history.get(key).unwrap().read()?;
               if history.contains(&revision )
               {
                   rtn.insert(nucleus_id.clone());
                   break;
               }
           }
       }

       Ok(rtn)
   }

    pub fn query_nucleus_content( &self, nucleus_id: &Id, revision: &Revision, configs: &Configs )-> Result<Vec<(ReadOnlyContent,ContentKey)>,Box<dyn Error>>
    {
        let mut rtn = vec!();

        let history = self.history.read()?;

        if history.contains_key(nucleus_id)
        {
            for nucleus_id in history.keys()
            {
                let history = history.get(nucleus_id).unwrap();
                for tron_key in  history.keys()
                {
                    let history = history.get(tron_key).unwrap().read()?;
                    if( history.contains(revision) )
                    {
                        let content = history.get(revision)?;
                        rtn.push((content.read_only(configs)?,ContentKey{tron_id:tron_key.clone(),revision:revision.clone()}) )
                    }
                }
            }
        }

        Ok(rtn)
    }
}

impl ContentIntake for InterCyclicContentStructure
{
    fn intake(&mut self,content: Content, key: ContentKey) -> Result<(), Box<dyn Error+'_>> {
        let mut history = self.history.write()?;
        if !history.contains_key(&key.tron_id.nucleus_id )
        {
            history.insert( key.tron_id.nucleus_id.clone(), HashMap::new() )
        }
        let mut history = history.get(&key.tron_id.nucleus_id ).unwrap();

        if !history.contains_key(&key.tron_id )
        {
            history.insert( key.tron_id.clone(), RwLock::new(ContentHistory::new(key.tron_id.clone() ) ) )
        }

        let mut history = history.get( &key.tron_id ).unwrap();
        let mut history = history.write()?;

        history.intake(content,key)?;

        Ok(())
    }
}

impl ContentRetrieval for InterCyclicContentStructure {

    fn read_only(&self, key: &ContentKey, configs: &Configs ) -> Result<&ReadOnlyContent, Box<dyn Error>> {
        let rtn = self.get(key)?;
        Ok(rtn)
    }

    fn copy(&self, revision: &ContentKey) -> Result<Content, Box<dyn Error+'_>> {

        let rtn = self.get(key)?;
        let rtn = rtn.copy()?;
        Ok(rtn)
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


pub trait ContentRetrieval
{
    fn read_only( &self, key: &ContentKey, configs: &Configs )->Result<&ReadOnlyContent,Box<dyn Error+'_>>;
    fn copy( &self, key: &ContentKey )->Result<Content,Box<dyn Error+'_>>;
}


pub struct ContentHistory
{
    key: TronKey,
    content: HashMap<i64,ReadOnlyContent>
}

impl ContentHistory {
    pub fn new(key: TronKey) ->Self{
        ContentHistory{
            key: key,
            content: HashMap::new()
        }
    }

    pub fn get(&self, revision: &Revision )->Result<&ReadOnlyContent,Box<dyn Error>>
    {
        match self.content.get(&revision.cycle )
        {
            None => Err(format!("could not find history for cycle {}", key.revision.cycle ).into()),
            Some(content) => Ok(content)
        }
    }

    pub fn contains( &self, revision: &Revision  )->bool
    {
        return self.content.contains_key(&revision.cycle);
    }

}

impl ContentIntake for ContentHistory
{
    fn intake(&mut self, content: Content,  key:ContentKey ) -> Result<(), Box<dyn Error>> {
       self.content.insert(key.revision.cycle.clone(),content.read_only()? );
       Ok(())
    }
}

pub struct IntraCyclicContentStructure
{
    revision: Revision,
    store: HashMap<Id,Content>,
    read_only_store: HashMap<Id,ReadOnlyContent>,
}

impl IntraCyclicContentStructure
{
    pub fn intake( &mut self, content: ReadOnlyContent, key: ContentKey )
    {
        self.read_only_store.insert(key.tron_id.tron_id, content );
    }

    pub fn new( revision: Revision )->Self
    {
        IntraCyclicContentStructure{
            revision: revision,
            store: HashMap::new(),
            read_only_store: HashMap::new()
        }
    }


}

impl ReadOnlyContentIntake for IntraCyclicContentStructure
{
    fn intake(&mut self, content: ReadOnlyContent, key: ContentKey) -> Result<(), Box<dyn Error>> {
        self.read_only_store.insert(key.tron_id.tron_id.clone(), content );
        Ok(())
    }
}

impl ContentAccess for IntraCyclicContentStructure
{
    fn get( &mut self, key: ContentKey )->Result<&Content,Box<dyn Error>>
    {
        if !self.store.contains_key(&key.tron_id.tron_id)
        {
            if( !self.read_only_store.contains_key(&key.tron_id.tron_id) )
            {
                return Err("could not find expected content".into());
            }
            let content = self.read_only_store.get(&key.tron_id.tron_id).unwrap();
            let content = content.copy( )?;
            self.store.insert( key.tron_id.tron_id.clone(), content );
        }

        Ok(self.store.get(*key.tron_id.tron_id ))
    }
}

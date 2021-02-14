#![macro_use]

use no_proto::buffer::NP_Buffer;
use no_proto::NP_Factory;

use crate::artifact::Artifact;
use std::error::Error;
use no_proto::memory::{NP_Memory_Owned, NP_Memory, NP_Mem_New};
use no_proto::pointer::{NP_Scalar, NP_Value};
use std::sync::Arc;
use no_proto::error::NP_Error;
use std::iter::FromIterator;


#[macro_export]
macro_rules! path{
    ($($x:expr),*) => (vec![$($x.to_string()),*]);
}

pub trait BufferFactories
{
    fn create_buffer(&self, artifact: &Artifact) ->Result<NP_Buffer<NP_Memory_Owned>,Box<dyn Error>>;
    fn create_buffer_from_array(&self, artifact: &Artifact, array: Vec<u8> ) ->Result<NP_Buffer<NP_Memory_Owned>,Box<dyn Error>>;
    fn create_buffer_from_buffer(&self, artifact: &Artifact, buffer: NP_Buffer<NP_Memory_Owned> ) ->Result<NP_Buffer<NP_Memory_Owned>,Box<dyn Error>>;
    fn get_buffer_factory(&self, artifact: &Artifact) ->Option<&'static NP_Factory<'static>>;
}




fn cat( path: &[&str])->String
{
    let mut rtn = String::new();
    for segment in path{
        rtn.push_str(segment);
        rtn.push('/');
    }
    return rtn;
}

#[derive(Clone)]
pub struct Buffer
{
    np_buffer: NP_Buffer<NP_Memory_Owned>
}

impl Buffer
{
    pub fn new( np_buffer: NP_Buffer<NP_Memory_Owned> )->Self
    {
        Buffer{
            np_buffer: np_buffer
        }
    }

    pub fn is_set<'get, X: 'get>( &'get self, path: &Vec<String>) -> Result<bool, Box<dyn Error>> where X: NP_Value<'get> + NP_Scalar<'get> {
        let path  = Vec::from_iter(path.iter().map(String::as_str));
        let path = path.as_slice();
        match self.np_buffer.get::<X>(path)
        {
            Ok(option)=>{
                Ok(option.is_some())
            },
            Err(e)=>Err(format!("could not get {}",cat(path)).into())
        }
    }


    pub fn get<'get, X: 'get>( &'get self, path: &Vec<String>) -> Result<X, Box<dyn Error>> where X: NP_Value<'get> + NP_Scalar<'get> {

        let path  = Vec::from_iter(path.iter().map(String::as_str));
        let path = path.as_slice();
        match self.np_buffer.get::<X>(path)
        {
            Ok(option)=>{
                match option{
                    Some(rtn)=>Ok(rtn),
                    None=>Err(format!("expected a value for {}", path[path.len()-1] ).into())
                }
            },
            Err(e)=>Err(format!("could not get {}",cat(path)).into())
        }
    }

    pub fn set<'set, X: 'set>(&'set mut self, path: &Vec<String>, value: X) -> Result<bool, Box<dyn Error>> where X: NP_Value<'set> + NP_Scalar<'set> {

        let path  = Vec::from_iter(path.iter().map(String::as_str));
        let path = path.as_slice();
        match self.np_buffer.set::<X>(path, value)
        {
            Ok(option)=>{
               Ok(option)
            },
            Err(e)=>Err(format!("could not set {}",cat(path)).into())
        }
    }

    pub fn compact( &mut self )->Result<(),Box<dyn Error>>
    {
        match self.np_buffer.compact(Option::None)
        {
            Ok(_) => Ok(()),
            Err(e) => Err("could not compact".into() )
        }
    }

    pub fn read_only(buffer: Buffer ) ->RO_Buffer
    {
        RO_Buffer {
            np_buffer: buffer.np_buffer
        }
    }

    pub fn read_bytes(&mut self)->&[u8]
    {
        return self.np_buffer.read_bytes();
    }

    pub fn bytes(buffer:Buffer)->Vec<u8>
    {
        buffer.np_buffer.finish().bytes()
    }


}


#[derive(Clone)]
pub struct RO_Buffer
{
    np_buffer: NP_Buffer<NP_Memory_Owned>
}

impl RO_Buffer
{

    pub fn new( np_buffer: NP_Buffer<NP_Memory_Owned> )->Self
    {
        RO_Buffer{
            np_buffer: np_buffer
        }
    }
    pub fn is_set<'get, X: 'get>( &'get self, path: &Vec<String>) -> Result<bool, Box<dyn Error>> where X: NP_Value<'get> + NP_Scalar<'get> {
        let path  = Vec::from_iter(path.iter().map(String::as_str));
        let path = path.as_slice();
        match self.np_buffer.get::<X>(path)
        {
            Ok(option)=>{
                Ok(option.is_some())
            },
            Err(e)=>Err(format!("could not get {}",cat(path)).into())
        }
    }

    pub fn get<'get, X: 'get>( &'get self, path: &Vec<String>) -> Result<X, Box<dyn Error>> where X: NP_Value<'get> + NP_Scalar<'get> {
        let path  = Vec::from_iter(path.iter().map(String::as_str));
        let path = path.as_slice();
        match self.np_buffer.get::<X>(path)
        {
            Ok(option)=>{
                match option{
                    Some(rtn)=>Ok(rtn),
                    None=>Err(format!("expected a value for {}", path[path.len()-1] ).into())
                }
            },
            Err(e)=>Err(format!("could not get {}",cat(path)).into())
        }
    }

    pub fn clone_to_buffer( &self )->Result<Buffer,Box<dyn Error>>
    {
        Ok(Buffer {
            np_buffer: self.np_buffer.clone()
        })
    }

    pub fn read_bytes(&self)->&[u8]
    {
        return self.np_buffer.read_bytes();
    }


    pub fn bytes(buffer: RO_Buffer)->Vec<u8>
    {
        buffer.np_buffer.finish().bytes()
    }

    pub fn size(&self)->usize
    {
        self.np_buffer.data_length()
    }
}

pub struct Path
{
    segments: Vec<String>
}

impl Path
{
    pub fn new( segments: Vec<String> )->Self
    {
        Path{
            segments: segments
        }
    }

    pub fn push( &self, more: Vec<String> )->Self
    {
        let mut segment = self.segments.clone();
        let mut more = more;
        segment.append(&mut more);
        Path{
            segments: segment
        }
    }

    pub fn with( & self, more: Vec<String> )->Vec<String>
    {
        let mut segment = self.segments.clone();
        let mut more = more;
        segment.append(&mut more);
        return segment;
    }
}

#[cfg(test)]
mod tests {
    use no_proto::NP_Factory;
    use crate::buffers::{Buffer, Path};

    #[test]
    fn test_example() {
        let factory: NP_Factory = NP_Factory::new(r#"list({of: map({ value: list({ of: string() })})})"#).unwrap();

        let mut new_buffer = factory.new_buffer(None);
// third item in the top level list -> key "alpha" of map at 3rd element -> 9th element of list at "alpha" key
//
        new_buffer.set(&["3", "alpha", "9"], "look at all this nesting madness");

// get the same item we just set
        let message = new_buffer.get::<&str>(&["3", "alpha", "9"]).unwrap();

        assert_eq!( message, Option::Some("look at all this nesting madness"));
    }

    #[test]
    fn check_schema() {
     let schema=   r#"struct({fields: {
                         userId: string(),
                         password: string(),
                         email: string(),
                         age: u8()
}})"#;
        let np_factory = NP_Factory::new( schema ).unwrap();
        let mut np_buffer = np_factory.new_buffer(Option::None);
        assert_eq!( true, np_buffer.set( &["userId"], "Henry").unwrap() );
        assert_eq!( true, np_buffer.set::<u8>( &["age"], 27).unwrap());

        assert_eq!("Henry", np_buffer.get::<String>(&["userId"]).unwrap().unwrap());
        assert_eq!(27, np_buffer.get::<u8>(&["age"]).unwrap().unwrap());
        assert_eq!(Option::None, np_buffer.get::<String>(&["password"]).unwrap());
        assert!(np_buffer.get::<u8>(&["junk"]).unwrap().is_none());
    }


    #[test]
    fn check_buffer() {
        let schema=   r#"struct({fields: {
                         userId: string(),
                         password: string(),
                         email: string(),
                         age: u8()
}})"#;
        let np_factory = NP_Factory::new( schema ).unwrap();
        let mut np_buffer = np_factory.new_buffer(Option::None);
        let mut buffer = Buffer::new(np_buffer);

        assert_eq!( true, buffer.set( &path!("userId"), "Henry").unwrap() );
        assert_eq!( true, buffer.set::<u8>( &path!("age"), 27).unwrap());

        assert_eq!("Henry", buffer.get::<String>(&path!("userId")).unwrap());
        assert_eq!(27, buffer.get::<u8>(&path!("age")).unwrap());
        assert!(buffer.get::<String>(&path!("password")).is_err());
        assert!(buffer.get::<String>(&path!("junk")).is_err());
        assert_eq!(false, buffer.is_set::<String>(&path!["password"]).unwrap());
        assert_eq!(true, buffer.is_set::<String>(&path!["userId"]).unwrap());
    }


    #[test]
    fn check_ro_buffer() {
        let schema=   r#"struct({fields: {
                         userId: string(),
                         password: string(),
                         email: string(),
                         age: u8()
}})"#;
        let np_factory = NP_Factory::new( schema ).unwrap();
        let mut np_buffer = np_factory.new_buffer(Option::None);
        let mut buffer = Buffer::new(np_buffer);

        assert_eq!( true, buffer.set( &path!("userId"), "Henry").unwrap() );
        assert_eq!( true, buffer.set::<u8>( &path!("age"), 27).unwrap());

        let buffer = Buffer::read_only(buffer);

        assert_eq!("Henry", buffer.get::<String>(&path!("userId")).unwrap());
        assert_eq!(27, buffer.get::<u8>(&path!("age")).unwrap());
        assert!(buffer.get::<String>(&path!("password")).is_err());
        assert!(buffer.get::<String>(&path!("junk")).is_err());
        assert_eq!(false, buffer.is_set::<String>(&path!("password")).unwrap());
        assert_eq!(true, buffer.is_set::<String>(&path!("userId")).unwrap());
    }


    #[test]
    fn check_how_lists_work1() {

        let factory: NP_Factory = NP_Factory::new(r#"list({of: string() })"#).unwrap();
        let mut buffer = factory.new_buffer(Option::None);

        assert_eq!(Option::Some(0),buffer.list_push(&[], "hi" ).unwrap());
    }


    #[test]
    fn check_how_maps_work() {

        let factory: NP_Factory = NP_Factory::new(r#"map({ value: string() })"#).unwrap();
        let mut buffer = factory.new_buffer(Option::None);
        assert!(buffer.set(&["blah"], "hi" ).unwrap());
    }

    #[test]
    fn check_nested_example(){
        let factory: NP_Factory = NP_Factory::new(r#"list({of: map({ value: list({ of: string() })})})"#).unwrap();

        let mut new_buffer = factory.new_buffer(None);
// third item in the top level list -> key "alpha" of map at 3rd element -> 9th element of list at "alpha" key
//
        new_buffer.set(&["3", "alpha", "9"], "look at all this nesting madness").unwrap();

// get the same item we just set
        let message = new_buffer.get::<&str>(&["3", "alpha", "9"]).unwrap();

        assert_eq!(message, Some("look at all this nesting madness"))
    }

    #[test]
    fn check_path(){
        let factory: NP_Factory = NP_Factory::new(r#"list({of: map({ value: list({ of: string() })})})"#).unwrap();
        let mut new_buffer = factory.new_buffer(None);

        let path= Path::new(path!("0") );
        assert_eq!( &["0","alpha","9"], path.with(path!("alpha", "9")).as_slice());
        let deep_path = path.push(path!("alpha", "9"));
        assert_eq!( path!("0","alpha","9"), deep_path.segments);

        /*new_buffer.set(path.with(&["alpha", "9"]).as_slice(), "look at all this nesting madness").unwrap();
        let message = new_buffer.get::<&str>(&deep_path.segments).unwrap();

        assert_eq!(message, Some("look at all this nesting madness"))*/
    }
}
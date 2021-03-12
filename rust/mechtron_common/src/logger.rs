use std::sync::RwLock;
use std::cell::{Cell, RefCell};
use std::sync::atomic::{AtomicPtr, Ordering};
lazy_static! {
 pub static ref LOGGER: RwLock<Logger> = RwLock::new(Logger::new());
}

pub fn log( message: &str )
{
    let log = LOGGER.read().unwrap();
    log.log(message);
}

pub fn replace_logger( appender: Box<dyn Appender>)
{
    let mut log = LOGGER.write().unwrap();
    log.appender = appender;
}



pub struct Logger
{
    appender: Box<dyn Appender>
}

impl Logger
{
    pub fn new()->Self
    {
        Logger{
            appender: Box::new(StdOutAppender{})
        }
    }

    pub fn log( &self, message: &str )
    {
        self.appender.log(message);
    }
}

pub trait Appender: Sync+Send
{
    fn log( &self, message: &str );
}

pub struct StdOutAppender
{

}

impl StdOutAppender
{
    pub fn new()->Self
    {
        StdOutAppender{}
    }
}

impl Appender for StdOutAppender
{

    fn log(&self, message: &str) {
        println!("{}",message)
    }
}

unsafe impl Send for StdOutAppender
{

}
unsafe impl Sync for StdOutAppender
{

}


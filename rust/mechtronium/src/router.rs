use crate::mechtronium::Mechtronium;
use std::sync::Arc;
use std::error::Error;
use mechtron_core::message::Message;

pub trait Router {
    fn send(&mut self, message: Message);
}

pub struct GlobalRouter<'configs> {
    local: LocalRouter<'configs>,
    sys: Option<Arc<Mechtronium<'configs>>>,
}

impl <'configs> GlobalRouter<'configs> {
    pub fn new() -> Self {
        GlobalRouter {
            local: LocalRouter::new(),
            sys: Option::None,
        }
    }

    pub fn init(&mut self, sys: Arc<Mechtronium<'configs>>) {
        self.local.init(sys.clone());
        self.sys = Option::Some(sys);
    }

    pub fn sys<'get>(&'get self) -> Result<Arc<Mechtronium<'configs>>, Box<dyn Error>> {
        match &self.sys {
            None => Err("sys is not set".into()),
            Some(sys) => Ok(sys.clone()),
        }
    }
}

impl <'configs> Router for GlobalRouter<'configs> {
    fn send(&mut self, message: Message) {
        self.local.send(message);
    }
}

struct LocalRouter<'configs> {
    sys: Option<Arc<Mechtronium<'configs>>>,
}

impl <'configs> LocalRouter<'configs> {
    pub fn new() -> Self {
        LocalRouter { sys: Option::None }
    }

    pub fn init(&mut self, sys: Arc<Mechtronium<'configs>>) {
        self.sys = Option::Some(sys);
    }

    pub fn sys(&self) -> Result<Arc<Mechtronium<'configs>>, Box<dyn Error>> {
        match &self.sys {
            None => Err("sys is not set".into()),
            Some(sys) => Ok(sys.clone()),
        }
    }
}

impl <'configs> Router for LocalRouter<'configs> {
    fn send(&mut self, message: Message) {
        if self.sys().is_err() {
            println!("cannot send message because MessageRouter has no connection to the system");
            return;
        }

        match self
            .sys()
            .unwrap()
            .local
            .nuclei
            .get(&message.to.tron.nucleus)
        {
            Ok(nucleus) => {
                let mut nucleus = nucleus;
                nucleus.intake(message);
            }
            Err(e) => {
                print!("message failed to be sent: {:?}", e)
            }
        }
    }
}

use crate::node::Node;
use std::sync::Arc;
use std::error::Error;
use mechtron_core::message::Message;
use crate::nucleus::Nuclei;

pub trait Router {
    fn send(&self, message: Arc<Message>);
}

pub struct GlobalRouter {
}

impl GlobalRouter {
    pub fn new() -> Self {
        GlobalRouter {
//            local: LocalRouter::new()
        }
    }

}

impl  Router for GlobalRouter {
    fn send(&self, message: Arc<Message>) {
//        self.local.send(message);
    }
}

pub struct LocalRouter {
}

impl  LocalRouter {

}

impl  Router for LocalRouter {
    fn send(&self, message: Arc<Message>) {

        /*
        let nucleus = self .sys() .unwrap() .local .nuclei() .get(&message.to.tron.nucleus);

        match nucleus
        {
            Ok(mut nucleus) => {
                nucleus.intake(message);
            }
            Err(e) => {
                print!("message failed to be sent: {:?}", e)
            }
        }

         */
        unimplemented!()
    }
}

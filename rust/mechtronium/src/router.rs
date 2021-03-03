use std::cell::{Cell, RefCell};
use std::error::Error;
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::Weak;

use mechtron_core::id::Id;
use mechtron_core::message::Message;

use crate::node::{Local, Node};
use crate::nucleus::{Nuclei, NucleiContainer};

pub trait Router {
    fn send(&self, message: Arc<Message>);
    fn receive(&self, message: Arc<Message>);
    fn has_nucleus_local(&self, nucleus: &Id) ->HasNucleus;
}

pub enum HasNucleus
{
    Yes,
    No,
    NotSure
}

pub struct NetworkRouter<'local> {
    shared: Arc<SharedRouter<'local,LocalRouter<'local>,NetworkRouter<'local>>>
}

impl <'local> NetworkRouter<'local>
{
    pub fn new( shared: Arc<SharedRouter<'local,LocalRouter<'local>,NetworkRouter<'local>>>) -> Self {
        NetworkRouter{
            shared: shared
        }
    }
}

impl <'local> Router for NetworkRouter<'local>{

    fn send(&self, message: Arc<Message>) {

    }

    fn receive(&self, message: Arc<Message>) {
        self.shared.receive(message);
    }

    fn has_nucleus_local(&self, nucleus: &Id) ->HasNucleus
    {
        self.shared.has_nucleus_local(nucleus)
    }

}


pub struct LocalRouter<'local>
 {
     local: Arc<SharedRouter<'local,Local<'local>,LocalRouter<'local>>>
 }

impl  <'local> LocalRouter<'local>
{
    pub fn new( shared: Arc<SharedRouter<'local,Local<'local>,LocalRouter<'local>>>) -> Self {
        LocalRouter{
            local: shared
        }
    }
}

impl <'local> LocalRouter<'local> {

    fn panic( &self, message: &str )
    {
        println!("MESSAGE PANIC: {} ",message);
    }
}

impl <'local> Router for LocalRouter<'local> {

    fn send(&self, message: Arc<Message>) {
        match self.has_nucleus_local(&message.to.tron.nucleus)
        {
            HasNucleus::Yes => {
                self.receive(message);
            }
            HasNucleus::No => {
                self.send(message);
            }
            HasNucleus::NotSure => {
                self.send(message);
            }
        }
    }


    fn receive(&self, message: Arc<Message>) {
        self.local.receive(message);
    }

    fn has_nucleus_local(&self, nucleus: &Id) -> HasNucleus {
        self.local.has_nucleus_local(nucleus)
    }


}

impl <'local> Drop for LocalRouter<'local>
{
    fn drop(&mut self) {
        println!("DROPING NODE!");
//        self.destroy();
    }
}

pub struct SharedRouter<'local,L: Router+'local,R: Router+'local>
{
    pub local : RefCell<Option<Weak<L>>>,
    pub remote: RefCell<Option<Weak<R>>>,
    pub phantom: PhantomData<&'local R>
}

impl <'local,L: Router+'local,R: Router+'local> SharedRouter<'local,L,R>
{
    pub fn new( ) -> Self {
        SharedRouter{
            local: RefCell::new(Option::None),
            remote: RefCell::new(Option::None),
            phantom: PhantomData::default()
        }
    }

    pub fn set_local( &self, local : Arc<L>)
    {
        self.local.replace(Option::Some( Arc::downgrade(&local)));
    }

    pub fn set_remote( &self, remote : Arc<R>)
    {
        self.remote.replace(Option::Some( Arc::downgrade(&remote)));
    }

    fn panic(&self)
    {
        println!("shared router failure!");
    }
}

impl <'local,L: Router+'local,R: Router+'local>Router for  SharedRouter<'local,L,R>
{
    fn send(&self, message: Arc<Message>) {


        let router = self.remote.borrow();

        if router.is_none()
        {
            self.panic( );
            return;
        }

        let router = router.clone();
        let router = router.unwrap();

        let router = router.upgrade();

        if router.is_none()
        {
            println!("shared router could not upgrade");
            self.panic( );
            return;
        }

        let router = router.unwrap();
        router.send(message);
    }

    fn receive(&self, message: Arc<Message>) {

        let router = self.local.borrow();

        if router.is_none()
        {
            self.panic( );
            return;
        }

        let router = router.clone();
        let router = router.unwrap();

        let router = router.upgrade();

        if router.is_none()
        {
            println!("shared router could not upgrade");
            self.panic( );
            return;
        }

        let router = router.unwrap();
        router.receive(message);
    }


    fn has_nucleus_local(&self, nucleus: &Id) -> HasNucleus {
        let router = self.local.borrow();

        if router.is_none()
        {
            self.panic( );
            return HasNucleus::No;
        }

        let router = router.clone();
        let router = router.unwrap();
        let router = router.upgrade();

        if router.is_none()
        {
            println!("shared router could not upgrade");
            self.panic( );
            return HasNucleus::No;
        }

        let router = router.unwrap();
        router.has_nucleus_local(nucleus)
    }


}



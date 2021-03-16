use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::error::Error;
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::Weak;

use mechtron_common::id::Id;
use mechtron_common::message::{Message, MessageTransport};

use crate::network::Connection;
use crate::star::{Local, Star};
use crate::nucleus::{Nuclei, NucleiContainer};

pub trait InternalRouter {
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

pub struct NetworkRouter {
    shared: Arc<SharedRouter<LocalRouter,NetworkRouter>>
}

impl  NetworkRouter
{
    pub fn new( shared: Arc<SharedRouter<LocalRouter,NetworkRouter>>) -> Self {
        NetworkRouter{
            shared: shared
        }
    }
}

impl InternalRouter for NetworkRouter{

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


pub struct LocalRouter
 {
     local: Arc<SharedRouter<Local,LocalRouter>>
 }

impl  LocalRouter
{
    pub fn new( shared: Arc<SharedRouter<Local,LocalRouter>>) -> Self {
        LocalRouter{
            local: shared
        }
    }
}

impl LocalRouter {

    fn panic( &self, message: &str )
    {
        println!("MESSAGE PANIC: {} ",message);
    }
}

impl InternalRouter for LocalRouter {

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

impl Drop for LocalRouter
{
    fn drop(&mut self) {
        println!("DROPING NODE!");
//        self.destroy();
    }
}

pub struct SharedRouter<L: InternalRouter,R: InternalRouter>
{
    pub local : RefCell<Option<Weak<L>>>,
    pub remote: RefCell<Option<Weak<R>>>,
}

impl <L: InternalRouter,R: InternalRouter> SharedRouter<L,R>
{
    pub fn new( ) -> Self {
        SharedRouter{
            local: RefCell::new(Option::None),
            remote: RefCell::new(Option::None),
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

impl <L: InternalRouter,R: InternalRouter> InternalRouter for  SharedRouter<L,R>
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




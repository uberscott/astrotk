use mechtron::app::SYS;
use mechtron_common::message::Message;

fn main() -> Result<(),Box<dyn std::error::Error>>{

    let sim_id= SYS.net.id_seq.next();
    let nucleus_id= SYS.net.id_seq.next();
    SYS.local.sources.add( &sim_id );
    let mut source = SYS.local.sources.get( &sim_id )?;
    source.add_nuclues(nucleus_id);

    let create_neutron_message = Message::new( &SYS.net.id_seq,
                                                         );
    source.messaging.cyclic_intake().intake(message);

    Ok(())
}



use mechtron::app::SYS;

fn main() -> Result<(),Box<dyn std::error::Error>>{

    let sim_id= SYS.net.id_seq.next();
    let nucleus_id= SYS.net.id_seq.next();
    SYS.local.sources.add( sim_id );
    let mut source = SYS.local.sources.get( sim_id )?;
    source.add_nuclues(nucleus_id);

    Ok(())
}



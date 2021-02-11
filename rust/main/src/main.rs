use mechtron::app::SYS;

fn main() -> Result<(),Box<dyn std::error::Error>>{

    let sim_id = SYS.net.next_sim_id();
    let nucleus_id= SYS.net.next_nucleus_id();
    SYS.local.sources.add( sim_id );
    let source = SYS.local.sources.get( sim_id )?;
    source.nuclei.add_to_sim(sim_id, nucleus_id);

    Ok(())

}



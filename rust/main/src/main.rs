use mechtron::app::App;

fn main() -> Result<(),Box<dyn std::error::Error>>{
    let mut app = App::new();
    app.create_source_simulation("mechtron:examples:0.0.1:hello-world-simulation.yaml")?;
    app.run()?;
    Ok(())
}



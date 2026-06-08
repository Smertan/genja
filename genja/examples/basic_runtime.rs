use genja::Genja;

fn main() -> Result<(), genja::GenjaError> {
    let genja = Genja::from_settings_file("genja/examples/settings.yaml")?;

    println!("Loaded hosts:");
    for host_id in genja.host_ids() {
        println!("- {host_id}");
    }

    Ok(())
}

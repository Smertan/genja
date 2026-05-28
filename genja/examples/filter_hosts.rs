use genja::Genja;

fn main() -> Result<(), genja::GenjaError> {
    let genja = Genja::from_settings_file("genja/examples/settings.yaml")?;

    let core_site = genja.filter_by_key_value("data.site.name", "^core$")?;

    println!("Hosts in the core site:");
    for host_id in core_site.host_ids() {
        println!("- {host_id}");
    }

    Ok(())
}

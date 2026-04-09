mod config;

fn main() {
    let cfg = config::Config::load();
    println!("Loaded {} portals and {} tunnels", cfg.portals.len(), cfg.tunnels.len());
}

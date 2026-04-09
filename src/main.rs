mod config;
mod fzf;
mod resolve;

fn main() {
    let cfg = config::Config::load();
    println!("Loaded {} portals and {} tunnels", cfg.portals.len(), cfg.tunnels.len());
}

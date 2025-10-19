use hjkl_snake::{GameConfig, GameState, rasterize_game};

fn tick(g: &mut GameState) {
    println!("===============================");
    println!("{}", rasterize_game(g));
    let res = g.tick();
    println!("===============================");
    println!("{:?}", res);
}

fn main() {
    let cfg = GameConfig {
        width: 10,
        height: 8,
        wrap_edges: false,
        initial_len: 3,
        braille_friendly: true,
    };
    let mut g = GameState::with_seed(cfg, 42);
    tick(&mut g);
    tick(&mut g);
    tick(&mut g);
    tick(&mut g);
    tick(&mut g);
}

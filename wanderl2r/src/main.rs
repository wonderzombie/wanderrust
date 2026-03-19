use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use clap::Parser;
use wanderrust::cell::Cell;
use wanderrust::tiles::{self, TileIdx};

use tracing::warn;

#[derive(Parser)]
struct Cli {
    #[arg(short, long)]
    path: std::path::PathBuf,
}

fn main() {
    println!("Hello, world!");

    let args = Cli::parse();

    let mut p = args.path.clone();

    if !args.path.exists() {
        p = PathBuf::from(
            "/Users/wonderzombie/src/wanderrust/wanderl2r/data/tile_replacer.smugglers_cave.json",
        );
    }

    let content = std::fs::read_to_string(&p).expect("could not read file");
    let json: Value = serde_json::from_str(&content).expect("could not parse json");

    let map = json.as_object().expect("expected json to result in object");
    println!("read object with entries:\n{:?}", map.keys());

    // Now load the reverse lookup map
    let reverse_map = reverse_lookup();

    for node_path in map.keys() {
        if !node_path.ends_with("_level") {
            continue;
        }

        let key = node_path.as_str();
        println!("[+] LEVEL: {}", key);
        handle_level(
            &reverse_map,
            map.get(node_path).expect("expected key to exist"),
        );
    }

    println!("done");
}

fn reverse_lookup() -> HashMap<usize, TileIdx> {
    let mut map = HashMap::new();
    for tile in TileIdx::all() {
        let idx: usize = tile.into();
        map.entry(idx).or_insert(*tile);
    }
    map
}

fn handle_level(reverse_map: &HashMap<usize, TileIdx>, val: &Value) {
    let level_info = val.as_array().expect("expected level to be an array");

    println!(" - level_info: {} values", level_info.len());

    let mut counts: HashMap<&TileIdx, usize> = HashMap::new();
    let mut missing: HashSet<(Cell, usize)> = HashSet::new();

    for v in level_info.iter() {
        let map = v.as_object().expect("expected level info to have objects");

        let coords = map
            .get("atlas_coords")
            .expect("expected atlas_coords to exist")
            .as_array()
            .expect("expected atlas_coords to be an array");

        // println!("level_info[{}]: atlas_coords {:?}", i, coords);
        //
        //

        let cell = cell_from_array(
            coords
                .as_array()
                .expect("expected atlas_coords to be an array of 2 values"),
        );
        let idx = cell.to_idx(tiles::SHEET_SIZE_G.x);

        let Some(tile) = reverse_map.get(&idx) else {
            warn!("missing: atlas_idx {} atlas_coords: {}", idx, cell);
            missing.insert((cell, idx));
            continue;
        };

        // println!("{:?} at atlas_idx {} atlas_coords {}", tile, idx, cell);

        counts.entry(tile).and_modify(|v| *v += 1).or_insert(1);
    }

    println!("[-] counts: {:?}", counts);

    println!("[-] missing: {:?}", missing);
}

fn cell_from_array(coords: &[Value; 2]) -> Cell {
    Cell {
        x: coords[0]
            .as_i64()
            .expect("expected atlas_coords[0] to be an integer") as i32,
        y: coords[1]
            .as_i64()
            .expect("expected atlas_coords[1] to be an integer") as i32,
    }
}

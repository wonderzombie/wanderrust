use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use wanderrust::light::LightLevel;
use wanderrust::tilemap::{Dimensions, SavedTilemap, Stratum};

use clap::Parser;
use wanderrust::cell::Cell;
use wanderrust::tiles::{self, TileIdx};

use log::warn;

#[derive(Parser)]
struct Cli {
    #[arg(short, long)]
    path: std::path::PathBuf,
}

fn main() {
    let args = Cli::parse();
    let mut p = args.path.clone();
    if !args.path.exists() {
        p = PathBuf::from(
            "/Users/wonderzombie/src/wanderrust/wanderl2r/data/tile_exporter.smugglers_cave.json",
        );
    }

    let content = std::fs::read_to_string(&p).expect("could not read file");
    let json: Value = serde_json::from_str(&content).expect("could not parse json");

    let map_info = json.as_object().expect("expected json to result in object");
    println!("read object with entries:\n{:?}", map_info.keys());

    // Now load the reverse lookup map so we can use `usize` to get [`TileIdx`].
    let reverse_lookup = reverse_lookup();

    let mut saved_maps: HashMap<String, SavedTilemap> = HashMap::new();
    for (node_path, val) in map_info.iter() {
        if !node_path.ends_with("_level") {
            println!("skipping {} (not a level)", node_path);
            continue;
        }
        println!("[+] {}", node_path);

        let info = val
            .as_array()
            .expect("expected cell tile info to be an array");

        let cell_data = load_from_exporter_json(info);
        println!("• loaded {} cells", cell_data.len());

        let (tiles, size) = fill_map(&reverse_lookup, cell_data);
        println!("• filled {:?} cells ({:?})", tiles.len(), size);

        saved_maps.insert(
            node_path
                .replace("/", "_")
                .replace("'", "")
                .replace(" ", ""),
            SavedTilemap {
                tiles,
                size,
                light_level: LightLevel::Light,
                flip_v: true,
                ..default()
            },
        );
    }

    for (node_path, saved) in saved_maps.iter_mut() {
        if let Ok(serialized) = ron::to_string(&saved) {
            let path = format!("../data/{}.ron", node_path);
            let Ok(_) = std::fs::write(&path, serialized) else {
                warn!("failed to write tilemap for map {}", node_path);
                continue;
            };

            println!("wrote {}", path);
        }
    }
}

fn load_from_exporter_json(map_info: &[Value]) -> HashMap<Cell, usize> {
    let mut cell_data: HashMap<Cell, usize> = HashMap::new();
    for value in map_info {
        let tile_info = value
            .as_object()
            .expect("expected tile info to be an object")
            .clone();

        let idx = get_atlas_idx(&tile_info).expect("expected atlas_coords to be present");
        let cell = get_cell(&tile_info).expect("expected cell to be present");

        cell_data.insert(cell, idx as usize);
    }
    cell_data
}

fn get_atlas_idx(tile_info: &Map<String, Value>) -> Option<usize> {
    let atlas_coords = tile_info.get("atlas_coords")?;
    let atlas_coords = json2cell(atlas_coords).ok()?;
    Some(atlas_coords.to_idx(tiles::SHEET_SIZE_G.x))
}

fn get_cell(tile_info: &Map<String, Value>) -> Option<Cell> {
    let cell_val = tile_info.get("cell")?;
    json2cell(cell_val).ok()
}

fn fill_map(
    atlas_to_tile_idx: &HashMap<usize, TileIdx>,
    cell_to_atlas_idx: HashMap<Cell, usize>,
) -> (Vec<(TileIdx, Stratum)>, Dimensions) {
    let dims = calculate_dimensions(&cell_to_atlas_idx);

    let out: Vec<(TileIdx, Stratum)> = (0..dims.ntiles() as usize)
        .map(|idx| {
            let cell = dims.idx_to_cell(idx as u32);
            // We are effectively joining these two HashMaps. However, we also
            // need to visit each tile no matter what, and it has to be
            // *something* (for now).
            cell_to_atlas_idx
                .get(&cell)
                .and_then(|&src_idx| atlas_to_tile_idx.get(&src_idx))
                .map(|&tile_idx| (tile_idx, Stratum::default()))
                .unwrap_or_default()
        })
        .collect();

    let mut tally: HashMap<TileIdx, usize> = HashMap::new();
    for (tile_idx, _) in &out {
        *tally.entry(*tile_idx).or_default() += 1;
    }
    println!("• tile breakdown: {:#?}", tally);

    (out, dims)
}

fn calculate_dimensions<T>(cell_data: &HashMap<Cell, T>) -> Dimensions {
    let ul_x = cell_data.keys().map(Cell::x).min().unwrap_or(0);
    let ul_y = cell_data.keys().map(Cell::y).min().unwrap_or(0);
    let ul = Cell { x: ul_x, y: ul_y };
    let lr_x = cell_data.keys().map(Cell::x).max().unwrap_or(0);
    let lr_y = cell_data.keys().map(Cell::y).max().unwrap_or(0);
    let lr = Cell { x: lr_x, y: lr_y };

    let size = lr - ul;
    let dims = Dimensions {
        width: size.x as u32,
        height: size.y as u32,
        tile_size: 16,
    };
    dims
}

fn json2cell(value: &Value) -> Result<Cell, anyhow::Error> {
    let arr = value
        .as_array()
        .ok_or(anyhow::anyhow!("not a valid array: {}", value))?;
    let x = arr[0]
        .as_i64()
        .ok_or(anyhow::anyhow!("not a valid integer: {}", arr[0]))? as i32;
    let y = arr[1]
        .as_i64()
        .ok_or(anyhow::anyhow!("not a valid integer: {}", arr[1]))? as i32;
    Ok(Cell { x, y })
}

/// See [`TileIdx`], especially [`tiles::tiles!`] for the definition of tiles.
/// wanderrust doesn't need to map an atlas index (Bevy's [`bevy::sprite::TextureAtlas`]) to
/// a tile; it's typically the other way around for any system that syncs
/// [`TileIdx`] with [`bevy::sprite::Sprite`].
///
/// Here we generate a reverse mapping using TileIdx::all(). This allows us to
/// translate from wanderlust's `tile_replacer` (or `tile_exporter`) format into
/// something we can translate into SavedTilemap and then [`ron`].
fn reverse_lookup() -> HashMap<usize, TileIdx> {
    let mut map = HashMap::new();
    for tile in TileIdx::all() {
        let idx: usize = tile.into();
        map.entry(idx).or_insert(*tile);
    }
    map
}

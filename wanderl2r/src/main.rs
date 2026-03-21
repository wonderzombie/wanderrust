use serde_json::Value;
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
    let reverse_lookup = &reverse_lookup();

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

        let cell_data = load_from_exporter_json(info, reverse_lookup);
        println!("• loaded {} cells", cell_data.len());

        let (tiles, size) = fill_map(cell_data);
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
                ..Default::default()
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

fn load_from_exporter_json(
    map_info: &[Value],
    reverse_lookup: &HashMap<usize, TileIdx>,
) -> HashMap<Cell, TileIdx> {
    let mut cell_data: HashMap<Cell, TileIdx> = HashMap::new();
    for value in map_info {
        // TODO: get `"props"` for such as `flip_h`, `flip_v`, and `transpose`.
        let tile_info = value
            .as_object()
            .expect("expected tile info to be an object");

        let cell = tile_info.get("cell").expect("expected cell to be present");
        let atlas_coords = tile_info
            .get("atlas_coords")
            .expect("expected atlas_coords to be present");

        let Some(atlas_coords) = json2cell(atlas_coords).ok() else {
            warn!("failed to parse atlas_coords: {:?}", atlas_coords);
            continue;
        };
        let Some(cell) = json2cell(cell).ok() else {
            warn!("failed to parse cell: {:?}", cell);
            continue;
        };

        let atlas_idx = atlas_coords.to_idx(tiles::SHEET_SIZE_G.x);
        let Some(tile_idx) = reverse_lookup.get(&atlas_idx) else {
            warn!("failed to find tile_idx for atlas_idx: {}", atlas_idx);
            continue;
        };

        cell_data.insert(cell, *tile_idx);
    }
    cell_data
}

fn fill_map(cell_data: HashMap<Cell, TileIdx>) -> (Vec<(TileIdx, Stratum)>, Dimensions) {
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

    let mut out = vec![(TileIdx::default(), Stratum::default()); dims.ntiles() as usize];
    let mut tally: HashMap<TileIdx, usize> = HashMap::new();

    for idx in 0..out.len() {
        let cell = dims.idx_to_cell(idx as u32);
        if let Some(&tile_idx) = cell_data.get(&cell) {
            out[idx] = (tile_idx, Stratum::default());
            *tally.entry(tile_idx).or_default() += 1;
        }
    }

    println!("• tile breakdown: {:?}", tally);

    (out, dims)
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

/// See [`TileIdx`], especially [`tiles!`] for the definition of tiles.
/// wanderrust doesn't care about going from an atlas index (Bevy's default) to
/// a tile; it's typically the other way around for any system that syncs
/// [`TileIdx`] with [`Sprite`].
///
/// Here we generate the opposite by iterating through TileIdx::all(). This
/// allows us to translate from wanderlust's `tile_replacer` format into
/// something we can translate into SavedTilemap and then [`ron`].
fn reverse_lookup() -> HashMap<usize, TileIdx> {
    let mut map = HashMap::new();
    for tile in TileIdx::all() {
        let idx: usize = tile.into();
        map.entry(idx).or_insert(*tile);
    }
    map
}

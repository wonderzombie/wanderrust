use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use wanderrust::light::LightLevel;
use wanderrust::tilemap::{Dimensions, SavedTilemap, Stratum};

use clap::Parser;
use wanderrust::cell::{self, Cell};
use wanderrust::tiles::{self, TileIdx};

use indicatif::ProgressBar;
use log::warn;
use std::time::Duration;

#[derive(Parser)]
struct Cli {
    #[arg(short, long)]
    path: std::path::PathBuf,
}

fn main() {
    env_logger::init();

    let args = Cli::parse();
    let mut p = args.path.clone();
    if !args.path.exists() {
        p = PathBuf::from(
            "/Users/wonderzombie/src/wanderrust/wanderl2r/data/tile_replacer.smugglers_cave.json",
        );
    }

    let spinner = ProgressBar::new_spinner();
    spinner.set_message("Loading...");
    spinner.enable_steady_tick(Duration::from_millis(100));

    let content = std::fs::read_to_string(&p).expect("could not read file");
    let json: Value = serde_json::from_str(&content).expect("could not parse json");

    let map = json.as_object().expect("expected json to result in object");
    println!("read object with entries:\n{:?}", map.keys());

    // Now load the reverse lookup map
    let reverse_map = reverse_lookup();

    let mut maps: HashMap<String, SavedTilemap> = HashMap::new();

    for node_path in map.keys() {
        if !node_path.ends_with("_level") {
            continue;
        }

        // load_from_json returns (transposed_map, size, offset)
        let (transposed, size_cell, offset) = load_from_json(map, &reverse_map, node_path);
        let size = Dimensions {
            width: size_cell.x as u32,
            height: size_cell.y as u32,
            tile_size: tiles::TILE_SIZE_PX as u32,
        };

        let filled = fill_map(transposed, size_cell, offset);

        let saved = SavedTilemap {
            tiles: filled.clone(),
            size: size,
            light_level: LightLevel::Light,
            ..Default::default()
        };

        let map_name = node_path.replace("/", "_");
        maps.insert(map_name, saved);
    }

    println!("[+] loaded {} maps", maps.len());

    println!("saving");
    for (map_name, saved) in maps.iter() {
        if let Ok(serialized) = ron::to_string(&saved) {
            let path = format!("data/{}.ron", map_name);
            println!("saving {}", path);
            let Ok(_) = std::fs::write(&path, serialized) else {
                continue;
            };
        }
    }

    spinner.finish_and_clear();
    println!("[+] done");
}

fn load_from_json(
    map: &serde_json::Map<String, Value>,
    reverse_map: &HashMap<usize, TileIdx>,
    node_path: &String,
) -> (HashMap<cell::Cell, TileIdx>, cell::Cell, cell::Cell) {
    println!("[+] LEVEL: {}", node_path);
    let level_data = map
        .get(node_path)
        .expect("expected key to exist")
        .as_array()
        .expect("expected level to be an array");

    let transposed = transpose_level_info(reverse_map, level_data);

    let upper_left_x = transposed.keys().map(|it| it.x).min().unwrap_or(0);
    let upper_left_y = transposed.keys().map(|it| it.y).min().unwrap_or(0);
    let upper_left = Cell {
        x: upper_left_x,
        y: upper_left_y,
    };

    let bottom_right_x = transposed.keys().map(|it| it.x).max().unwrap_or(0);
    let bottom_right_y = transposed.keys().map(|it| it.y).max().unwrap_or(0);
    let bottom_right = Cell {
        x: bottom_right_x,
        y: bottom_right_y,
    };

    // wanderlust permits negative cells and normalizes for MRPAS.
    // wanderrust does not permit negative cells, so we offset ahead of time.
    println!("{}: offset: {}", node_path, upper_left);
    println!("{}: bottom_right: {}", node_path, bottom_right);
    let size = bottom_right - upper_left;
    println!("{}: effective map size: {}", node_path, size);
    println!(
        "{}: cells / total = {} / {}",
        node_path,
        transposed.len(),
        size.x * size.y
    );

    (transposed, size, upper_left.neg())
}

type TileStratum = (TileIdx, Stratum);

/// Fills a HashMap with a "full" suite of cells with tiles.
///
/// Important:
///
/// wanderrust maps are treated as square. We store them as a list
/// because we can easily translate between an index and a cell *if* we have
/// a fixed width for every single line. Even if there's nothing *at* a cell,
/// we could still have Option<TileIdx> or TileIdx::Blank.
///
/// wanderlust maps are NOT square. They arrive as a list of cells and they
/// have many irregularities that prevent neat classification based on the
/// existing `tile_replacer.foo.json`.
///
/// This presents an incompatibility which we need to overcome.
///
fn fill_map(
    transposed_map: HashMap<cell::Cell, TileIdx>,
    size: cell::Cell,
    offset: cell::Cell,
) -> Vec<TileStratum> {
    let num_tiles = size.x * size.y;
    let mut map = vec![TileStratum::default(); num_tiles as usize];

    for idx in 0..num_tiles {
        let cell = Cell::from_idx(size.x as u32, idx as usize);

        let offset_cell = cell + offset;

        let tile = transposed_map
            .get(&offset_cell)
            .copied()
            .unwrap_or(TileIdx::default());
        map[idx as usize] = (tile, Stratum::default());
    }

    println!("filled {} tiles", map.len());

    map
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

/// Transposes level info from a list of tile definitions and the cells using it
/// into a list of cells and the tile at that cell (HashMap<Cell, TileIdx>).
///
/// wanderlust maps are not square so the map dimensions are not inferred from
/// the level info. See also [`fill_map`].
fn transpose_level_info(
    reverse_map: &HashMap<usize, TileIdx>,
    level: &[Value],
) -> HashMap<Cell, TileIdx> {
    let mut transposed = HashMap::new();
    for v in level.iter() {
        let tile_to_cell_map = v.as_object().expect("expected level info to have objects");

        let atlas_coords = tile_to_cell_map
            .get("atlas_coords")
            .expect("expected level info to have atlas_coords");

        let Some(atlas_coords) = json2cell(atlas_coords).ok() else {
            warn!("failed to parse atlas_coords: {:?}", atlas_coords);
            continue;
        };

        let atlas_idx = atlas_coords.to_idx(tiles::SHEET_SIZE_G.x);
        let Some(tile) = reverse_map.get(&atlas_idx).copied() else {
            warn!("failed to find tile for atlas_idx: {}", atlas_idx);
            continue;
        };

        let map_cells = tile_to_cell_map
            .get("cells")
            .expect("expected level_info to have cells")
            .as_array()
            .expect("expected cells to be an array");

        for map_cell in map_cells.iter() {
            if let Ok(cell) = json2cell(map_cell) {
                transposed.insert(cell, tile);
            } else {
                warn!("failed to parse cell: {:?}", map_cell);
            }
        }
    }
    transposed
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

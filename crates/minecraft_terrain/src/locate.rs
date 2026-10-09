use std::collections::BTreeMap;
use std::time::Instant;

use minecraftoss_core::ChunkPos;
use minecraftoss_core::ident::Identifier;
use minecraftoss_generator::structure::StructureId;
use minecraftoss_generator::structure::placement::{Kind, Placement};
use minecraftoss_world::chunk_map::WorldGen;
use serde_json::Value;

pub const STRUCTURE_RADIUS: i32 = 100;
pub const BIOME_RADIUS: i32 = 6400;
pub const BIOME_STEP: i32 = 32;
pub const BIOME_VERTICAL_STEP: i32 = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Search {
    Found([i32; 3], String),
    NotFound,
    TimedOut,
}

struct TimedOut;

fn check(deadline: Instant) -> Result<(), TimedOut> {
    if Instant::now() > deadline {
        Err(TimedOut)
    } else {
        Ok(())
    }
}

pub fn structure_names(gen: &WorldGen) -> Vec<String> {
    gen.structures
        .defs
        .iter()
        .map(|def| def.name.clone())
        .collect()
}

pub fn biome_names(gen: &WorldGen) -> Vec<String> {
    gen.terrain
        .registries
        .biomes
        .iter()
        .map(|(_, info)| info.name.to_string())
        .collect()
}

pub fn tags(gen: &WorldGen, registry: &str) -> BTreeMap<String, Vec<String>> {
    let pack = &gen.terrain.registries.datapack;
    let kind = format!("tags/{registry}");
    let mut raw = BTreeMap::new();
    for id in pack.list(&kind).unwrap_or_else(|_| Vec::new()) {
        if let Ok(json) = pack.read_json(&kind, &id) {
            raw.insert(id.to_string(), json);
        }
    }
    raw.keys()
        .map(|id| {
            let mut members = Vec::new();
            expand(&raw, id, &mut members, 0);
            (id.clone(), members)
        })
        .collect()
}

fn expand(raw: &BTreeMap<String, Value>, id: &str, out: &mut Vec<String>, depth: usize) {
    let Some(values) = raw.get(id).and_then(|json| json["values"].as_array()) else {
        return;
    };
    if depth > 32 {
        return;
    }
    for entry in values {
        let text = match entry {
            Value::String(text) => text.as_str(),
            Value::Object(fields) => fields.get("id").and_then(Value::as_str).unwrap_or(""),
            _ => "",
        };
        let Some(nested) = text.strip_prefix('#') else {
            if let Ok(id) = Identifier::parse(text) {
                let id = id.to_string();
                if !out.contains(&id) {
                    out.push(id);
                }
            }
            continue;
        };
        if let Ok(nested) = Identifier::parse(nested) {
            expand(raw, &nested.to_string(), out, depth + 1);
        }
    }
}

fn distance_squared(a: [i32; 3], b: [i32; 3]) -> f64 {
    (0..3)
        .map(|axis| {
            let delta = f64::from(a[axis]) - f64::from(b[axis]);
            delta * delta
        })
        .sum()
}

fn locate_pos(placement: &Placement, chunk: ChunkPos) -> [i32; 3] {
    let pos = placement.locate_pos(chunk);
    [pos.x, pos.y, pos.z]
}

fn generating_at(
    gen: &WorldGen,
    members: &[StructureId],
    placement: &Placement,
    chunk: ChunkPos,
) -> Option<([i32; 3], StructureId)> {
    let starts = gen.structures.starts_in(&gen.library, &gen.terrain, chunk);
    members
        .iter()
        .find(|member| starts.iter().any(|start| start.structure == **member))
        .map(|member| (locate_pos(placement, chunk), *member))
}

fn possible_sets(gen: &WorldGen) -> Vec<bool> {
    let possible = gen.terrain.possible_biomes();
    let defs = &gen.structures.defs;
    gen.structures
        .sets()
        .iter()
        .map(|set| {
            set.structures.iter().any(|(id, _)| {
                let biomes = &defs[usize::from(id.0)].biomes;
                possible
                    .iter()
                    .any(|biome| biomes.get(usize::from(biome.0)).copied().unwrap_or(false))
            })
        })
        .collect()
}

pub fn nearest_structure(
    gen: &WorldGen,
    targets: &[String],
    origin: [i32; 3],
    radius: i32,
    deadline: Instant,
) -> Search {
    match search_structures(gen, targets, origin, radius, deadline) {
        Ok(Some((pos, id))) => {
            Search::Found(pos, gen.structures.defs[usize::from(id.0)].name.clone())
        }
        Ok(None) => Search::NotFound,
        Err(TimedOut) => Search::TimedOut,
    }
}

fn search_structures(
    gen: &WorldGen,
    targets: &[String],
    origin: [i32; 3],
    radius: i32,
    deadline: Instant,
) -> Result<Option<([i32; 3], StructureId)>, TimedOut> {
    let structures = &gen.structures;
    if !structures.enabled {
        return Ok(None);
    }
    let possible = possible_sets(gen);
    let mut groups: Vec<(usize, Vec<StructureId>)> = Vec::new();
    for id in targets.iter().filter_map(|name| structures.id(name)) {
        for (index, set) in structures.sets().iter().enumerate() {
            if !possible[index] || !set.structures.iter().any(|(member, _)| *member == id) {
                continue;
            }
            match groups.iter_mut().find(|(known, _)| *known == index) {
                Some((_, members)) if !members.contains(&id) => members.push(id),
                Some(_) => {}
                None => groups.push((index, vec![id])),
            }
        }
    }
    let mut best = None;
    let mut best_distance = f64::MAX;
    let mut spread = Vec::new();
    for (index, members) in &groups {
        let placement = structures.placement(*index);
        match placement.kind {
            Kind::ConcentricRings { .. } => {
                let found = nearest_ring(gen, members, placement, origin, deadline)?;
                if let Some((pos, id)) = found {
                    let distance = distance_squared(origin, pos);
                    if distance < best_distance {
                        best_distance = distance;
                        best = Some((pos, id));
                    }
                }
            }
            Kind::RandomSpread { spacing, .. } => spread.push((placement, members, spacing)),
            Kind::DimensionOrigin => {}
        }
    }
    if spread.is_empty() {
        return Ok(best);
    }
    let (chunk_x, chunk_z) = (origin[0] >> 4, origin[2] >> 4);
    for ring in 0..=radius {
        let mut found_any = false;
        for (placement, members, spacing) in &spread {
            let found = nearest_in_ring(
                gen,
                members,
                placement,
                (chunk_x, chunk_z),
                ring,
                *spacing,
                deadline,
            )?;
            if let Some((pos, id)) = found {
                found_any = true;
                let distance = distance_squared(origin, pos);
                if distance < best_distance {
                    best_distance = distance;
                    best = Some((pos, id));
                }
            }
        }
        if found_any {
            return Ok(best);
        }
    }
    Ok(best)
}

fn nearest_ring(
    gen: &WorldGen,
    members: &[StructureId],
    placement: &Placement,
    origin: [i32; 3],
    deadline: Instant,
) -> Result<Option<([i32; 3], StructureId)>, TimedOut> {
    let mut best = None;
    let mut best_distance = f64::MAX;
    for chunk in placement.ring_chunks(&gen.terrain, usize::MAX) {
        check(deadline)?;
        let center = [chunk.x * 16 + 8, 32, chunk.z * 16 + 8];
        let distance = distance_squared(center, origin);
        if best.is_some() && distance >= best_distance {
            continue;
        }
        if let Some(found) = generating_at(gen, members, placement, chunk) {
            best = Some(found);
            best_distance = distance;
        }
    }
    Ok(best)
}

fn nearest_in_ring(
    gen: &WorldGen,
    members: &[StructureId],
    placement: &Placement,
    (chunk_x, chunk_z): (i32, i32),
    ring: i32,
    spacing: i32,
    deadline: Instant,
) -> Result<Option<([i32; 3], StructureId)>, TimedOut> {
    let seed = gen.terrain.seed;
    for dx in -ring..=ring {
        let edge_x = dx == -ring || dx == ring;
        for dz in -ring..=ring {
            let edge_z = dz == -ring || dz == ring;
            if !edge_x && !edge_z {
                continue;
            }
            check(deadline)?;
            let x = chunk_x.wrapping_add(spacing.wrapping_mul(dx));
            let z = chunk_z.wrapping_add(spacing.wrapping_mul(dz));
            let Some(chunk) = placement.potential_chunk(seed, x, z) else {
                continue;
            };
            if let Some(found) = generating_at(gen, members, placement, chunk) {
                return Ok(Some(found));
            }
        }
    }
    Ok(None)
}

pub struct Spiral {
    legs: i32,
    leg: i32,
    leg_size: i32,
    leg_index: i32,
    last: (i32, i32),
}

const SPIRAL_DIRECTIONS: [(i32, i32); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];

impl Spiral {
    pub fn new(radius: i32) -> Self {
        Self {
            legs: 4 * radius,
            leg: -1,
            leg_size: 0,
            leg_index: 0,
            last: (0, 1),
        }
    }
}

impl Iterator for Spiral {
    type Item = (i32, i32);

    fn next(&mut self) -> Option<(i32, i32)> {
        let (dx, dz) = SPIRAL_DIRECTIONS[((self.leg + 4) % 4) as usize];
        self.last = (self.last.0 + dx, self.last.1 + dz);
        if self.leg_index >= self.leg_size {
            if self.leg >= self.legs {
                return None;
            }
            self.leg += 1;
            self.leg_index = 0;
            self.leg_size = self.leg / 2 + 1;
        }
        self.leg_index += 1;
        Some(self.last)
    }
}

pub fn out_from_origin(origin: i32, lower: i32, upper: i32, step: i32) -> Vec<i32> {
    let center = origin.clamp(lower, upper);
    let mut out = Vec::new();
    let mut current = center;
    loop {
        let distance = (center - current).abs();
        if center - distance < lower && center + distance > upper {
            return out;
        }
        out.push(current);
        let was_lower = current <= center;
        let can_go_up = center + distance + step <= upper;
        let mut next = center + distance + step;
        if !was_lower || !can_go_up {
            let down = center - distance - if was_lower { step } else { 0 };
            if down >= lower {
                next = down;
            }
        }
        current = next;
    }
}

pub fn nearest_biome(
    gen: &WorldGen,
    targets: &[String],
    origin: [i32; 3],
    radius: i32,
    deadline: Instant,
) -> Search {
    let terrain = &gen.terrain;
    let registry = &terrain.registries.biomes;
    let wanted: Vec<_> = terrain
        .possible_biomes()
        .into_iter()
        .filter(|biome| targets.contains(&registry.get(*biome).name.to_string()))
        .collect();
    if wanted.is_empty() {
        return Search::NotFound;
    }
    let lower = terrain.chunk_min_y + 1;
    let upper = terrain.chunk_min_y + terrain.chunk_height;
    let heights = out_from_origin(origin[1], lower, upper, BIOME_VERTICAL_STEP);
    for (index, (step_x, step_z)) in Spiral::new(radius.div_euclid(BIOME_STEP)).enumerate() {
        if index % 256 == 0 && Instant::now() > deadline {
            return Search::TimedOut;
        }
        let x = origin[0] + step_x * BIOME_STEP;
        let z = origin[2] + step_z * BIOME_STEP;
        for &y in &heights {
            let biome = terrain.biome_at_quart(x >> 2, y >> 2, z >> 2);
            if wanted.contains(&biome) {
                return Search::Found([x, y, z], registry.get(biome).name.to_string());
            }
        }
    }
    Search::NotFound
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;

    use minecraftoss_core::Registries;
    use minecraftoss_core::registries::DataPaths;
    use minecraftoss_generator::terrain::TerrainGenerator;

    #[test]
    fn the_spiral_walks_out_from_the_centre_like_vanilla() {
        let first: Vec<_> = Spiral::new(1).collect();
        assert_eq!(
            first,
            vec![
                (0, 0),
                (1, 0),
                (1, 1),
                (0, 1),
                (-1, 1),
                (-1, 0),
                (-1, -1),
                (0, -1),
                (1, -1)
            ]
        );
        assert_eq!(Spiral::new(0).collect::<Vec<_>>(), vec![(0, 0)]);
        let wide: Vec<_> = Spiral::new(200).collect();
        assert_eq!(wide.len(), 401 * 401);
        assert!(wide.iter().all(|(x, z)| x.abs() <= 200 && z.abs() <= 200));
    }

    #[test]
    fn heights_alternate_out_from_the_origin_like_vanilla() {
        assert_eq!(
            out_from_origin(64, -63, 320, 64),
            vec![64, 128, 0, 192, 256, 320]
        );
        assert_eq!(
            out_from_origin(-100, -63, 320, 64),
            vec![-63, 1, 65, 129, 193, 257]
        );
        assert_eq!(out_from_origin(5, 0, 10, 3), vec![5, 8, 2]);
    }

    fn overworld() -> Option<WorldGen> {
        let paths = DataPaths::discover().ok()?;
        let registries = Arc::new(Registries::load(&paths).ok()?);
        let terrain = Arc::new(TerrainGenerator::overworld(registries, 0).ok()?);
        WorldGen::new(terrain).ok()
    }

    #[test]
    fn a_seeded_world_finds_villages_and_biomes() {
        let Some(gen) = overworld() else { return };
        let deadline = Instant::now() + Duration::from_secs(120);
        assert!(structure_names(&gen).contains(&"minecraft:village_plains".to_owned()));
        assert!(biome_names(&gen).contains(&"minecraft:desert".to_owned()));
        let villages = tags(&gen, "worldgen/structure");
        assert!(villages["minecraft:village"].contains(&"minecraft:village_plains".to_owned()));
        let village = nearest_structure(
            &gen,
            &villages["minecraft:village"],
            [0, 64, 0],
            STRUCTURE_RADIUS,
            deadline,
        );
        assert_eq!(
            village,
            Search::Found([272, 0, 944], "minecraft:village_plains".to_owned())
        );
        let plains = nearest_structure(
            &gen,
            &["minecraft:village_plains".to_owned()],
            [0, 64, 0],
            STRUCTURE_RADIUS,
            deadline,
        );
        assert_eq!(plains, village);
        let desert = nearest_biome(
            &gen,
            &["minecraft:desert".to_owned()],
            [0, 64, 0],
            BIOME_RADIUS,
            deadline,
        );
        assert_eq!(
            desert,
            Search::Found([608, 64, -1632], "minecraft:desert".to_owned())
        );
        let sampled = gen.terrain.biome_at_quart(608 >> 2, 64 >> 2, -1632 >> 2);
        assert_eq!(
            gen.terrain.registries.biomes.get(sampled).name.to_string(),
            "minecraft:desert"
        );
        let nether = nearest_structure(
            &gen,
            &["minecraft:fortress".to_owned()],
            [0, 64, 0],
            STRUCTURE_RADIUS,
            deadline,
        );
        assert_eq!(nether, Search::NotFound);
        let past = nearest_biome(
            &gen,
            &["minecraft:mushroom_fields".to_owned()],
            [0, 64, 0],
            BIOME_RADIUS,
            Instant::now(),
        );
        assert!(
            matches!(past, Search::TimedOut | Search::Found(..)),
            "{past:?}"
        );
    }
}

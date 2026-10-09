use minecraft_net::Dimension;
use minecraft_terrain::containers::Containers;
use minecraft_terrain::environment::DimensionEnvironment;
use minecraft_terrain::scene::HandcraftedScene;
use minecraft_terrain::terrain::TerrainStream;

use crate::entities::Entities;

pub struct Level {
    pub dimension: Dimension,
    pub entities: Option<Entities>,
    pub containers: Containers,
    pub terrain: Terrain,
}

pub struct Terrain {
    pub stream: TerrainStream,
    pub scene: HandcraftedScene,
    pub environment: DimensionEnvironment,
}

use std::collections::BTreeMap;

use minecraft_net::Dimension;

use crate::level::Level;

pub struct Server {
    pub levels: BTreeMap<Dimension, Level>,
}

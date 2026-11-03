mod creakings;
mod phantoms;

terrain_group!(night {
    phantoms::Phantom,
    creakings::Creaking,
});

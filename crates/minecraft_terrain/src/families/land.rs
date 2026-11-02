mod foxes;
mod pandas;
mod polar_bears;
mod rabbits;

terrain_group!(land {
    rabbits::Rabbit,
    foxes::Fox,
    pandas::Panda,
    polar_bears::PolarBear,
});

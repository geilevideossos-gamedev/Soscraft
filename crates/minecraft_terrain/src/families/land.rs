mod pandas;
mod polar_bears;
mod rabbits;

terrain_group!(land {
    rabbits::Rabbit,
    pandas::Panda,
    polar_bears::PolarBear,
});

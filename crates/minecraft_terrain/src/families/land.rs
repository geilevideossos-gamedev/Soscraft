mod polar_bears;
mod rabbits;

terrain_group!(land {
    rabbits::Rabbit,
    polar_bears::PolarBear,
});

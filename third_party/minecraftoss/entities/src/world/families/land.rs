pub mod animal_goals;
pub mod foxes;
pub mod pandas;
pub mod polar_bears;
pub mod rabbits;

family_group!(land 48..=63 {
    rabbits: rabbits::Rabbit,
    foxes: foxes::Fox,
    pandas: pandas::Panda,
    polar_bears: polar_bears::PolarBear,
});

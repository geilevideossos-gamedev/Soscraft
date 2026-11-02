pub mod animal_goals;
pub mod polar_bears;
pub mod rabbits;

family_group!(land 48..=63 {
    rabbits: rabbits::Rabbit,
    polar_bears: polar_bears::PolarBear,
});

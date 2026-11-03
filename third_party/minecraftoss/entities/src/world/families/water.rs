pub mod abstract_fish;
pub mod fish;
pub mod squids;
pub mod swim;
pub mod swimming;
#[cfg(test)]
mod tests;

family_group!(water 64..=79 {
    squids: squids::Squid,
    fish: fish::Fish,
    #[cfg(test)]
    swim_probe: swimming::swim_probe::SwimProbe,
});

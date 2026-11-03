pub mod squids;
pub mod swim;
pub mod swimming;
#[cfg(test)]
mod tests;

family_group!(water 64..=79 {
    squids: squids::Squid,
    #[cfg(test)]
    swim_probe: swimming::swim_probe::SwimProbe,
});

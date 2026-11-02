pub(crate) mod swimming;
#[cfg(test)]
mod tests;

family_group!(water 64..=79 {
    #[cfg(test)]
    swim_probe: swimming::swim_probe::SwimProbe,
});

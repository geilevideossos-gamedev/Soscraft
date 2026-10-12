use playerstate_iw4::{PlayerState, UserCmd, buttons, pm_flags};

const DROPPED: u32 = pm_flags::SPRINTING
    | pm_flags::JUMPING
    | pm_flags::MANTLE
    | pm_flags::LADDER
    | pm_flags::MC_GLIDING
    | pm_flags::PRONE
    | pm_flags::CROUCH;

pub fn riding(ps: &PlayerState) -> bool {
    ps.pm_flags & pm_flags::MC_RIDING != 0
}

pub fn seat(ps: &mut PlayerState, cmd: &UserCmd) {
    ps.pm_flags &= !DROPPED;
    if cmd.buttons & (buttons::CROUCH | buttons::PRONE) != 0 {
        ps.pm_flags |= pm_flags::CROUCH;
    }
    ps.velocity = [0.0; 3];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crouch_while_riding_sets_the_crouch_flag_only() {
        for held in [buttons::CROUCH, buttons::PRONE] {
            let mut ps = PlayerState::ZERO;
            ps.pm_flags = pm_flags::MC_RIDING | pm_flags::PRONE | pm_flags::SPRINTING;
            ps.velocity = [3.0, 4.0, 5.0];
            let cmd = UserCmd {
                buttons: held,
                ..UserCmd::default()
            };
            seat(&mut ps, &cmd);
            assert_eq!(ps.pm_flags, pm_flags::MC_RIDING | pm_flags::CROUCH);
            assert_eq!(ps.velocity, [0.0; 3]);
        }
        let mut ps = PlayerState::ZERO;
        ps.pm_flags = pm_flags::MC_RIDING | pm_flags::CROUCH;
        seat(&mut ps, &UserCmd::default());
        assert_eq!(
            ps.pm_flags,
            pm_flags::MC_RIDING,
            "letting go stands up again"
        );
        assert!(riding(&ps));
    }
}

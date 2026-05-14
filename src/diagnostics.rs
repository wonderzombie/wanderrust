use bevy::app::App;
use bevy::diagnostic::{Diagnostic, DiagnosticPath, RegisterDiagnostic};

use bevy::prelude::*;

pub(super) fn plugin(app: &mut App) {
    diagnostic_paths().into_iter().for_each(|(path, sfx)| {
        app.register_diagnostic(Diagnostic::new(path).with_suffix(sfx));
    });
}

macro_rules! diagnostics {
    (
        $( [ $id:ident, $path:expr, $sfx:expr $(,)? ] $(,)? )*
    ) => {

        $( pub(crate) const $id: DiagnosticPath = DiagnosticPath::const_new($path); )*

        pub(crate) fn diagnostic_paths() -> Vec<(DiagnosticPath, &'static str)> {
            vec![
                $( ($id, $sfx), )*
            ]
        }
    };
}

diagnostics!(
    [INIT_CALLS, "wanderrust/grid/init_agent", " calls"],
    [
        NOT_ALERTED,
        "wanderrust/grid/pathfind/not_alerted",
        " checks"
    ],
    [PATHFIND_CALLS, "wanderrust/grid/pathfind", " calls"],
    [PATH_ADDED, "wanderrust/grid/pathfind/path_added", " paths"],
    [
        ALREADY_PATHED,
        "wanderrust/grid/pathfind/already_pathed",
        " checks"
    ],
    [MOVE_AGENT_CALLS, "wanderrust/grid/move_agents", " calls"],
    [AGENT_MOVES, "wanderrust/grid/move_agents/move", " moves"],
    [
        AGENT_ATTACKS,
        "wanderrust/grid/move_agents/attack",
        " attacks"
    ],
    [FOV_CHECKS, "wanderrust/mobs/check_fov/calls", " calls"],
    [MOB_ALERTED, "wanderrust/mobs/check_fov/alerted", " alerts"],
    [
        TURN_COMPLETE_CHECKS,
        "wanderrust/gamestate/turns/check",
        " checks"
    ],
    [
        FINALIZED_TURNS,
        "wanderrust/gamestate/turns/finalize",
        " turns"
    ],
    [
        FINALIZE_CALLS,
        "wanderrust/gamestate/turns/finalize/calls",
        " calls",
    ],
    [
        PLAYER_TURN_ENDED,
        "wanderrust/gamestate/turns/player/ended",
        " turns"
    ],
    [
        SET_WAITING,
        "wanderrust/gamestate/turns/others/forced_finalize",
        " actors",
    ],
    [
        COMPLETED,
        "wanderrust/gamestate/turns/others/all_completed",
        " turns"
    ],
);

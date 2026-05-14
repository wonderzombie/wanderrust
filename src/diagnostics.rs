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
        $( [ $id:ident, $path:expr, $sfx:expr ] $(,)? )*
    ) => {

        $( pub(crate) const $id: DiagnosticPath = DiagnosticPath::const_new($path); )*

        pub(crate) fn diagnostic_paths() -> Vec<(DiagnosticPath, &'static str)> {
            vec![
                $( ($id, $sfx), )*
            ]
        }
    };
}

diagnostics!();

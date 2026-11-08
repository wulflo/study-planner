//! Utilidades compartidas por los tests de la capa de persistencia.

use study_planner_core::TermKind;

use crate::{Store, SubjectInput, TermInput};

pub(crate) fn migrated_store() -> Store {
    let mut store = Store::open_in_memory().expect("abrir memoria");
    store.migrate().expect("migrar");
    store
}

pub(crate) fn migrated_store_with_subject() -> (Store, String, String) {
    let store = migrated_store();
    let year = store
        .create_academic_year("2026", "2026-01-01", "2026-12-31")
        .expect("crear año");
    let term = store
        .create_term(
            &year.id,
            &TermInput {
                name: "Semestre 1".to_string(),
                kind: TermKind::Semester.as_str().to_string(),
                start_date: "2026-03-01".to_string(),
                end_date: "2026-07-31".to_string(),
            },
        )
        .expect("crear término");
    let subject = store
        .create_subject(
            &term.id,
            &SubjectInput {
                name: "Álgebra".to_string(),
                ..SubjectInput::default()
            },
        )
        .expect("crear materia");
    (store, subject.id, term.id)
}

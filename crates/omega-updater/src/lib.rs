//! Módulo de actualización aislado.
//!
//! La Fase 5 es la única que puede agregar dependencias de red a este crate.
//! Hasta entonces no se registra ningún plugin de updater ni se expone nada
//! al frontend.

/// Verdadero solo cuando el módulo de actualización está implementado.
pub const UPDATER_IMPLEMENTED: bool = false;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn updater_is_disabled_in_phase_zero() {
        assert!(!UPDATER_IMPLEMENTED);
    }
}

//! Respaldo de instancia única para Unix.
//!
//! La vía principal es `tauri-plugin-single-instance`, que publica un nombre
//! D-Bus y le pasa los argumentos a la instancia viva. En entornos donde ese
//! nombre no se puede publicar (snap `strict`, contenedores, sesiones sin bus)
//! el plugin degrada a «arrancar normalmente» y una segunda apertura agrega
//! otra ventana y otro icono a la bandeja.
//!
//! Este módulo cierra ese hueco: un `flock` exclusivo sobre un archivo en el
//! directorio de datos de la app, más un socket local que le pide a la
//! instancia primaria que muestre su ventana. El lock se libera solo cuando el
//! proceso muere, así que no quedan archivos bloqueados por un cierre abrupto.

use std::fs::{File, OpenOptions, TryLockError};
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

const LOCK_NAME: &str = "instance.lock";
const SOCKET_NAME: &str = "instance.sock";

/// Intentos de conexión al socket de la instancia primaria. Cubre la carrera
/// entre tomar el lock y quedar escuchando.
const CONNECT_ATTEMPTS: usize = 20;
const CONNECT_DELAY: Duration = Duration::from_millis(25);

/// Resultado de la comprobación de instancia.
pub enum Acquired {
    /// Somos la primera instancia; el guard debe vivir hasta el cierre.
    Primary(InstanceGuard),
    /// Ya hay otra instancia viva. `notified` indica si se le pudo pedir que
    /// muestre su ventana por el socket local.
    Secondary { notified: bool },
}

/// Mantiene el lock y el socket de la instancia primaria.
pub struct InstanceGuard {
    _lock: File,
    socket: PathBuf,
}

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.socket);
    }
}

/// Intenta quedarse con la instancia única dentro de `dir`.
///
/// `on_second_instance` se ejecuta en un hilo aparte cada vez que otra
/// instancia pide mostrar la ventana.
pub fn acquire<F>(dir: &Path, on_second_instance: F) -> Result<Acquired, String>
where
    F: Fn() + Send + 'static,
{
    std::fs::create_dir_all(dir)
        .map_err(|error| format!("no se pudo crear {}: {error}", dir.display()))?;

    let lock_path = dir.join(LOCK_NAME);
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock_path)
        .map_err(|error| format!("no se pudo abrir {}: {error}", lock_path.display()))?;

    match lock.try_lock() {
        Ok(()) => {}
        Err(TryLockError::WouldBlock) => {
            return Ok(Acquired::Secondary {
                notified: request_focus(&dir.join(SOCKET_NAME)),
            });
        }
        Err(TryLockError::Error(error)) => {
            return Err(format!(
                "no se pudo bloquear {}: {error}",
                lock_path.display()
            ));
        }
    }

    let socket = dir.join(SOCKET_NAME);
    // Un cierre abrupto puede dejar el archivo del socket; con el lock tomado
    // sabemos que nadie lo está escuchando.
    let _ = std::fs::remove_file(&socket);
    let listener = UnixListener::bind(&socket)
        .map_err(|error| format!("no se pudo escuchar en {}: {error}", socket.display()))?;

    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
                    let mut request = [0_u8; 8];
                    let _ = stream.read(&mut request);
                    let _ = stream.write_all(b"ok");
                    on_second_instance();
                }
                Err(_) => continue,
            }
        }
    });

    Ok(Acquired::Primary(InstanceGuard {
        _lock: lock,
        socket,
    }))
}

/// Le pide a la instancia primaria que muestre la ventana. Si el socket aún no
/// está listo se reintenta unos milisegundos. Devuelve si el aviso se entregó.
fn request_focus(socket: &Path) -> bool {
    for attempt in 0..CONNECT_ATTEMPTS {
        if let Ok(mut stream) = UnixStream::connect(socket) {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
            let _ = stream.write_all(b"show\n");
            let mut reply = [0_u8; 2];
            let _ = stream.read(&mut reply);
            return true;
        }
        if attempt + 1 < CONNECT_ATTEMPTS {
            thread::sleep(CONNECT_DELAY);
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "study-planner-instance-{}-{name}",
            std::process::id()
        ))
    }

    #[test]
    fn la_primera_instancia_gana_el_lock_y_la_segunda_avisa() {
        let dir = temp_dir("segunda");
        let calls = Arc::new(AtomicUsize::new(0));

        let first = acquire(&dir, {
            let calls = Arc::clone(&calls);
            move || {
                calls.fetch_add(1, Ordering::SeqCst);
            }
        })
        .expect("primera instancia");
        assert!(matches!(first, Acquired::Primary(_)));

        let second = acquire(&dir, || {}).expect("segunda instancia");
        assert!(matches!(second, Acquired::Secondary { notified: true }));

        // El aviso llega al hilo de la primaria.
        for _ in 0..100 {
            if calls.load(Ordering::SeqCst) > 0 {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        drop(first);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sin_socket_la_segunda_instancia_tampoco_arranca() {
        let dir = temp_dir("sin-socket");
        let first = acquire(&dir, || {}).expect("primera instancia");
        assert!(matches!(first, Acquired::Primary(_)));

        // Simula una primaria que todavía no escucha.
        std::fs::remove_file(dir.join(SOCKET_NAME)).expect("eliminar socket");

        let started = std::time::Instant::now();
        let second = acquire(&dir, || {}).expect("segunda instancia");
        assert!(matches!(second, Acquired::Secondary { notified: false }));
        assert!(started.elapsed() >= CONNECT_DELAY);

        drop(first);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

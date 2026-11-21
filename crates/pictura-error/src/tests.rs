use super::*;
use std::error::Error as _;
use std::io;

#[test]
fn operation_status_registry_is_stable() {
    let published = [
        (0, "operation.ok"),
        (1, "error.abi.version"),
        (2, "error.capability.denied"),
        (3, "error.argument.invalid"),
        (4, "operation.cancelled"),
        (5, "error.host"),
        (6, "error.panic"),
        (100, "error.codec"),
        (101, "error.io"),
        (102, "error.color"),
        (103, "error.gpu"),
        (104, "error.scratch"),
        (105, "error.script"),
        (107, "error.license"),
    ];
    assert_eq!(std::mem::size_of::<OpStatus>(), 4);
    assert_eq!(ErrorCode::ALL.len(), published.len());
    for (&code, &(number, key)) in ErrorCode::ALL.iter().zip(&published) {
        assert_eq!(code.status().raw(), number);
        assert_eq!(OpStatus::from_raw(number).code(), Some(code));
        assert_eq!(code.message_key(), key);
    }
}

#[test]
fn unknown_status_preserves_its_number() {
    for number in [106, u32::MAX] {
        let status = OpStatus::from_raw(number);
        assert!(status.code().is_none());
        assert!(!status.is_ok());
        assert_eq!(status.raw(), number);
    }
}

#[test]
fn guard_preserves_results_and_contains_panics() {
    struct Payload;
    impl Drop for Payload {
        fn drop(&mut self) {
            panic!("a panic payload destructor must not run");
        }
    }

    extern "C" fn boundary() -> u32 {
        guard(|| -> Result<(), Error> { std::panic::panic_any(Payload) })
            .unwrap_err()
            .code()
            .status()
            .raw()
    }

    assert_eq!(guard(|| Ok(42)).unwrap(), 42);
    let error = guard(|| -> Result<(), Error> {
        Err(Error::with_source(
            ErrorCode::Io,
            "could not read the file",
            io::Error::from(io::ErrorKind::NotFound),
        ))
    })
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::Io);
    assert_eq!(
        error
            .source()
            .unwrap()
            .downcast_ref::<io::Error>()
            .unwrap()
            .kind(),
        io::ErrorKind::NotFound
    );
    assert_eq!(boundary(), 6);
    let panic = guard(|| -> Result<(), Error> { panic!("private payload") }).unwrap_err();
    assert_eq!(panic.code(), ErrorCode::Panic);
    assert_eq!(panic.to_string(), "the operation stopped unexpectedly");
    assert_eq!(guard(|| Ok(7)).unwrap(), 7);
}

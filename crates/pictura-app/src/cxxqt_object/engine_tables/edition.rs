use pictura_core::Edition;

pub(crate) fn edition_is_extended() -> bool {
    Edition::from_env().is_extended()
}

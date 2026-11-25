//! Thread names the debug toolset groups CPU time by. The kernel keeps 15
//! characters of a name, so each group is a short prefix.

pub(crate) const OPEN: &str = "pictura-open";
pub(crate) const SAVE: &str = "pictura-save";
pub(crate) const FILTER: &str = "pictura-filter";
pub(crate) const DOCUMENT: &str = "pictura-doc";
pub(crate) const RAYON_PREFIX: &str = "pictura-rayon-";

pub(crate) fn rayon_thread_name(index: usize) -> String {
    format!("{RAYON_PREFIX}{index}")
}

/// Name the global rayon pool's workers. Fails quietly when something already
/// built the pool: the names are diagnostics, not behaviour.
pub(crate) fn init_rayon_names() {
    let _ = rayon::ThreadPoolBuilder::new()
        .thread_name(rayon_thread_name)
        .build_global();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_threads_carry_group_names() {
        assert_eq!(rayon_thread_name(3), "pictura-rayon-3");
        let pool = rayon::ThreadPoolBuilder::new()
            .thread_name(rayon_thread_name)
            .num_threads(1)
            .build()
            .unwrap();
        let name = pool.install(|| std::thread::current().name().map(str::to_owned));
        assert!(name.unwrap().starts_with(RAYON_PREFIX));
        let job = std::thread::Builder::new()
            .name(OPEN.into())
            .spawn(|| std::thread::current().name().map(str::to_owned))
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(job.as_deref(), Some(OPEN));
    }
}

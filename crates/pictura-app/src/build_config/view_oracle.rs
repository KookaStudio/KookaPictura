use super::{read, repo_root};

fn section<'a>(text: &'a str, heading: &str) -> &'a str {
    let marker = format!("[{heading}]");
    text.split_once(marker.as_str())
        .map_or("", |(_, body)| body)
        .split("\n[")
        .next()
        .unwrap_or_default()
}

fn boundary(lib: &str, render: &str, app: &str, oracle: &str, shared: &str) -> bool {
    let gate = "#[cfg(any(test, feature = \"test-oracle\"))]";
    let dev = section(app, "dev-dependencies");
    lib.contains(&format!("{gate}\nmod view_pyramid;"))
        && lib.contains(&format!(
            "{gate}\npub use view_pyramid::{{Planes, ViewPyramid}};"
        ))
        && lib.contains("pub use pyramid_level::{PyramidLevel, SMALLEST_SIDE, TILE};")
        && section(render, "features")
            .lines()
            .any(|line| line.trim() == "test-oracle = []")
        && !section(render, "features").contains("default =")
        && dev.contains(
            "pictura-render = { path = \"../pictura-render\", features = [\"test-oracle\"] }",
        )
        && app.matches("\"test-oracle\"").count() == 1
        && ["rebuild", "level_count", "level_size", "crop"]
            .iter()
            .all(|name| oracle.contains(&format!("pub fn {name}(")))
        && ["coarse", "coarse_level", "fits", "update", "patch_level"]
            .iter()
            .all(|name| !oracle.contains(&format!("pub fn {name}(")))
        && [
            "crate::region_level",
            "crate::tile_pyramid",
            "crate::level_count",
            "crate::level_size",
            "crate::premultiply_planar",
            "use crate::*",
        ]
        .iter()
        .all(|name| !oracle.contains(name))
        && !shared.contains("fn half")
        && !shared.contains("fn premul")
        && !shared.contains("fn average")
}

#[test]
fn view_oracle_is_test_only() {
    let root = repo_root();
    let lib = read(&root.join("crates/pictura-render/src/lib.rs"));
    let render = read(&root.join("crates/pictura-render/Cargo.toml"));
    let app = read(&root.join("crates/pictura-app/Cargo.toml"));
    let oracle = read(&root.join("crates/pictura-render/src/view_pyramid.rs"));
    let shared = read(&root.join("crates/pictura-render/src/pyramid_level.rs"));
    assert!(boundary(&lib, &render, &app, &oracle, &shared));

    let ungated = lib.replace("#[cfg(any(test, feature = \"test-oracle\"))]", "");
    assert!(!boundary(&ungated, &render, &app, &oracle, &shared));
    let coupled = format!("{oracle}\nuse crate::region_level::level_size;");
    assert!(!boundary(&lib, &render, &app, &coupled, &shared));
    // The dependency also carries the renderer's own `gpu` feature; the oracle
    // must stay out of it. The simulation is asserted to have changed the line,
    // so a future edit to the dependency cannot leave this case testing nothing.
    let dependency = "pictura-render = { path = \"../pictura-render\", features = [\"gpu\"] }";
    assert!(app.contains(dependency), "the app dependency shape moved");
    let enabled = app.replace(
        dependency,
        "pictura-render = { path = \"../pictura-render\", features = [\"gpu\", \"test-oracle\"] }",
    );
    assert_ne!(
        enabled, app,
        "the oracle-in-the-dependency case tested nothing"
    );
    assert!(!boundary(&lib, &render, &enabled, &oracle, &shared));
}

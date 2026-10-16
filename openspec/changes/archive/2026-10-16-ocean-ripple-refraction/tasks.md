# Tasks: ocean-ripple-refraction

## 1. Engine

- [x] 1.1 `ocean_ripple` displaces along the slope of seeded smoothstep value noise.

## 2. App

- [x] 2.1 Default Ripple Magnitude 9 in `filter_commands.cpp` and `filter_map.rs`.

## 3. Verification

- [x] 3.1 Reach test: mean shift under 1 px at (14, 2), over 2 px at (9, 9), and larger again at (2, 12); seed determinism and zero no-op kept.
- [x] 3.2 Oracle notes updated; `bash scripts/verify-full.sh`; `openspec validate --all --strict`.

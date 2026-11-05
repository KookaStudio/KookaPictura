Cold composites with large bevels and glows now prepare independent layers in
parallel and retain the passes before band replay. Combined with PR #1221's exact
SIMD Gaussian and parallel bevel work, resolved mask gates, bulk matte writes,
row replay and cache-local Gaussian tiles bring first/median cold CPU compositing
to 808–990 ms on the acceptance workstation (16 workers). Every output byte
matches the original scalar reference; one later sample was 1022 ms.

Validation: 2497 non-ignored regression checks pass across renderer/filter units
and integration/oracle suites; the production first/median acceptance passes in
three fresh processes. Clippy, formatting, test-plan, file-size and repository
guards pass. No new dependency or golden change. Peak process RSS reaches about
1.46 GiB across repeated samples, including retained allocations and reference
buffers. This branch builds on the exact kernel work merged in PR #1221.

Review corrections remove blocking feather-cache initialization during nested
Rayon work and require existing independent profiling references. Four added
regressions pass, as do all 1503 renderer unit checks and 32 renderer oracle
checks after correction. The unit binary runs with outer execution serialized
because its initial parallel run exposed an existing global cache-count race;
the regressions still force concurrent Rayon work internally. Post-review
production acceptance passes at 975 ms first and 856 ms median. An earlier
attempt measured 1095 ms first and 1399 ms median and failed the latency check;
all output bytes matched in both attempts. The cause of timing variation was
not established.

Reviewer checklist:

- [ ] Repeat the locally supplied workload check with equal worker count and
  kernel settings against an existing independently generated reference; confirm
  exact output, independent build overlap, cold/warm
  latency and memory trade-offs. This workload is not a committed CI fixture.

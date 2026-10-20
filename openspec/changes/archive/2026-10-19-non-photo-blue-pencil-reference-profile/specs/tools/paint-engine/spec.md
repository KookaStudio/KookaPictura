## MODIFIED Requirements

### Requirement: Pencil paper grain

`StrokeConfig::grain` SHALL modulate the painted alpha of every dab by a fixed,
deterministic paper texture keyed to layer coordinates. The texture SHALL
combine a per-pixel hash with bilinear value noise on a 2 px and a 3 px grid so
the grain forms 2–3 px clumps with fine per-pixel variation, weighted toward
the clumps to match the 2B Pencil reference. It SHALL NOT punch holes in the
dab core, and a grainy stroke SHALL render identically on every run. Grainy
strokes SHALL stay on the CPU path because the GPU dab shader does not model
grain.

#### Scenario: Grain breaks up the dab alpha

- **WHEN** a dab is painted with grain 75 and the same dab with grain 0
- **THEN** the grained dab has partial alpha, no zero-alpha pixels in its core,
  at least half the plain coverage, less total coverage, and renders
  identically twice

#### Scenario: Grain strength follows the pencil presets

- **WHEN** a dab is painted with grain 50 and another with grain 55
- **THEN** the grain-50 dab keeps about 75 % of a plain dab's coverage and the
  grain-55 dab keeps about 72 %

#### Scenario: Grain is clumped

- **WHEN** grain noise is sampled across a layer
- **THEN** a 2 px box average keeps most of the texture's spread and
  horizontally adjacent pixels correlate at 0.55–0.72, so the grain forms
  2–3 px clumps rather than single-pixel static

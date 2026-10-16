# imaging/parallel-filter-kernels Specification

## Purpose
Parallel filter kernels produce the same bytes as their serial version.

## Requirements

### Requirement: Parallel kernels keep their serial bytes

Gaussian Blur, Unsharp Mask, High Pass, Median, Motion Blur, Box Blur, Maximum,
Minimum and Custom SHALL run their passes in parallel by row (or by column strip
for Box Blur's vertical pass), with each sample summed or ranked in the same
order as the serial kernel, so the output is byte-identical to it.

#### Scenario: Digests match the serial kernels

- **WHEN** each of these kernels filters the fixed 97×61 test picture
- **THEN** the output's digest equals the digest recorded from its serial version

### Requirement: Radial Blur samples in tiles

Radial Blur SHALL compute its output over 64×64 tiles in parallel, finding each
sample point and its bilinear weights once for every plane. Spin SHALL rotate
the pixel's offset from the centre by each sample angle's precomputed cosine
and sine. The output SHALL keep the digests recorded from the serial kernel.

#### Scenario: Radial digests are unchanged

- **WHEN** Spin (amount 30, Good) and Zoom (amount 60, Best) filter the fixed 97×61 test picture
- **THEN** each digest equals the one recorded from the serial kernel

### Requirement: Layer filters write back in parallel

The layer-filter wrapper SHALL crop the layer and write the result back by row
in parallel. Without a selection mask or a transparency lock it SHALL copy the
filtered rows directly, which equals the blend at full coverage.

#### Scenario: An unmasked filter equals its kernel

- **WHEN** an unmasked filter is applied to an unlocked layer
- **THEN** each colour channel inside the region equals the kernel's output

### Requirement: An opaque layer skips the alpha pass

When a moving kernel is applied to an unlocked layer whose alpha in the region
is fully opaque, the wrapper SHALL skip rerunning the kernel over the alpha. Every
such kernel maps a flat opaque plane to itself.

#### Scenario: Every moving kernel maps opaque to opaque

- **WHEN** each kernel that `filter_preserves_opacity` rejects runs over a flat 255 grey plane
- **THEN** the plane comes back flat 255

# Spec Delta

## ADDED Requirements

### Requirement: Anti-aliased magnetic lasso commit
The Magnetic Lasso SHALL commit its fastened outline through the shared lasso
path with the tool-time Anti-alias setting. With Anti-alias on, the committed
polygon boundary SHALL hold fractional coverage so that boundary pixels MAY
have values strictly between 0 and 255. With it off, every coverage value SHALL
be 0 or 255. This SHALL NOT change Width, Contrast, Frequency, automatic
fastening, Delete, Escape, or the close-by-click behavior.

#### Scenario: Magnetic Lasso Anti-alias produces fractional coverage [mag_aa]
- **WHEN** a Magnetic Lasso outline closes with Anti-alias on and then again with it off
- **THEN** the anti-aliased mask contains boundary values strictly between 0 and 255, while the mask with Anti-alias off contains only 0 and 255

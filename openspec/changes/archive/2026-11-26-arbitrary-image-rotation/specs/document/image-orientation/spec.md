## MODIFIED Requirements

### Requirement: Arbitrary image-rotation entry point

The system SHALL expose `Image > Image Rotation > Arbitrary` with a two-decimal signed angle from −359.99° through 359.99° and separate °CW and °CCW controls. CW SHALL use the entered value as the clockwise rotation and CCW SHALL negate it. The dialog SHALL open at 0° CW unless the active document has a ruler measurement, in which case the dialog SHALL prefill the magnitude and direction needed to level the line to the nearer axis. If accepted with a nonzero valid angle, the system SHALL apply the rotation about the document centre through the cancellable document command, recomposite, and record one history state. Cancel, empty or invalid input, zero, and a failed operation SHALL leave the document and history unchanged.

#### Scenario: Arbitrary command rotates and records [lio_arbitrary_command]

- **WHEN** the user accepts a nonzero valid angle with a direction
- **THEN** the document rotates about its centre in the requested direction, grows to the bounding box, and records exactly one undoable state

#### Scenario: Arbitrary command refuses bad input [lio_arbitrary_refusal]

- **WHEN** the user cancels, enters zero, or attempts to apply invalid or empty text
- **THEN** the document and history are unchanged

#### Scenario: Ruler measurement prefills correction [lio_ruler_rotation_prefill]

- **WHEN** the user opens Arbitrary with a measured line in the active document
- **THEN** the direction and angle show the correction to the nearer axis, to 0.01°, independently of endpoint order
- **AND** clearing the line restores the 0° CW default

## ADDED Requirements

### Requirement: Arbitrary image-rotation interpolation

General preferences SHALL persist Image Interpolation with Nearest Neighbor, Bilinear, Bicubic, Bicubic Smoother, Bicubic Sharper, and Bicubic Automatic. The default SHALL be Bicubic. A non-right-angle interactive whole-document rotation SHALL apply the selected kernel to the existing rotated raster planes. Bicubic Automatic SHALL use Bicubic at unit scale. Exact centre right-angle rotations SHALL map raster samples exactly and independently of the preference, including when vector-mask vertices require the geometry path. Fixed-angle commands SHALL remain unchanged. New Actions recordings SHALL carry the interpolation ID and signed angle through a Pictura .atn save and reload. Imported or direct angle-only calls SHALL retain bilinear behavior at non-right angles; Adobe playback of the Pictura sidecar is not guaranteed.

#### Scenario: Interactive rotation uses the saved preference [lio_rotation_interpolation]

- **WHEN** the user saves Nearest Neighbor and rotates a contrast pattern through a non-right angle
- **THEN** the choice survives reloading preferences and rotated samples match Nearest Neighbor

#### Scenario: Tiled interpolation preserves geometry and handles all kernels [lio_rotation_kernels]

- **WHEN** arbitrary rotation samples a multi-tile contrast pattern through each supported method
- **THEN** the output uses the requested kernel, keeps identical bounds and zero fill for all methods, and Automatic matches Bicubic
- **AND** samples at source edges and tile seams match dense sampling with edge-clamped taps, including masks and native-depth planes

#### Scenario: Recorded rotation uses its interpolation [lio_rotation_action]

- **WHEN** a recorded command runs after the preference changes
- **THEN** it uses its recorded kernel
- **AND** an angle-only command uses Bilinear

#### Scenario: Right-angle rotation with a vector mask is exact [lio_rotation_vector_right_angle]

- **WHEN** a document with raster content and a vector mask is rotated by an exact centre quarter turn using each preference
- **THEN** raster samples match the exact index remap and the vector-mask vertices follow the rotation

#### Scenario: Rotation action survives an ATN round trip [lio_rotation_atn]

- **WHEN** a newly recorded non-right-angle rotation is saved to and reloaded from a Pictura .atn file
- **THEN** its signed angle and interpolation ID survive and playback uses the recorded kernel
- **AND** an imported Adobe angle-only rotation uses Bilinear

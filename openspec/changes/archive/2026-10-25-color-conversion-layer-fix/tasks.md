# Tasks

## 1. Convert to Profile native stores

- [x] 1.1 Skip native-store conversion when `doc.source_mode` is recorded; convert only the 8-bit working channels.
- [x] 1.2 Add a 16-bit CMYK layered regression test asserting the layer and composite stay in sync through Convert.

## 2. Color panel entry point

- [x] 2.1 Make the Color panel foreground and background swatches open the Color Picker and apply the accepted colour.
- [x] 2.2 Add a Qt Test case that clicks the foreground swatch, accepts a colour, and checks the shared state.

## 3. Finish

- [x] 3.1 Archive this change and sync the capability specs.

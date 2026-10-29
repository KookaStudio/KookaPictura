# Test Plan

One row per scenario in the delta spec. Every row is automated.

## tools/healing-brushes

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| A blemish on a flat field disappears | Run Proximity Match over an 8×8 white blemish on a flat field, fully covered. | Every pixel under the mask equals the field. | `pictura-paint::a_blemish_on_a_flat_field_disappears` |
| Content-Aware carries an edge | Run Content-Aware over a hole that straddles a dark/light step. | Left of the step stays dark and right stays light. | `pictura-paint::content_aware_carries_an_edge_across_the_hole` |
| A brush the size of a dot removes the dot | Draw an anti-aliased black dot of radius 4 on a vertical sky gradient. Place one hard 8 px `HealStroke` dab on its centre, and a 10 px dab one pixel off-centre, and commit in Proximity Match and in Create Texture. | Every pixel within 8 px of the dot is within tolerance of the clean sky. | `pictura-paint::a_click_with_a_dot_sized_brush_removes_the_dot`, `pictura-paint::create_texture_click_removes_the_dot` |
| A heal next to an edge leaves no trace | Draw a dark field left of x = 28 and a sky gradient right of it. Put a black dot on the sky beside the edge and heal it with a brush that overlaps the edge. | The edge step keeps at least 80% of its height on every healed row (the old Laplace fill kept under 10%), and every pixel is within 12 levels of the clean image. The residual is the smooth lighting correction from a source lit differently. | `pictura-paint::a_heal_next_to_an_edge_leaves_no_trace` |
| Another defect is not used as the source | Draw two identical black dots 14 px apart in a row on a sky gradient and heal one. | The healed dot matches the clean sky, the other dot is untouched, and no dark pixel is left in the healed area. | `pictura-paint::another_dot_is_not_used_as_the_source` |
| Pixels beyond the margin are untouched | Heal a fully covered 8×8 square with odd pixels placed just past the 3 px margin, then run the same heal through `heal_layer`. | The odd pixels keep their values, and the returned rectangle is the region grown by 3. | `pictura-paint::healing_leaves_pixels_beyond_the_margin_alone`, `pictura-paint::heal_layer_writes_the_pixels_and_returns_a_document_rect` |
| Healing a blemish | Click the Spot Healing Brush once on a black dot in a document. | One "Spot Healing Brush" state is recorded and the dot's centre is near the surrounding sky. | `tst_retouch_tools::spotHealingClickRemovesADot` |
| A click with a dot-sized brush removes the dot | Open a sky-gradient image with an anti-aliased black dot of radius 4. Set the brush to 8 px and Proximity Match, then click once on the dot. | One "Spot Healing Brush" state is recorded and every pixel within 7 px of the dot is within tolerance of the clean sky. | `tst_retouch_tools::spotHealingClickRemovesADot` |

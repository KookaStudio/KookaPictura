# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `rs_refuses` | New 40×20 RGB/8 document; call `image_size_apply` with `bicubic` to 30000×30000 | Returns false; width, height and history count are unchanged | `tst_image_ops::imageSizeRefusesAnOverBudgetTarget` |
| `rs_bridge_refuses` | New 40×20 document; call `PictureView::resize_image` with `bicubic` to 30000×30000 | Returns false; the document and its history are unchanged | `tst_image_ops::resizeImageRefusesAnOverBudgetTarget` |
| `rs_allows` | New 40×20 document; accept Image Size at 20×10 with `bicubic` | Returns true; the document is 20×10 and the current history label is "Image Size" | `tst_image_ops::imageSizeResamplesAndSetsResolution` |
| `rs_resolution_only` | After the over-budget refusal, call `image_size_apply` with an empty resample kind at 40×20 and 300 ppi | Returns true; the pixel size stays 40×20 and the resolution reads 300 ppi | `tst_image_ops::imageSizeRefusesAnOverBudgetTarget` |
| `rs_message` | Ask the budget about 30000×30000 and read the warning text | `imageSizeFitsBudget` is false; the text names 30000 and the memory shortfall | `tst_image_ops::imageSizeRefusesAnOverBudgetTarget` |
| `rs_warning_shown` | Dispatch `image.image.size`, answer the dialog with 30000×30000, then answer the warning | One warning is shown; the document and its history are unchanged | `tst_image_ops::imageSizeShowsTheBudgetWarning` |
| `rs_shared_budget` | Run the shared predicate against synthetic budgets for dimension, per-allocation and per-document limits, and the resize bound against low, high and unknown available memory | Sides are judged first, byte limits are enforced, an inside-budget document is accepted, and the resize bound drops the default minimum allocation floor | `pictura-app::budget_judges_sides_then_bytes`, `pictura-app::budget_honours_the_document_limit`, `pictura-app::a_resize_document_limit_drops_the_allocation_floor` |

# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `pdf_photoshop_layers` | Write a document with a type layer as PSD, repackage it in Photoshop's PDF layout (predictor-2 composite), rebuild and read it | Same layer names; type layer with its text; layer pixels and composite equal | `pictura-render::photoshop_pdf_reopens_with_its_layers` |
| `pdf_photoshop_absent` | Rebuild from a PDF without Photoshop data; open a generic PDF in the window | `None`; Import PDF dialog shown | `pictura-render::a_pdf_without_photoshop_data_is_not_rebuilt`, `tst_import_pdf::genericPdfShowsDialog` |
| A real Photoshop PDF | Save a PSD with a type layer from Photoshop as Photoshop PDF with Preserve Photoshop Editing Capabilities; open it in Pictura | Background and an editable type layer, no Import PDF dialog | manual: needs Photoshop to produce the file (verified on the owner's `pdf_from_fotoshop.pdf`); Adobe-generated files are not committed as fixtures |

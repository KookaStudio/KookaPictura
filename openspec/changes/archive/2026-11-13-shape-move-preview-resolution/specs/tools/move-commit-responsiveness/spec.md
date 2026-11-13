## ADDED Requirements

### Requirement: A large shape move defers its reach composite

When a shape layer's before-and-after reach union reaches 16 MP and the document
also reaches 16 MP, releasing a Move drag SHALL translate the shape and return
without compositing, and a worker SHALL composite the reach rectangles. The
landed composite MUST equal a full composite of the moved document, and landing
SHALL capture one history state labelled with the commit's label. A shape reach
below 16 MP, or a document below 16 MP, SHALL commit synchronously.

#### Scenario: A large shape release lands off the GUI thread [mcr_shape_deferred]

- **WHEN** a shape whose before-and-after reach union is at least 16 MP on a
  4200² document is released after a Move drag
- **THEN** the commit returns with the shape moved and the composite pending, and
  when the job lands the composite equals a full composite and history gained one
  `Move Layer` state

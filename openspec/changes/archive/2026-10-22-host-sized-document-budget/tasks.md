# Tasks

- [x] Rust tests: `the_document_limit_follows_available_memory`, `meminfo_available_is_parsed_in_bytes`, `the_host_reports_available_memory`, `a_document_over_its_limit_is_refused_and_the_default_keeps_a_plane`; pin the document limit in the tests that measure one
- [x] `ImageBudget::max_document_bytes` with a host-derived default; the PSD decode budget and the PDF inflate budget use it

# agentic-control delta

## ADDED Requirements

### Requirement: WebP export options are validated

The `export_webp` method SHALL answer `invalid_param`, without writing anything,
when `quality` is present and is not an integer from 0 through 100, or when
`lossless` is present and is not a boolean.

#### Scenario: Invalid WebP options are refused [ce_export_webp_options]

- **WHEN** a client calls `export_webp` with `quality` 101, `quality` -1, or a
  non-boolean `lossless`
- **THEN** the response is `invalid_param` and the destination is not created
